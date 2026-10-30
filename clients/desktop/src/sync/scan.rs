use std::{
    collections::{HashMap, HashSet},
    fs, io,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use protocol::NodeKind;

use super::model::{Base, LocalEntry, nanos};

const HASH_BUFFER: usize = 64 * 1024;
/// Names the sync never uploads: system clutter, editor locks and our own partial downloads.
const IGNORED_NAMES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini", ".localized"];
const IGNORED_PREFIXES: &[&str] = &["~$", ".~lock.", "._"];
const IGNORED_SUFFIXES: &[&str] = &[".part", ".tmp", ".swp"];

/// What a scan found under the mapped folders.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub entries: HashMap<PathBuf, LocalEntry>,
    /// Folders whose content is unknown: a missing mapped folder (such as an unmounted
    /// drive) or one that could not be listed. Nothing under them may be treated as deleted.
    pub blind: Vec<PathBuf>,
}

impl Scan {
    /// Whether nothing is known about `path`.
    pub fn is_blind(&self, path: &Path) -> bool {
        self.blind.iter().any(|blind| path.starts_with(blind))
    }
}

/// Whether the sync leaves a file of this name alone.
pub fn is_ignored(name: &str) -> bool {
    IGNORED_NAMES.contains(&name)
        || IGNORED_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
        || IGNORED_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}

/// Lists everything under `roots` except the `skipped` paths.
///
/// Files are hashed unless their size and modification time match what `known` recorded at
/// the same path. Symbolic links are not followed or synced.
pub fn scan(roots: &[PathBuf], skipped: &HashSet<PathBuf>, known: &HashMap<PathBuf, Base>) -> Scan {
    let mut scan = Scan::default();
    for root in roots {
        if !root.is_dir() {
            scan.blind.push(root.clone());
            continue;
        }
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            let Ok(listing) = fs::read_dir(&dir) else {
                scan.blind.push(dir);
                continue;
            };
            for entry in listing.flatten() {
                let path = entry.path();
                let ignored = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_none_or(is_ignored);
                if ignored || skipped.contains(&path) || roots.contains(&path) {
                    continue;
                }
                let Ok(meta) = entry.metadata() else {
                    scan.blind.push(path);
                    continue;
                };
                if meta.is_dir() {
                    scan.entries.insert(path.clone(), LocalEntry::folder());
                    pending.push(path);
                } else if meta.is_file() {
                    let entry = file_entry(&path, &meta, known.get(&path));
                    scan.entries.insert(path, entry);
                }
            }
        }
    }
    scan
}

fn file_entry(path: &Path, meta: &fs::Metadata, known: Option<&Base>) -> LocalEntry {
    let size = meta.len();
    let mtime = meta.modified().map(nanos).unwrap_or(0);
    let sha256 = match known {
        Some(base) if base.kind == NodeKind::File && base.size == size && base.mtime == mtime => {
            base.sha256.clone()
        }
        _ => hash_file(path).ok(),
    };
    LocalEntry {
        kind: NodeKind::File,
        size,
        mtime,
        sha256,
    }
}

/// The lowercase hex SHA-256 of the file at `path`.
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; HASH_BUFFER];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("home-space-scan-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temp dir");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn scan_hashes_files_and_leaves_out_ignored_and_skipped_paths() {
        let dir = TempDir::new("basic");
        let root = dir.0.clone();
        fs::create_dir_all(root.join("docs")).expect("docs");
        fs::create_dir_all(root.join("skip")).expect("skip");
        fs::write(root.join("docs/a.txt"), "hello").expect("a");
        fs::write(root.join("skip/b.txt"), "b").expect("b");
        fs::write(root.join(".DS_Store"), "x").expect("ds");
        fs::write(root.join("c.txt.part"), "x").expect("part");
        let skipped = HashSet::from([root.join("skip")]);

        let scan = scan(std::slice::from_ref(&root), &skipped, &HashMap::new());

        let mut paths: Vec<_> = scan.entries.keys().cloned().collect();
        paths.sort();
        assert_eq!(paths, [root.join("docs"), root.join("docs/a.txt")]);
        assert_eq!(
            scan.entries[&root.join("docs/a.txt")].sha256.as_deref(),
            Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
    }

    #[test]
    fn an_unchanged_file_keeps_the_recorded_hash() {
        let dir = TempDir::new("known");
        let path = dir.0.join("a.txt");
        fs::write(&path, "hello").expect("a");
        let meta = fs::metadata(&path).expect("meta");
        let base = Base {
            node: crate::sync::layout::tests::id(1),
            path: path.clone(),
            kind: NodeKind::File,
            sha256: Some("recorded".to_owned()),
            size: meta.len(),
            mtime: nanos(meta.modified().expect("mtime")),
        };
        let known = HashMap::from([(path.clone(), base)]);

        let scan = scan(std::slice::from_ref(&dir.0), &HashSet::new(), &known);

        assert_eq!(scan.entries[&path].sha256.as_deref(), Some("recorded"));
    }

    #[test]
    fn a_missing_root_is_blind() {
        let root = std::env::temp_dir().join("home-space-scan-missing-root");
        let scan = scan(
            std::slice::from_ref(&root),
            &HashSet::new(),
            &HashMap::new(),
        );
        assert!(scan.entries.is_empty());
        assert!(scan.is_blind(&root.join("anything")));
    }

    #[test]
    fn office_locks_and_partial_downloads_are_ignored() {
        assert!(is_ignored("~$report.docx"));
        assert!(is_ignored("a.pdf.part"));
        assert!(!is_ignored("report.docx"));
    }
}
