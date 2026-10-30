use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
};

use rusqlite::{Connection, OptionalExtension, params};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use protocol::{NodeId, NodeKind};

use super::model::{Base, Mapping, RemoteNode};

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS mappings (
        remote TEXT PRIMARY KEY,  -- '' for Home
        local  TEXT               -- NULL: do not sync
    );
    CREATE TABLE IF NOT EXISTS remote (
        id         TEXT PRIMARY KEY,
        parent     TEXT,
        kind       TEXT NOT NULL,
        name       TEXT NOT NULL,
        size       INTEGER NOT NULL,
        sha256     TEXT,
        updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS base (
        node   TEXT PRIMARY KEY,
        path   TEXT NOT NULL,
        kind   TEXT NOT NULL,
        sha256 TEXT,
        size   INTEGER NOT NULL,
        mtime  INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
";
const HOME: &str = "";
const CURSOR: &str = "cursor";

/// What the sync remembers between runs, for one account: the mappings, the last known
/// remote tree with its change cursor, and the bases.
pub struct Store {
    db: Mutex<Connection>,
}

type Result<T> = rusqlite::Result<T>;

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::with(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        Self::with(Connection::open_in_memory()?)
    }

    fn with(db: Connection) -> Result<Self> {
        db.execute_batch("PRAGMA journal_mode = WAL;")?;
        db.execute_batch(SCHEMA)?;
        Ok(Self { db: Mutex::new(db) })
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn mappings(&self) -> Result<Vec<Mapping>> {
        let db = self.db();
        let mut statement = db.prepare("SELECT remote, local FROM mappings")?;
        let rows = statement.query_map([], |row| {
            let remote: String = row.get(0)?;
            let local: Option<String> = row.get(1)?;
            Ok(Mapping {
                remote: (remote != HOME).then(|| node_id(&remote)).flatten(),
                local: local.map(PathBuf::from),
            })
        })?;
        rows.collect()
    }

    /// Sets where `mapping.remote` syncs to, replacing an earlier mapping of it.
    pub fn set_mapping(&self, mapping: &Mapping) -> Result<()> {
        self.db().execute(
            "INSERT INTO mappings (remote, local) VALUES (?1, ?2) \
             ON CONFLICT (remote) DO UPDATE SET local = excluded.local",
            params![
                remote_key(mapping.remote),
                mapping.local.as_deref().map(path_text)
            ],
        )?;
        Ok(())
    }

    pub fn remove_mapping(&self, remote: Option<NodeId>) -> Result<()> {
        self.db().execute(
            "DELETE FROM mappings WHERE remote = ?1",
            params![remote_key(remote)],
        )?;
        Ok(())
    }

    pub fn cursor(&self) -> Result<Option<i64>> {
        self.db()
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![CURSOR],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map(|value| value.and_then(|value| value.parse().ok()))
    }

    pub fn set_cursor(&self, cursor: Option<i64>) -> Result<()> {
        match cursor {
            Some(cursor) => self.db().execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2) \
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![CURSOR, cursor.to_string()],
            ),
            None => self
                .db()
                .execute("DELETE FROM meta WHERE key = ?1", params![CURSOR]),
        }?;
        Ok(())
    }

    pub fn remote_nodes(&self) -> Result<Vec<RemoteNode>> {
        let db = self.db();
        let mut statement =
            db.prepare("SELECT id, parent, kind, name, size, sha256, updated_at FROM remote")?;
        let rows = statement.query_map([], |row| {
            let id: String = row.get(0)?;
            let parent: Option<String> = row.get(1)?;
            let kind: String = row.get(2)?;
            let updated_at: String = row.get(6)?;
            Ok((
                id,
                parent,
                kind,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                updated_at,
            ))
        })?;
        let mut nodes = Vec::new();
        for row in rows {
            let (id, parent, kind, name, size, sha256, updated_at) = row?;
            let (Some(id), Some(kind)) = (node_id(&id), node_kind(&kind)) else {
                continue;
            };
            nodes.push(RemoteNode {
                id,
                parent: parent.as_deref().and_then(node_id),
                kind,
                name,
                size,
                sha256,
                updated_at: OffsetDateTime::parse(&updated_at, &Rfc3339)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            });
        }
        Ok(nodes)
    }

    /// Records changes to the remote tree: `upserted` nodes and `removed` ids, or with
    /// `replace` the whole tree.
    pub fn update_remote(
        &self,
        replace: bool,
        upserted: &[RemoteNode],
        removed: &[NodeId],
    ) -> Result<()> {
        let mut db = self.db();
        let tx = db.transaction()?;
        if replace {
            tx.execute("DELETE FROM remote", [])?;
        }
        {
            let mut upsert = tx.prepare(
                "INSERT OR REPLACE INTO remote (id, parent, kind, name, size, sha256, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for node in upserted {
                upsert.execute(params![
                    node.id.to_string(),
                    node.parent.map(|id| id.to_string()),
                    kind_text(node.kind),
                    node.name,
                    node.size,
                    node.sha256,
                    node.updated_at.format(&Rfc3339).unwrap_or_default(),
                ])?;
            }
            let mut delete = tx.prepare("DELETE FROM remote WHERE id = ?1")?;
            for id in removed {
                delete.execute(params![id.to_string()])?;
            }
        }
        tx.commit()
    }

    pub fn bases(&self) -> Result<Vec<Base>> {
        let db = self.db();
        let mut statement = db.prepare("SELECT node, path, kind, sha256, size, mtime FROM base")?;
        let rows = statement.query_map([], |row| {
            let node: String = row.get(0)?;
            let path: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let size: i64 = row.get(4)?;
            Ok((node, path, kind, row.get(3)?, size, row.get(5)?))
        })?;
        let mut bases = Vec::new();
        for row in rows {
            let (node, path, kind, sha256, size, mtime) = row?;
            let (Some(node), Some(kind)) = (node_id(&node), node_kind(&kind)) else {
                continue;
            };
            bases.push(Base {
                node,
                path: PathBuf::from(path),
                kind,
                sha256,
                size: u64::try_from(size).unwrap_or(0),
                mtime,
            });
        }
        Ok(bases)
    }

    pub fn put_base(&self, base: &Base) -> Result<()> {
        self.db().execute(
            "INSERT OR REPLACE INTO base (node, path, kind, sha256, size, mtime) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                base.node.to_string(),
                path_text(&base.path),
                kind_text(base.kind),
                base.sha256,
                i64::try_from(base.size).unwrap_or(i64::MAX),
                base.mtime,
            ],
        )?;
        Ok(())
    }

    /// Forgets the base of `node`; with `at`, only while it is still recorded at that path.
    pub fn forget_base(&self, node: NodeId, at: Option<&Path>) -> Result<()> {
        match at {
            Some(path) => self.db().execute(
                "DELETE FROM base WHERE node = ?1 AND path = ?2",
                params![node.to_string(), path_text(path)],
            ),
            None => self.db().execute(
                "DELETE FROM base WHERE node = ?1",
                params![node.to_string()],
            ),
        }?;
        Ok(())
    }

    /// Moves the bases at and under `from` to `to`.
    pub fn rebase(&self, from: &Path, to: &Path) -> Result<()> {
        let moved: Vec<Base> = self
            .bases()?
            .into_iter()
            .filter_map(|base| {
                let path = super::model::rebased(&base.path, from, to)?;
                Some(Base { path, ..base })
            })
            .collect();
        let mut db = self.db();
        let tx = db.transaction()?;
        for base in &moved {
            tx.execute(
                "UPDATE base SET path = ?2 WHERE node = ?1",
                params![base.node.to_string(), path_text(&base.path)],
            )?;
        }
        tx.commit()
    }
}

fn remote_key(remote: Option<NodeId>) -> String {
    remote.map_or_else(|| HOME.to_owned(), |id| id.to_string())
}

fn node_id(text: &str) -> Option<NodeId> {
    text.parse().ok().map(NodeId)
}

fn node_kind(text: &str) -> Option<NodeKind> {
    match text {
        "file" => Some(NodeKind::File),
        "folder" => Some(NodeKind::Folder),
        _ => None,
    }
}

fn kind_text(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "file",
        NodeKind::Folder => "folder",
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::layout::tests::{id, node};

    fn base(n: u128, path: &str) -> Base {
        Base {
            node: id(n),
            path: PathBuf::from(path),
            kind: NodeKind::File,
            sha256: Some("x".to_owned()),
            size: 3,
            mtime: 42,
        }
    }

    #[test]
    fn mappings_round_trip_with_home_and_do_not_sync() {
        let store = Store::in_memory().expect("store");
        let home = Mapping {
            remote: None,
            local: Some(PathBuf::from("/sync")),
        };
        let off = Mapping {
            remote: Some(id(1)),
            local: None,
        };
        store.set_mapping(&home).expect("home");
        store.set_mapping(&off).expect("off");
        let mut mappings = store.mappings().expect("mappings");
        mappings.sort_by_key(|m| m.remote.is_some());
        assert_eq!(mappings, [home, off]);
        store.remove_mapping(None).expect("remove");
        assert_eq!(store.mappings().expect("mappings").len(), 1);
    }

    #[test]
    fn remote_nodes_are_upserted_removed_and_replaced() {
        let store = Store::in_memory().expect("store");
        let a = node(1, None, NodeKind::File, "a.txt");
        let b = node(2, None, NodeKind::Folder, "b");
        store
            .update_remote(false, &[a.clone(), b.clone()], &[])
            .expect("insert");
        store.update_remote(false, &[], &[a.id]).expect("remove");
        assert_eq!(store.remote_nodes().expect("nodes"), [b]);
        store
            .update_remote(true, std::slice::from_ref(&a), &[])
            .expect("replace");
        assert_eq!(store.remote_nodes().expect("nodes"), [a]);
    }

    #[test]
    fn bases_are_rebased_and_forgotten_only_at_their_path() {
        let store = Store::in_memory().expect("store");
        store.put_base(&base(1, "/s/dir/a.txt")).expect("a");
        store.put_base(&base(2, "/s/other.txt")).expect("b");
        store
            .rebase(Path::new("/s/dir"), Path::new("/s/new"))
            .expect("rebase");
        store
            .forget_base(id(1), Some(Path::new("/s/dir/a.txt")))
            .expect("stale forget");
        let mut bases = store.bases().expect("bases");
        bases.sort_by_key(|base| base.path.clone());
        assert_eq!(bases[0].path, Path::new("/s/new/a.txt"));
        assert_eq!(bases.len(), 2);
        store.forget_base(id(2), None).expect("forget");
        assert_eq!(store.bases().expect("bases").len(), 1);
    }

    #[test]
    fn the_cursor_is_kept_and_cleared() {
        let store = Store::in_memory().expect("store");
        assert_eq!(store.cursor().expect("none"), None);
        store.set_cursor(Some(7)).expect("set");
        assert_eq!(store.cursor().expect("some"), Some(7));
        store.set_cursor(None).expect("clear");
        assert_eq!(store.cursor().expect("none"), None);
    }
}
