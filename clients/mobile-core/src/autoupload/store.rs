//! What auto-upload remembers between runs, for one user: settings, sources, what was sent, the
//! server folders made for a source, and the uploads under way.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

use super::plan::Record;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY,
        value INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS sources (
        id                 TEXT PRIMARY KEY,
        kind               TEXT NOT NULL,
        handle             TEXT NOT NULL,   -- the platform's way back to it, such as a tree URI
        label              TEXT NOT NULL,
        target             TEXT,            -- server folder; NULL for Home
        target_label       TEXT NOT NULL,
        leave_out_existing INTEGER NOT NULL, -- until the first pass has seen what is there
        problem            TEXT,            -- why the source is paused
        last_synced_at     INTEGER
    );
    CREATE TABLE IF NOT EXISTS records (
        source      TEXT NOT NULL,
        key         TEXT NOT NULL,
        size        INTEGER NOT NULL,
        mtime       INTEGER NOT NULL,
        remote_name TEXT,                   -- NULL: left out, never sent
        remote_dir  TEXT NOT NULL,
        PRIMARY KEY (source, key)
    );
    CREATE TABLE IF NOT EXISTS folders (
        source TEXT NOT NULL,
        dir    TEXT NOT NULL,               -- path below the target, '/' separated
        node   TEXT NOT NULL,
        PRIMARY KEY (source, dir)
    );
    CREATE TABLE IF NOT EXISTS uploads (
        source TEXT NOT NULL,
        key    TEXT NOT NULL,
        id     TEXT NOT NULL,               -- the server's upload session
        parent TEXT,
        name   TEXT NOT NULL,
        size   INTEGER NOT NULL,
        mtime  INTEGER NOT NULL,
        PRIMARY KEY (source, key)
    );
";
const WIFI_ONLY: &str = "wifi_only";
const BACKGROUND: &str = "background";

pub(super) type Result<T> = rusqlite::Result<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceRow {
    pub id: String,
    pub kind: String,
    pub handle: String,
    pub label: String,
    pub target: Option<String>,
    pub target_label: String,
    pub leave_out_existing: bool,
    pub problem: Option<String>,
    pub last_synced_at: Option<i64>,
}

/// An upload under way, saved so that it continues later. The bytes the server holds are only
/// worth keeping while the file and its destination are still the ones described here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SavedUpload {
    pub source: String,
    pub key: String,
    pub id: Uuid,
    pub parent: Option<String>,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
}

pub(super) struct Store {
    db: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA journal_mode = WAL;")?;
        db.execute_batch(SCHEMA)?;
        Ok(Self { db: Mutex::new(db) })
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn flag(&self, key: &str, default: bool) -> Result<bool> {
        let value: Option<i64> = self
            .db()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(value.map_or(default, |value| value != 0))
    }

    fn set_flag(&self, key: &str, value: bool) -> Result<()> {
        self.db().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, i64::from(value)],
        )?;
        Ok(())
    }

    /// Whether to sync only on Wi-Fi: yes, until the user says otherwise.
    pub fn wifi_only(&self) -> Result<bool> {
        self.flag(WIFI_ONLY, true)
    }

    /// Whether to sync in the background: no, until the user says otherwise.
    pub fn background(&self) -> Result<bool> {
        self.flag(BACKGROUND, false)
    }

    pub fn set_settings(&self, wifi_only: bool, background: bool) -> Result<()> {
        self.set_flag(WIFI_ONLY, wifi_only)?;
        self.set_flag(BACKGROUND, background)
    }

    pub fn add_source(&self, source: &SourceRow) -> Result<()> {
        self.db().execute(
            "INSERT INTO sources (id, kind, handle, label, target, target_label, \
             leave_out_existing, problem, last_synced_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                source.id,
                source.kind,
                source.handle,
                source.label,
                source.target,
                source.target_label,
                source.leave_out_existing,
                source.problem,
                source.last_synced_at,
            ],
        )?;
        Ok(())
    }

    pub fn sources(&self) -> Result<Vec<SourceRow>> {
        let db = self.db();
        let mut statement = db.prepare(
            "SELECT id, kind, handle, label, target, target_label, leave_out_existing, problem, \
             last_synced_at FROM sources ORDER BY label, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SourceRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                handle: row.get(2)?,
                label: row.get(3)?,
                target: row.get(4)?,
                target_label: row.get(5)?,
                leave_out_existing: row.get(6)?,
                problem: row.get(7)?,
                last_synced_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    /// Forgets a source and everything about it.
    pub fn remove_source(&self, id: &str) -> Result<()> {
        let mut db = self.db();
        let tx = db.transaction()?;
        tx.execute("DELETE FROM sources WHERE id = ?1", [id])?;
        tx.execute("DELETE FROM records WHERE source = ?1", [id])?;
        tx.execute("DELETE FROM folders WHERE source = ?1", [id])?;
        tx.execute("DELETE FROM uploads WHERE source = ?1", [id])?;
        tx.commit()
    }

    /// Points a source at another server folder and clears its problem. The folders made for it
    /// belong to the old target. What was sent stays recorded, so only new and changed files go
    /// to the new folder.
    pub fn set_target(&self, id: &str, target: Option<&str>, label: &str) -> Result<()> {
        let mut db = self.db();
        let tx = db.transaction()?;
        tx.execute(
            "UPDATE sources SET target = ?2, target_label = ?3, problem = NULL WHERE id = ?1",
            params![id, target, label],
        )?;
        tx.execute("DELETE FROM folders WHERE source = ?1", [id])?;
        tx.commit()
    }

    pub fn set_problem(&self, id: &str, problem: Option<&str>) -> Result<()> {
        self.db().execute(
            "UPDATE sources SET problem = ?2 WHERE id = ?1",
            params![id, problem],
        )?;
        Ok(())
    }

    pub fn existing_seen(&self, id: &str) -> Result<()> {
        self.db().execute(
            "UPDATE sources SET leave_out_existing = 0 WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn set_last_synced(&self, id: &str, at: i64) -> Result<()> {
        self.db().execute(
            "UPDATE sources SET last_synced_at = ?2 WHERE id = ?1",
            params![id, at],
        )?;
        Ok(())
    }

    pub fn records(&self, source: &str) -> Result<HashMap<String, Record>> {
        let db = self.db();
        let mut statement = db.prepare(
            "SELECT key, size, mtime, remote_name, remote_dir FROM records WHERE source = ?1",
        )?;
        let rows = statement.query_map([source], |row| {
            Ok(Record {
                key: row.get(0)?,
                size: row.get::<_, i64>(1)?.cast_unsigned(),
                mtime: row.get(2)?,
                remote_name: row.get(3)?,
                remote_dir: row.get(4)?,
            })
        })?;
        rows.map(|row| row.map(|record| (record.key.clone(), record)))
            .collect()
    }

    pub fn save_record(&self, source: &str, record: &Record) -> Result<()> {
        self.db().execute(
            "INSERT INTO records (source, key, size, mtime, remote_name, remote_dir) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT (source, key) DO UPDATE SET size = excluded.size, \
             mtime = excluded.mtime, remote_name = excluded.remote_name, \
             remote_dir = excluded.remote_dir",
            params![
                source,
                record.key,
                record.size.cast_signed(),
                record.mtime,
                record.remote_name,
                record.remote_dir,
            ],
        )?;
        Ok(())
    }

    pub fn folder(&self, source: &str, dir: &str) -> Result<Option<String>> {
        self.db()
            .query_row(
                "SELECT node FROM folders WHERE source = ?1 AND dir = ?2",
                [source, dir],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn save_folder(&self, source: &str, dir: &str, node: &str) -> Result<()> {
        self.db().execute(
            "INSERT INTO folders (source, dir, node) VALUES (?1, ?2, ?3) \
             ON CONFLICT (source, dir) DO UPDATE SET node = excluded.node",
            [source, dir, node],
        )?;
        Ok(())
    }

    /// Forgets the folders made for a source, after one turned out to be gone.
    pub fn forget_folders(&self, source: &str) -> Result<()> {
        self.db()
            .execute("DELETE FROM folders WHERE source = ?1", [source])?;
        Ok(())
    }

    pub fn saved_uploads(&self, source: &str) -> Result<Vec<SavedUpload>> {
        let db = self.db();
        let mut statement =
            db.prepare("SELECT key, id, parent, name, size, mtime FROM uploads WHERE source = ?1")?;
        let rows = statement.query_map([source], |row| {
            let id: String = row.get(1)?;
            Ok(SavedUpload {
                source: source.to_owned(),
                key: row.get(0)?,
                id: id.parse().unwrap_or(Uuid::nil()),
                parent: row.get(2)?,
                name: row.get(3)?,
                size: row.get::<_, i64>(4)?.cast_unsigned(),
                mtime: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    pub fn save_upload(&self, upload: &SavedUpload) -> Result<()> {
        self.db().execute(
            "INSERT INTO uploads (source, key, id, parent, name, size, mtime) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT (source, key) DO UPDATE SET id = excluded.id, parent = excluded.parent, \
             name = excluded.name, size = excluded.size, mtime = excluded.mtime",
            params![
                upload.source,
                upload.key,
                upload.id.to_string(),
                upload.parent,
                upload.name,
                upload.size.cast_signed(),
                upload.mtime,
            ],
        )?;
        Ok(())
    }

    pub fn forget_upload(&self, source: &str, key: &str) -> Result<()> {
        self.db().execute(
            "DELETE FROM uploads WHERE source = ?1 AND key = ?2",
            [source, key],
        )?;
        Ok(())
    }
}
