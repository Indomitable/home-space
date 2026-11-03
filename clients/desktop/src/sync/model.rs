use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use time::OffsetDateTime;
use uuid::Uuid;

use protocol::{Node, NodeId, NodeKind};

/// A remote file or folder as the sync engine remembers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub kind: NodeKind,
    pub name: String,
    pub size: i64,
    pub sha256: Option<String>,
    pub updated_at: OffsetDateTime,
}

impl From<Node> for RemoteNode {
    fn from(node: Node) -> Self {
        Self {
            id: node.id,
            parent: node.parent_id,
            kind: node.kind,
            name: node.name,
            size: node.size,
            sha256: node.sha256,
            updated_at: node.updated_at,
        }
    }
}

/// Where a remote folder syncs to: `remote` is `None` for Home, `local` is `None` when the
/// folder is set to not sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    pub remote: Option<NodeId>,
    pub local: Option<PathBuf>,
}

/// A local file or folder found by a scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEntry {
    pub kind: NodeKind,
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch.
    pub mtime: i64,
    /// Lowercase hex SHA-256 of a file; `None` for folders, files that could not be read and
    /// files left `unhashed`.
    pub sha256: Option<String>,
    /// A new file whose hash the plan does not need: nothing is recorded or on the server at
    /// its path and it cannot be a moved file. It is hashed when it is uploaded.
    pub unhashed: bool,
}

impl LocalEntry {
    pub fn folder() -> Self {
        Self {
            kind: NodeKind::Folder,
            size: 0,
            mtime: 0,
            sha256: None,
            unhashed: false,
        }
    }

    /// Whether the file was meant to be hashed but could not be read.
    pub fn is_unreadable(&self) -> bool {
        self.kind == NodeKind::File && self.sha256.is_none() && !self.unhashed
    }
}

/// The last state both sides agreed on for a node, which tells which side changed since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base {
    pub node: NodeId,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub sha256: Option<String>,
    pub size: u64,
    pub mtime: i64,
}

impl Base {
    pub fn new(node: NodeId, path: PathBuf, local: &LocalEntry) -> Self {
        Self {
            node,
            path,
            kind: local.kind,
            sha256: local.sha256.clone(),
            size: local.size,
            mtime: local.mtime,
        }
    }
}

/// A chunked upload under way, saved so that it continues after a restart. The bytes the
/// server holds are only worth keeping while the file is still the one described here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedUpload {
    pub path: PathBuf,
    /// The server's upload session.
    pub id: Uuid,
    /// The file it replaces; `None` for a new file named `name` in `parent`.
    pub node: Option<NodeId>,
    pub parent: Option<NodeId>,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
    pub sha256: String,
}

impl SavedUpload {
    /// Whether this upload sends the same content to the same place as `other`.
    pub fn is_same(&self, other: &Self) -> bool {
        *self
            == Self {
                id: self.id,
                ..other.clone()
            }
    }
}

/// `time` as nanoseconds since the Unix epoch, saturating outside what fits.
pub fn nanos(time: SystemTime) -> i64 {
    let nanos = OffsetDateTime::from(time).unix_timestamp_nanos();
    i64::try_from(nanos).unwrap_or(if nanos < 0 { i64::MIN } else { i64::MAX })
}

/// The time `nanos` after the Unix epoch.
pub fn datetime(nanos: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos))
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

/// `path` moved from under `from` to under `to`, or `None` when it is not under `from`.
pub fn rebased(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    path.strip_prefix(from).ok().map(|rest| {
        if rest.as_os_str().is_empty() {
            to.to_path_buf()
        } else {
            to.join(rest)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebased_moves_paths_under_the_old_prefix_only() {
        let (from, to) = (Path::new("/a/b"), Path::new("/c"));
        assert_eq!(
            rebased(Path::new("/a/b"), from, to),
            Some(PathBuf::from("/c"))
        );
        assert_eq!(
            rebased(Path::new("/a/b/x/y.txt"), from, to),
            Some(PathBuf::from("/c/x/y.txt"))
        );
        assert_eq!(rebased(Path::new("/a/bc"), from, to), None);
    }

    #[test]
    fn nanos_round_trip_through_datetime() {
        let at =
            SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(1_700_000_000_123_456_789);
        assert_eq!(nanos(at), 1_700_000_000_123_456_789);
        assert_eq!(datetime(nanos(at)), OffsetDateTime::from(at));
    }
}
