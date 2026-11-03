use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Db, DbError, UserId};

/// A device as its client describes it on sign-in.
#[derive(Clone, Copy, Debug)]
pub struct NewDevice<'a> {
    pub id: Uuid,
    pub name: &'a str,
    pub platform: &'a str,
}

/// A device of a user.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub platform: String,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
    /// Whether it has a session that has not ended.
    pub signed_in: bool,
    /// How many uploads it has left unfinished.
    pub uploads: i64,
}

impl Db {
    /// Records a device of `user`, or marks a known one as seen. A known device keeps its
    /// name, which the user may have changed.
    pub async fn register_device(
        &self,
        user: UserId,
        device: NewDevice<'_>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO devices (user_id, id, name, platform) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (user_id, id) DO UPDATE \
             SET platform = EXCLUDED.platform, last_seen_at = now()",
        )
        .bind(user)
        .bind(device.id)
        .bind(device.name)
        .bind(device.platform)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// The devices of `user`, the most recently seen first.
    pub async fn devices(&self, user: UserId) -> Result<Vec<Device>, DbError> {
        let devices = sqlx::query_as::<_, Device>(
            "SELECT d.id, d.name, d.platform, d.created_at, d.last_seen_at, \
             EXISTS (SELECT 1 FROM sessions s WHERE s.user_id = d.user_id \
                     AND s.device_id = d.id AND s.expires_at > now()) AS signed_in, \
             (SELECT count(*) FROM uploads u WHERE u.owner_id = d.user_id \
              AND u.device_id = d.id) AS uploads \
             FROM devices d WHERE d.user_id = $1 ORDER BY d.last_seen_at DESC, d.id",
        )
        .bind(user)
        .fetch_all(self.pool())
        .await?;
        Ok(devices)
    }

    /// Renames a device of `user`.
    pub async fn rename_device(&self, user: UserId, id: Uuid, name: &str) -> Result<(), DbError> {
        let renamed = sqlx::query("UPDATE devices SET name = $3 WHERE user_id = $1 AND id = $2")
            .bind(user)
            .bind(id)
            .bind(name)
            .execute(self.pool())
            .await?;
        if renamed.rows_affected() == 0 {
            Err(DbError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Marks a device of `user` as seen now.
    pub async fn touch_device(&self, user: UserId, id: Uuid) -> Result<(), DbError> {
        sqlx::query("UPDATE devices SET last_seen_at = now() WHERE user_id = $1 AND id = $2")
            .bind(user)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    /// Forgets devices that have no live session and were last seen before `seen_before`,
    /// returning how many were removed.
    pub async fn prune_devices(&self, seen_before: OffsetDateTime) -> Result<u64, DbError> {
        let pruned = sqlx::query(
            "DELETE FROM devices d WHERE d.last_seen_at < $1 \
             AND NOT EXISTS (SELECT 1 FROM sessions s WHERE s.user_id = d.user_id \
                             AND s.device_id = d.id AND s.expires_at > now())",
        )
        .bind(seen_before)
        .execute(self.pool())
        .await?;
        Ok(pruned.rows_affected())
    }

    /// Ends every session of a device of `user`. The device stays in the list until
    /// [`Db::prune_devices`] forgets it.
    pub async fn sign_out_device(&self, user: UserId, id: Uuid) -> Result<(), DbError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM devices WHERE user_id = $1 AND id = $2")
            .bind(user)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND device_id = $2")
            .bind(user)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: &str = "Laptop";

    fn device(id: Uuid) -> NewDevice<'static> {
        NewDevice {
            id,
            name: LAPTOP,
            platform: "desktop-linux",
        }
    }

    async fn session(db: &Db, user: UserId, device: Option<Uuid>) -> Uuid {
        let id = Uuid::now_v7();
        let expires = OffsetDateTime::now_utc() + time::Duration::days(1);
        db.start_session(id, user, device, id.as_bytes(), expires)
            .await
            .expect("session");
        id
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn signing_in_again_keeps_the_name_the_user_chose(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        db.rename_device(alice, id, "Work").await.expect("rename");

        db.register_device(alice, device(id)).await.expect("again");

        let devices = db.devices(alice).await.expect("devices");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Work");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn users_of_one_install_each_get_their_own_device(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let bob = db.create_user("bob", "x").await.expect("bob").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id)).await.expect("alice");
        db.register_device(bob, device(id)).await.expect("bob");

        assert!(matches!(
            db.rename_device(bob, Uuid::now_v7(), "x").await,
            Err(DbError::NotFound)
        ));
        db.rename_device(bob, id, "Bob's").await.expect("rename");
        assert_eq!(db.devices(alice).await.expect("alice")[0].name, LAPTOP);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn signing_out_a_device_ends_only_its_sessions(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let (laptop, phone) = (Uuid::now_v7(), Uuid::now_v7());
        db.register_device(alice, device(laptop))
            .await
            .expect("laptop");
        db.register_device(alice, device(phone))
            .await
            .expect("phone");
        let on_laptop = session(&db, alice, Some(laptop)).await;
        let on_phone = session(&db, alice, Some(phone)).await;
        let unnamed = session(&db, alice, None).await;

        db.sign_out_device(alice, laptop).await.expect("sign out");

        assert_eq!(
            db.live_session(on_laptop, alice).await.expect("laptop"),
            None
        );
        assert!(
            db.live_session(on_phone, alice)
                .await
                .expect("phone")
                .is_some()
        );
        assert!(
            db.live_session(unnamed, alice)
                .await
                .expect("unnamed")
                .is_some()
        );
        let signed_in = db
            .devices(alice)
            .await
            .expect("devices")
            .into_iter()
            .map(|device| (device.id, device.signed_in))
            .collect::<Vec<_>>();
        assert_eq!(signed_in.len(), 2);
        assert!(signed_in.contains(&(laptop, false)));
        assert!(signed_in.contains(&(phone, true)));
    }

    async fn last_seen(db: &Db, user: UserId, id: Uuid, days_ago: i64) {
        sqlx::query(
            "UPDATE devices SET last_seen_at = now() - make_interval(days => $3) \
             WHERE user_id = $1 AND id = $2",
        )
        .bind(user)
        .bind(id)
        .bind(i32::try_from(days_ago).expect("days"))
        .execute(db.pool())
        .await
        .expect("last seen");
    }

    fn days_ago(days: i64) -> OffsetDateTime {
        OffsetDateTime::now_utc() - time::Duration::days(days)
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_device_signed_out_for_long_is_forgotten(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        last_seen(&db, alice, id, 31).await;

        assert_eq!(db.prune_devices(days_ago(30)).await.expect("prune"), 1);
        assert!(db.devices(alice).await.expect("devices").is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_device_signed_out_recently_is_kept(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        last_seen(&db, alice, id, 2).await;

        assert_eq!(db.prune_devices(days_ago(30)).await.expect("prune"), 0);
        assert_eq!(db.devices(alice).await.expect("devices").len(), 1);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_signed_in_device_is_kept(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        session(&db, alice, Some(id)).await;
        last_seen(&db, alice, id, 31).await;

        assert_eq!(db.prune_devices(days_ago(30)).await.expect("prune"), 0);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn touching_a_device_marks_it_seen(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        last_seen(&db, alice, id, 2).await;

        db.touch_device(alice, id).await.expect("touch");

        let seen = db.devices(alice).await.expect("devices")[0].last_seen_at;
        assert!(seen > days_ago(1));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_session_without_a_device_takes_the_one_attached(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        let unnamed = session(&db, alice, None).await;

        db.attach_device(unnamed, alice, id).await.expect("attach");

        let live = db.live_session(unnamed, alice).await.expect("live");
        assert_eq!(live.map(|live| live.device_id), Some(Some(id)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn a_session_keeps_the_device_it_has(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let (laptop, other) = (Uuid::now_v7(), Uuid::now_v7());
        db.register_device(alice, device(laptop))
            .await
            .expect("laptop");
        db.register_device(alice, device(other))
            .await
            .expect("other");
        let on_laptop = session(&db, alice, Some(laptop)).await;

        db.attach_device(on_laptop, alice, other)
            .await
            .expect("attach");

        let live = db.live_session(on_laptop, alice).await.expect("live");
        assert_eq!(live.map(|live| live.device_id), Some(Some(laptop)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn another_users_device_cannot_be_signed_out(pool: sqlx::PgPool) {
        let db = Db::from_pool(pool);
        let alice = db.create_user("alice", "x").await.expect("alice").id;
        let bob = db.create_user("bob", "x").await.expect("bob").id;
        let id = Uuid::now_v7();
        db.register_device(alice, device(id))
            .await
            .expect("register");
        let on_laptop = session(&db, alice, Some(id)).await;

        assert!(matches!(
            db.sign_out_device(bob, id).await,
            Err(DbError::NotFound)
        ));
        assert!(
            db.live_session(on_laptop, alice)
                .await
                .expect("live")
                .is_some()
        );
    }
}
