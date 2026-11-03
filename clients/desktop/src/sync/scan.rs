use std::{
    collections::{HashMap, HashSet},
    fs, io,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use sha2::{Digest, Sha256};

use protocol::NodeKind;

use super::model::{Base, LocalEntry, nanos};

const HASH_BUFFER: usize = 64 * 1024;
/// The most files hashed at once.
const HASHERS: usize = 8;
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
    /// Why the blind folders and files could not be read, for the user.
    pub unreadable: Vec<(PathBuf, String)>,
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
/// A file whose size and modification time match what `known` recorded at its path keeps
/// the recorded hash. Other files are hashed, several at a time, when the plan needs the
/// hash: something is recorded or on the server (`remote`) at their path, or they may be a
/// recorded file moved here, having the size of one gone from its path. The rest are new
/// and left `unhashed` for the upload. Symbolic links are not followed or synced. Once
/// `stop` is set the scan gives up, leaving every root blind.
pub fn scan(
    roots: &[PathBuf],
    skipped: &HashSet<PathBuf>,
    known: &HashMap<PathBuf, Base>,
    remote: &HashSet<PathBuf>,
    stop: &AtomicBool,
) -> Scan {
    let stopped = || Scan {
        blind: roots.to_vec(),
        ..Scan::default()
    };
    let Some(mut scan) = walk(roots, skipped, known, stop) else {
        return stopped();
    };
    let gone_sizes: HashSet<u64> = known
        .values()
        .filter(|base| base.kind == NodeKind::File)
        .filter(|base| {
            scan.entries
                .get(&base.path)
                .is_none_or(|entry| entry.kind != NodeKind::File)
        })
        .map(|base| base.size)
        .collect();
    let to_hash: Vec<PathBuf> = scan
        .entries
        .iter()
        .filter(|(path, entry)| {
            entry.unhashed
                && (remote.contains(*path)
                    || known.contains_key(*path)
                    || gone_sizes.contains(&entry.size))
        })
        .map(|(path, _)| path.clone())
        .collect();
    let hashed = hash_all(to_hash, stop);
    // Some files were left out or failed as interrupted.
    if stop.load(Ordering::Relaxed) {
        return stopped();
    }
    for (path, sha256) in hashed {
        if let Some(entry) = scan.entries.get_mut(&path) {
            entry.sha256 = sha256.ok();
            entry.unhashed = false;
        }
    }
    scan
}

/// Hashes `paths` on a few threads at once, as many as the machine has cores up to
/// [`HASHERS`].
fn hash_all(paths: Vec<PathBuf>, stop: &AtomicBool) -> Vec<(PathBuf, io::Result<String>)> {
    let threads = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, HASHERS)
        .min(paths.len());
    if threads <= 1 {
        return paths
            .into_iter()
            .map(|path| {
                let sha256 = hash_file(&path, stop);
                (path, sha256)
            })
            .collect();
    }
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut hashed = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(at) else { break };
                        if stop.load(Ordering::Relaxed) {
                            break;
                        }
                        hashed.push((path.clone(), hash_file(path, stop)));
                    }
                    hashed
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect()
    })
}

/// Lists the entries under `roots`, leaving files without a recorded hash `unhashed`;
/// `None` once `stop` is set.
fn walk(
    roots: &[PathBuf],
    skipped: &HashSet<PathBuf>,
    known: &HashMap<PathBuf, Base>,
    stop: &AtomicBool,
) -> Option<Scan> {
    let mut scan = Scan::default();
    for root in roots {
        if !root.is_dir() {
            scan.blind.push(root.clone());
            scan.unreadable
                .push((root.clone(), "the folder is missing".to_owned()));
            continue;
        }
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            let listing = match fs::read_dir(&dir) {
                Ok(listing) => listing,
                Err(err) => {
                    scan.unreadable
                        .push((dir.clone(), format!("cannot read the folder: {err}")));
                    scan.blind.push(dir);
                    continue;
                }
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
                let meta = match entry.metadata() {
                    Ok(meta) => meta,
                    Err(err) => {
                        scan.unreadable.push((path.clone(), err.to_string()));
                        scan.blind.push(path);
                        continue;
                    }
                };
                if meta.is_dir() {
                    scan.entries.insert(path.clone(), LocalEntry::folder());
                    pending.push(path);
                } else if meta.is_file() {
                    let entry = file_entry(&meta, known.get(&path));
                    scan.entries.insert(path, entry);
                }
            }
        }
    }
    Some(scan)
}

fn file_entry(meta: &fs::Metadata, known: Option<&Base>) -> LocalEntry {
    let size = meta.len();
    let mtime = meta.modified().map(nanos).unwrap_or(0);
    let recorded = known.and_then(|base| {
        let same = base.kind == NodeKind::File && base.size == size && base.mtime == mtime;
        same.then(|| base.sha256.clone()).flatten()
    });
    LocalEntry {
        kind: NodeKind::File,
        size,
        mtime,
        unhashed: recorded.is_none(),
        sha256: recorded,
    }
}

/// The lowercase hex SHA-256 of the file at `path`; fails as interrupted once `stop` is set.
pub fn hash_file(path: &Path, stop: &AtomicBool) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; HASH_BUFFER];
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err(io::ErrorKind::Interrupted.into());
        }
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
        let remote = HashSet::from([root.join("docs/a.txt")]);

        let scan = scan(
            std::slice::from_ref(&root),
            &skipped,
            &HashMap::new(),
            &remote,
            &AtomicBool::new(false),
        );

        let mut paths: Vec<_> = scan.entries.keys().cloned().collect();
        paths.sort();
        assert_eq!(paths, [root.join("docs"), root.join("docs/a.txt")]);
        let a = &scan.entries[&root.join("docs/a.txt")];
        assert_eq!(
            a.sha256.as_deref(),
            Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
        assert!(!a.unhashed);
    }

    #[test]
    fn new_files_are_left_unhashed_unless_they_may_be_moved() {
        let dir = TempDir::new("unhashed");
        let root = dir.0.clone();
        for (name, content) in [("new.txt", "hello"), ("moved.txt", "abc"), ("c.txt", "c")] {
            fs::write(root.join(name), content).expect("file");
        }
        let gone = |n: u128, name: &str, size: u64| Base {
            node: crate::sync::layout::tests::id(n),
            path: root.join(name),
            kind: NodeKind::File,
            sha256: Some("old".to_owned()),
            size,
            mtime: 0,
        };
        // Gone from its path with the size of moved.txt; c.txt is still where it was.
        let known = HashMap::from([
            (root.join("was.txt"), gone(1, "was.txt", 3)),
            (root.join("c.txt"), gone(2, "c.txt", 5)),
        ]);

        let scan = scan(
            std::slice::from_ref(&root),
            &HashSet::new(),
            &known,
            &HashSet::new(),
            &AtomicBool::new(false),
        );

        let new = &scan.entries[&root.join("new.txt")];
        assert!(new.unhashed && new.sha256.is_none() && !new.is_unreadable());
        let moved = &scan.entries[&root.join("moved.txt")];
        assert!(!moved.unhashed && moved.sha256.is_some());
        // Recorded at its path but changed: hashed.
        let changed = &scan.entries[&root.join("c.txt")];
        assert!(!changed.unhashed && changed.sha256.is_some());
    }

    #[test]
    fn many_files_are_hashed_in_parallel() {
        let dir = TempDir::new("parallel");
        let root = dir.0.clone();
        let names: Vec<String> = (0..50).map(|n| format!("f{n}.txt")).collect();
        for name in &names {
            fs::write(root.join(name), name).expect("file");
        }
        let remote = names.iter().map(|name| root.join(name)).collect();

        let scan = scan(
            std::slice::from_ref(&root),
            &HashSet::new(),
            &HashMap::new(),
            &remote,
            &AtomicBool::new(false),
        );

        let stop = AtomicBool::new(false);
        for name in &names {
            let path = root.join(name);
            assert_eq!(
                scan.entries[&path].sha256.as_ref().ok_or("unhashed"),
                hash_file(&path, &stop).as_ref().map_err(|_| "unreadable")
            );
        }
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

        let scan = scan(
            std::slice::from_ref(&dir.0),
            &HashSet::new(),
            &known,
            &HashSet::new(),
            &AtomicBool::new(false),
        );

        assert_eq!(scan.entries[&path].sha256.as_deref(), Some("recorded"));
    }

    #[test]
    fn a_missing_root_is_blind() {
        let root = std::env::temp_dir().join("home-space-scan-missing-root");
        let scan = scan(
            std::slice::from_ref(&root),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &AtomicBool::new(false),
        );
        assert!(scan.entries.is_empty());
        assert!(scan.is_blind(&root.join("anything")));
    }

    #[test]
    fn a_stopped_scan_knows_nothing() {
        let dir = TempDir::new("stopped");
        fs::write(dir.0.join("a.txt"), "x").expect("a");

        let scan = scan(
            std::slice::from_ref(&dir.0),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &AtomicBool::new(true),
        );

        assert!(scan.entries.is_empty());
        assert!(scan.is_blind(&dir.0.join("a.txt")));
    }

    #[test]
    fn office_locks_and_partial_downloads_are_ignored() {
        assert!(is_ignored("~$report.docx"));
        assert!(is_ignored("a.pdf.part"));
        assert!(!is_ignored("report.docx"));
    }
}
