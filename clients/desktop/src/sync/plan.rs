use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

use protocol::{NodeId, NodeKind};

use super::{
    layout::Layout,
    model::{Base, LocalEntry, RemoteNode, rebased},
    scan::Scan,
};

/// One step that brings a local path and its remote counterpart back in agreement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The node moved on the server; move the unchanged local copy along.
    MoveLocal {
        node: NodeId,
        from: PathBuf,
        to: PathBuf,
    },
    /// The unchanged file moved locally; move the node along.
    MoveRemote {
        node: NodeId,
        from: PathBuf,
        to: PathBuf,
    },
    CreateLocalDir {
        node: NodeId,
        path: PathBuf,
    },
    CreateRemoteDir {
        path: PathBuf,
    },
    /// Writes the current content of `node` to `path`, unless the local file no longer is
    /// what was scanned (`expect`, `None` for no file).
    Download {
        node: RemoteNode,
        path: PathBuf,
        expect: Option<LocalEntry>,
    },
    /// Sends the local file into `node`, or as a new file by name when `None`. The server
    /// keeps whichever content changed last as current and the other as a version.
    Upload {
        node: Option<NodeId>,
        path: PathBuf,
        local: LocalEntry,
    },
    /// Both sides already agree; only records that.
    Adopt {
        node: NodeId,
        path: PathBuf,
        local: LocalEntry,
    },
    /// Moves the node to the server's recycle bin.
    DeleteRemote {
        node: NodeId,
        path: PathBuf,
    },
    /// Moves the local file to the system trash, unless it changed since the scan.
    TrashLocal {
        node: NodeId,
        path: PathBuf,
        expect: LocalEntry,
    },
    /// Removes the local folder once nothing synced is left in it.
    RemoveLocalDir {
        node: NodeId,
        path: PathBuf,
    },
    /// Drops what was recorded for a node that is gone from both sides.
    Forget {
        node: NodeId,
    },
    /// Needs the user: the path cannot be synced as it is.
    Blocked {
        path: PathBuf,
        reason: String,
    },
}

impl Action {
    /// The local path the action is about; the destination for moves.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::MoveLocal { to, .. } | Self::MoveRemote { to, .. } => Some(to),
            Self::CreateLocalDir { path, .. }
            | Self::CreateRemoteDir { path }
            | Self::Download { path, .. }
            | Self::Upload { path, .. }
            | Self::Adopt { path, .. }
            | Self::DeleteRemote { path, .. }
            | Self::TrashLocal { path, .. }
            | Self::RemoveLocalDir { path, .. }
            | Self::Blocked { path, .. } => Some(path),
            Self::Forget { .. } => None,
        }
    }

    /// Actions run in phases: moves, then folder creation (parents first), then transfers,
    /// then deletions (children first).
    pub fn phase(&self) -> Phase {
        match self {
            Self::MoveLocal { .. } | Self::MoveRemote { .. } => Phase::Move,
            Self::CreateLocalDir { .. } | Self::CreateRemoteDir { .. } => Phase::Create,
            Self::Download { .. }
            | Self::Upload { .. }
            | Self::Adopt { .. }
            | Self::Forget { .. }
            | Self::Blocked { .. } => Phase::Transfer,
            Self::DeleteRemote { .. } | Self::TrashLocal { .. } | Self::RemoveLocalDir { .. } => {
                Phase::Delete
            }
        }
    }

    /// Whether the action keeps something at its path, so its folder must stay.
    fn keeps(&self) -> bool {
        !matches!(
            self,
            Self::DeleteRemote { .. }
                | Self::TrashLocal { .. }
                | Self::RemoveLocalDir { .. }
                | Self::Forget { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    Move,
    Create,
    Transfer,
    Delete,
}

/// Decides what to do to bring the local folders and the server back in agreement.
///
/// Each synced path is compared three ways: the remote node placed there by `layout`, the
/// local entry `scan` found and the `bases` recorded when both last agreed. A side that
/// differs from its base changed; when both changed, the upload goes to the server with its
/// modification time and the server keeps both, the later one current.
pub fn plan(layout: &Layout, scan: &Scan, bases: &[Base]) -> Vec<Action> {
    let mut work = Work {
        layout,
        scan,
        remote: layout.entries.clone(),
        local: scan.entries.clone(),
        bases: bases.iter().map(|base| (base.node, base.clone())).collect(),
        actions: Vec::new(),
    };
    work.remote_moves();
    work.local_moves();
    work.compare();
    work.create_missing_parents();
    work.keep_needed_folders();
    let mut actions = work.actions;
    actions.sort_by(|a, b| {
        let depth = |action: &Action| action.path().map_or(0, |path| path.components().count());
        let order = match a.phase() {
            Phase::Delete => depth(b).cmp(&depth(a)),
            _ => depth(a).cmp(&depth(b)),
        };
        a.phase()
            .cmp(&b.phase())
            .then(order)
            .then_with(|| a.path().cmp(&b.path()))
    });
    actions
}

/// The state being planned on; moves update it so later rules see where things went.
struct Work<'a> {
    layout: &'a Layout,
    scan: &'a Scan,
    remote: HashMap<PathBuf, RemoteNode>,
    local: HashMap<PathBuf, LocalEntry>,
    bases: HashMap<NodeId, Base>,
    actions: Vec<Action>,
}

impl Work<'_> {
    fn in_scope(&self, path: &Path) -> bool {
        self.layout.covers(path) && !self.scan.is_blind(path)
    }

    /// Bases ordered parents first, so a folder moves before anything inside it.
    fn bases_by_depth(&self) -> Vec<NodeId> {
        let mut bases: Vec<&Base> = self.bases.values().collect();
        bases.sort_by(|a, b| {
            a.path
                .components()
                .count()
                .cmp(&b.path.components().count())
                .then_with(|| a.path.cmp(&b.path))
        });
        bases.into_iter().map(|base| base.node).collect()
    }

    fn remote_moves(&mut self) {
        for id in self.bases_by_depth() {
            let Some(base) = self.bases.get(&id) else {
                continue;
            };
            let from = base.path.clone();
            let Some(to) = self.layout.paths.get(&id).cloned() else {
                continue;
            };
            if to == from || !self.in_scope(&from) || !self.in_scope(&to) {
                continue;
            }
            let Some(local) = self.local.get(&from) else {
                continue;
            };
            let unchanged = local.kind == base.kind
                && (base.kind == NodeKind::Folder || local.sha256 == base.sha256);
            if !unchanged || self.local.contains_key(&to) {
                continue;
            }
            self.actions.push(Action::MoveLocal {
                node: id,
                from: from.clone(),
                to: to.clone(),
            });
            self.local = std::mem::take(&mut self.local)
                .into_iter()
                .map(|(path, entry)| (rebased(&path, &from, &to).unwrap_or(path), entry))
                .collect();
            for base in self.bases.values_mut() {
                if let Some(path) = rebased(&base.path, &from, &to) {
                    base.path = path;
                }
            }
        }
    }

    /// A file that vanished from its path while an identical, unknown one appeared elsewhere
    /// was moved; folders moved locally are uploaded anew instead.
    fn local_moves(&mut self) {
        let base_paths: HashSet<PathBuf> =
            self.bases.values().map(|base| base.path.clone()).collect();
        let mut candidates: HashMap<(String, u64), Vec<PathBuf>> = HashMap::new();
        for (path, entry) in &self.local {
            if let Some(sha) = &entry.sha256
                && !base_paths.contains(path)
                && !self.remote.contains_key(path)
                && self.in_scope(path)
            {
                candidates
                    .entry((sha.clone(), entry.size))
                    .or_default()
                    .push(path.clone());
            }
        }
        for id in self.bases_by_depth() {
            let base = &self.bases[&id];
            let (Some(sha), NodeKind::File) = (&base.sha256, base.kind) else {
                continue;
            };
            let from = base.path.clone();
            let unchanged_remote = self
                .remote
                .get(&from)
                .is_some_and(|remote| remote.id == id && remote.sha256 == base.sha256);
            if self.local.contains_key(&from) || !unchanged_remote || !self.in_scope(&from) {
                continue;
            }
            let key = (sha.clone(), base.size);
            let Some([to]) = candidates.get(&key).map(Vec::as_slice) else {
                continue;
            };
            let to = to.clone();
            if !to
                .parent()
                .is_some_and(|parent| self.layout.folders.contains_key(parent))
            {
                continue;
            }
            candidates.remove(&key);
            self.actions.push(Action::MoveRemote {
                node: id,
                from: from.clone(),
                to: to.clone(),
            });
            if let Some(remote) = self.remote.remove(&from) {
                self.remote.insert(to.clone(), remote);
            }
            if let Some(base) = self.bases.get_mut(&id) {
                base.path = to;
            }
        }
    }

    fn compare(&mut self) {
        let base_at: HashMap<PathBuf, NodeId> = self
            .bases
            .values()
            .map(|base| (base.path.clone(), base.node))
            .collect();
        let paths: BTreeSet<PathBuf> = self
            .remote
            .keys()
            .chain(self.local.keys())
            .chain(base_at.keys())
            .cloned()
            .collect();
        for path in paths {
            let base = base_at.get(&path).map(|id| self.bases[id].clone());
            if !self.layout.covers(&path) {
                // A mapping was removed or a folder set to not sync: stop tracking, touch nothing.
                if let Some(base) = base
                    && !self.layout.paths.contains_key(&base.node)
                {
                    self.actions.push(Action::Forget { node: base.node });
                }
                continue;
            }
            if self.scan.is_blind(&path) {
                continue;
            }
            let remote = self.remote.get(&path).cloned();
            let local = self.local.get(&path).cloned();
            let (same, other) = match (&base, &remote) {
                (Some(base), Some(remote)) if base.node != remote.id => (None, Some(base.clone())),
                _ => (base, None),
            };
            if let Some(other) = &other
                && !self.layout.paths.contains_key(&other.node)
            {
                self.actions.push(Action::Forget { node: other.node });
            }
            let kinds = [
                remote.as_ref().map(|remote| remote.kind),
                local.as_ref().map(|local| local.kind),
                same.as_ref().map(|base| base.kind),
            ];
            let mut present = kinds.iter().flatten();
            let Some(kind) = present.next().copied() else {
                continue;
            };
            if present.any(|other| *other != kind) {
                self.actions.push(Action::Blocked {
                    path,
                    reason: "a file and a folder share this name".to_owned(),
                });
                continue;
            }
            match kind {
                NodeKind::File => self.file(path, remote, local, same, other),
                NodeKind::Folder => self.folder(path, remote, local, same),
            }
        }
    }

    fn file(
        &mut self,
        path: PathBuf,
        remote: Option<RemoteNode>,
        local: Option<LocalEntry>,
        base: Option<Base>,
        other: Option<Base>,
    ) {
        if local.as_ref().is_some_and(|local| local.sha256.is_none()) {
            self.actions.push(Action::Blocked {
                path,
                reason: "the file could not be read".to_owned(),
            });
            return;
        }
        let action = match (remote, local, base) {
            (Some(remote), Some(local), Some(base)) => {
                let local_changed = local.sha256 != base.sha256;
                let remote_changed = remote.sha256 != base.sha256;
                match (local_changed, remote_changed) {
                    (false, false) if local.size == base.size && local.mtime == base.mtime => {
                        return;
                    }
                    (false, false) => Action::Adopt {
                        node: remote.id,
                        path,
                        local,
                    },
                    (false, true) => Action::Download {
                        node: remote,
                        path,
                        expect: Some(local),
                    },
                    (true, _) if local.sha256 == remote.sha256 => Action::Adopt {
                        node: remote.id,
                        path,
                        local,
                    },
                    (true, _) => Action::Upload {
                        node: Some(remote.id),
                        path,
                        local,
                    },
                }
            }
            (Some(remote), Some(local), None) => {
                if local.sha256 == remote.sha256 {
                    Action::Adopt {
                        node: remote.id,
                        path,
                        local,
                    }
                } else if other.is_some_and(|other| other.sha256 == local.sha256) {
                    // The local file is what the replaced node held: take the new node.
                    Action::Download {
                        node: remote,
                        path,
                        expect: Some(local),
                    }
                } else {
                    Action::Upload {
                        node: Some(remote.id),
                        path,
                        local,
                    }
                }
            }
            (Some(remote), None, Some(base)) if remote.sha256 == base.sha256 => {
                Action::DeleteRemote {
                    node: remote.id,
                    path,
                }
            }
            // Deleted here but changed on the server: the change wins and comes back.
            (Some(remote), None, _) => Action::Download {
                node: remote,
                path,
                expect: None,
            },
            (None, Some(local), Some(base)) if local.sha256 == base.sha256 => Action::TrashLocal {
                node: base.node,
                path,
                expect: local,
            },
            // New here, or changed here but deleted on the server: it is uploaded as a new
            // file and the deleted one stays in the recycle bin.
            (None, Some(local), _) => Action::Upload {
                node: None,
                path,
                local,
            },
            (None, None, Some(base)) => Action::Forget { node: base.node },
            (None, None, None) => return,
        };
        self.actions.push(action);
    }

    fn folder(
        &mut self,
        path: PathBuf,
        remote: Option<RemoteNode>,
        local: Option<LocalEntry>,
        base: Option<Base>,
    ) {
        let action = match (remote, local, base) {
            (Some(remote), Some(local), None) => Action::Adopt {
                node: remote.id,
                path,
                local,
            },
            (Some(_), Some(_), Some(_)) | (None, None, None) => return,
            (Some(remote), None, Some(_)) => Action::DeleteRemote {
                node: remote.id,
                path,
            },
            (Some(remote), None, None) => Action::CreateLocalDir {
                node: remote.id,
                path,
            },
            (None, Some(_), Some(base)) => Action::RemoveLocalDir {
                node: base.node,
                path,
            },
            (None, Some(_), None) => Action::CreateRemoteDir { path },
            (None, None, Some(base)) => Action::Forget { node: base.node },
        };
        self.actions.push(action);
    }

    /// New uploads need their folders on the server, also those deleted there meanwhile.
    fn create_missing_parents(&mut self) {
        let planned: HashSet<PathBuf> = self
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::CreateRemoteDir { path } => Some(path.clone()),
                _ => None,
            })
            .collect();
        let mut missing = BTreeSet::new();
        for action in &self.actions {
            let path = match action {
                Action::Upload {
                    node: None, path, ..
                }
                | Action::CreateRemoteDir { path } => path,
                _ => continue,
            };
            for parent in path.ancestors().skip(1) {
                let exists = self.layout.roots.iter().any(|root| root == parent)
                    || self
                        .remote
                        .get(parent)
                        .is_some_and(|remote| remote.kind == NodeKind::Folder);
                if exists || !self.layout.covers(parent) {
                    break;
                }
                if !planned.contains(parent) {
                    missing.insert(parent.to_path_buf());
                }
            }
        }
        self.actions.extend(
            missing
                .into_iter()
                .map(|path| Action::CreateRemoteDir { path }),
        );
    }

    /// A folder deleted on one side stays when something inside it is kept, or when the
    /// server would take a folder synced elsewhere down with it.
    fn keep_needed_folders(&mut self) {
        let kept: Vec<PathBuf> = self
            .actions
            .iter()
            .filter(|action| action.keeps())
            .filter_map(|action| action.path().map(Path::to_path_buf))
            .collect();
        let deleted_remote: HashSet<NodeId> = self
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::DeleteRemote { node, .. } => Some(*node),
                _ => None,
            })
            .collect();
        let recreated: HashSet<PathBuf> = self
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::CreateRemoteDir { path } => Some(path.clone()),
                _ => None,
            })
            .collect();
        let keeps_inside =
            |dir: &Path| kept.iter().any(|path| path != dir && path.starts_with(dir));
        let remote = &self.remote;
        let layout = self.layout;
        self.actions = std::mem::take(&mut self.actions)
            .into_iter()
            .filter_map(|action| match &action {
                Action::DeleteRemote { path, .. } => {
                    let is_folder = remote
                        .get(path)
                        .is_some_and(|remote| remote.kind == NodeKind::Folder);
                    let blocked = is_folder
                        && (keeps_inside(path)
                            || layout
                                .skipped
                                .iter()
                                .any(|skipped| skipped.starts_with(path))
                            || remote.iter().any(|(inner, node)| {
                                inner != path
                                    && inner.starts_with(path)
                                    && !deleted_remote.contains(&node.id)
                            }));
                    (!blocked).then_some(action)
                }
                Action::RemoveLocalDir { node, path } => {
                    if recreated.contains(path) {
                        Some(Action::Forget { node: *node })
                    } else {
                        (!keeps_inside(path)).then_some(action)
                    }
                }
                _ => Some(action),
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use protocol::NodeKind::{File, Folder};

    use super::*;
    use crate::sync::{
        layout::tests::{id, node, tree},
        model::Mapping,
    };

    const ROOT: &str = "/sync";

    fn p(rel: &str) -> PathBuf {
        Path::new(ROOT).join(rel)
    }

    fn home(nodes: Vec<RemoteNode>) -> Layout {
        Layout::new(
            &tree(nodes),
            &[Mapping {
                remote: None,
                local: Some(PathBuf::from(ROOT)),
            }],
        )
    }

    fn with_sha(mut node: RemoteNode, sha: &str) -> RemoteNode {
        node.sha256 = Some(sha.to_owned());
        node
    }

    fn file(sha: &str, mtime: i64) -> LocalEntry {
        LocalEntry {
            kind: File,
            size: 1,
            mtime,
            sha256: Some(sha.to_owned()),
        }
    }

    fn base(n: u128, rel: &str, sha: &str) -> Base {
        Base {
            node: id(n),
            path: p(rel),
            kind: File,
            sha256: Some(sha.to_owned()),
            size: 1,
            mtime: 0,
        }
    }

    fn folder_base(n: u128, rel: &str) -> Base {
        Base {
            node: id(n),
            path: p(rel),
            kind: Folder,
            sha256: None,
            size: 0,
            mtime: 0,
        }
    }

    fn scan(entries: Vec<(&str, LocalEntry)>) -> Scan {
        Scan {
            entries: entries
                .into_iter()
                .map(|(rel, entry)| (p(rel), entry))
                .collect(),
            blind: Vec::new(),
        }
    }

    #[test]
    fn nothing_changed_plans_nothing() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("x", 0))]),
            &[base(1, "a.txt", "x")],
        );
        assert_eq!(actions, []);
    }

    #[test]
    fn a_remote_change_is_downloaded() {
        let remote = with_sha(node(1, None, File, "a.txt"), "new");
        let layout = home(vec![remote.clone()]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("old", 0))]),
            &[base(1, "a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Download {
                node: remote,
                path: p("a.txt"),
                expect: Some(file("old", 0)),
            }]
        );
    }

    #[test]
    fn a_local_change_is_uploaded_into_the_node() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "old")]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("new", 5))]),
            &[base(1, "a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Upload {
                node: Some(id(1)),
                path: p("a.txt"),
                local: file("new", 5),
            }]
        );
    }

    /// Scenario 3: both sides changed the file; the server keeps both, the later current.
    #[test]
    fn both_sides_changed_uploads_with_the_local_time() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "theirs")]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("mine", 5))]),
            &[base(1, "a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Upload {
                node: Some(id(1)),
                path: p("a.txt"),
                local: file("mine", 5),
            }]
        );
    }

    #[test]
    fn both_sides_made_the_same_change_only_records_it() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "same")]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("same", 5))]),
            &[base(1, "a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Adopt {
                node: id(1),
                path: p("a.txt"),
                local: file("same", 5),
            }]
        );
    }

    /// Scenario 1: the same name was created on both sides with different content.
    #[test]
    fn a_file_created_on_both_sides_is_merged_into_the_remote_one() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "theirs")]);
        let actions = plan(&layout, &scan(vec![("a.txt", file("mine", 5))]), &[]);
        assert_eq!(
            actions,
            [Action::Upload {
                node: Some(id(1)),
                path: p("a.txt"),
                local: file("mine", 5),
            }]
        );
    }

    #[test]
    fn identical_files_on_both_sides_are_adopted() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let actions = plan(&layout, &scan(vec![("a.txt", file("x", 5))]), &[]);
        assert!(matches!(actions.as_slice(), [Action::Adopt { .. }]));
    }

    /// Scenario 2: deleted on the server, changed here: uploaded as a new file.
    #[test]
    fn a_local_change_to_a_remotely_deleted_file_is_uploaded_anew() {
        let layout = home(vec![]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("mine", 5))]),
            &[base(1, "a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Upload {
                node: None,
                path: p("a.txt"),
                local: file("mine", 5),
            }]
        );
    }

    #[test]
    fn an_unchanged_file_deleted_on_the_server_is_trashed_locally() {
        let layout = home(vec![]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("x", 0))]),
            &[base(1, "a.txt", "x")],
        );
        assert_eq!(
            actions,
            [Action::TrashLocal {
                node: id(1),
                path: p("a.txt"),
                expect: file("x", 0),
            }]
        );
    }

    #[test]
    fn an_unchanged_file_deleted_locally_is_deleted_on_the_server() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let actions = plan(&layout, &scan(vec![]), &[base(1, "a.txt", "x")]);
        assert_eq!(
            actions,
            [Action::DeleteRemote {
                node: id(1),
                path: p("a.txt"),
            }]
        );
    }

    /// Mirror of scenario 2: deleted here, changed on the server: the change comes back.
    #[test]
    fn a_remote_change_to_a_locally_deleted_file_is_downloaded() {
        let remote = with_sha(node(1, None, File, "a.txt"), "theirs");
        let layout = home(vec![remote.clone()]);
        let actions = plan(&layout, &scan(vec![]), &[base(1, "a.txt", "old")]);
        assert_eq!(
            actions,
            [Action::Download {
                node: remote,
                path: p("a.txt"),
                expect: None,
            }]
        );
    }

    #[test]
    fn a_file_gone_from_both_sides_is_forgotten() {
        let actions = plan(&home(vec![]), &scan(vec![]), &[base(1, "a.txt", "x")]);
        assert_eq!(actions, [Action::Forget { node: id(1) }]);
    }

    #[test]
    fn a_new_local_file_in_a_new_folder_creates_the_folder_first() {
        let actions = plan(
            &home(vec![]),
            &scan(vec![
                ("dir", LocalEntry::folder()),
                ("dir/a.txt", file("x", 0)),
            ]),
            &[],
        );
        assert_eq!(
            actions,
            [
                Action::CreateRemoteDir { path: p("dir") },
                Action::Upload {
                    node: None,
                    path: p("dir/a.txt"),
                    local: file("x", 0),
                },
            ]
        );
    }

    #[test]
    fn a_new_remote_folder_is_created_before_its_files_download() {
        let remote = node(2, Some(1), File, "a.txt");
        let layout = home(vec![node(1, None, Folder, "dir"), remote.clone()]);
        let actions = plan(&layout, &scan(vec![]), &[]);
        assert_eq!(
            actions,
            [
                Action::CreateLocalDir {
                    node: id(1),
                    path: p("dir"),
                },
                Action::Download {
                    node: remote,
                    path: p("dir/a.txt"),
                    expect: None,
                },
            ]
        );
    }

    #[test]
    fn a_remote_rename_moves_the_local_file() {
        let layout = home(vec![with_sha(node(1, None, File, "b.txt"), "x")]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("x", 0))]),
            &[base(1, "a.txt", "x")],
        );
        assert_eq!(
            actions,
            [Action::MoveLocal {
                node: id(1),
                from: p("a.txt"),
                to: p("b.txt"),
            }]
        );
    }

    #[test]
    fn a_remote_folder_move_moves_its_content_along() {
        let layout = home(vec![
            node(1, None, Folder, "new"),
            with_sha(node(2, Some(1), File, "a.txt"), "x"),
        ]);
        let actions = plan(
            &layout,
            &scan(vec![
                ("old", LocalEntry::folder()),
                ("old/a.txt", file("x", 0)),
            ]),
            &[folder_base(1, "old"), base(2, "old/a.txt", "x")],
        );
        assert_eq!(
            actions,
            [Action::MoveLocal {
                node: id(1),
                from: p("old"),
                to: p("new"),
            }]
        );
    }

    #[test]
    fn a_local_rename_moves_the_remote_file() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let actions = plan(
            &layout,
            &scan(vec![("b.txt", file("x", 0))]),
            &[base(1, "a.txt", "x")],
        );
        assert_eq!(
            actions,
            [Action::MoveRemote {
                node: id(1),
                from: p("a.txt"),
                to: p("b.txt"),
            }]
        );
    }

    #[test]
    fn a_folder_deleted_locally_is_deleted_on_the_server_after_its_files() {
        let layout = home(vec![
            node(1, None, Folder, "dir"),
            with_sha(node(2, Some(1), File, "a.txt"), "x"),
        ]);
        let actions = plan(
            &layout,
            &scan(vec![]),
            &[folder_base(1, "dir"), base(2, "dir/a.txt", "x")],
        );
        assert_eq!(
            actions,
            [
                Action::DeleteRemote {
                    node: id(2),
                    path: p("dir/a.txt"),
                },
                Action::DeleteRemote {
                    node: id(1),
                    path: p("dir"),
                },
            ]
        );
    }

    #[test]
    fn a_folder_deleted_locally_stays_on_the_server_when_a_file_in_it_changed() {
        let remote = with_sha(node(2, Some(1), File, "a.txt"), "theirs");
        let layout = home(vec![node(1, None, Folder, "dir"), remote.clone()]);
        let actions = plan(
            &layout,
            &scan(vec![]),
            &[folder_base(1, "dir"), base(2, "dir/a.txt", "old")],
        );
        assert_eq!(
            actions,
            [Action::Download {
                node: remote,
                path: p("dir/a.txt"),
                expect: None,
            }]
        );
    }

    #[test]
    fn a_folder_deleted_on_the_server_keeps_a_locally_changed_file_and_is_recreated() {
        let actions = plan(
            &home(vec![]),
            &scan(vec![
                ("dir", LocalEntry::folder()),
                ("dir/a.txt", file("mine", 5)),
            ]),
            &[folder_base(1, "dir"), base(2, "dir/a.txt", "old")],
        );
        assert_eq!(
            actions,
            [
                Action::CreateRemoteDir { path: p("dir") },
                Action::Forget { node: id(1) },
                Action::Upload {
                    node: None,
                    path: p("dir/a.txt"),
                    local: file("mine", 5),
                },
            ]
        );
    }

    #[test]
    fn a_folder_with_a_mapping_of_its_own_is_never_deleted_with_its_parent() {
        let nodes = tree(vec![
            node(1, None, Folder, "dir"),
            node(2, Some(1), Folder, "mapped"),
        ]);
        let layout = Layout::new(
            &nodes,
            &[
                Mapping {
                    remote: None,
                    local: Some(PathBuf::from(ROOT)),
                },
                Mapping {
                    remote: Some(id(2)),
                    local: Some(PathBuf::from("/elsewhere")),
                },
            ],
        );
        let actions = plan(&layout, &scan(vec![]), &[folder_base(1, "dir")]);
        assert_eq!(actions, []);
    }

    #[test]
    fn nothing_is_deleted_under_a_missing_mapped_folder() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let blind = Scan {
            entries: HashMap::new(),
            blind: vec![PathBuf::from(ROOT)],
        };
        assert_eq!(plan(&layout, &blind, &[base(1, "a.txt", "x")]), []);
    }

    #[test]
    fn removing_a_mapping_forgets_its_files_without_touching_them() {
        let layout = Layout::new(&tree(vec![]), &[]);
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", file("x", 0))]),
            &[base(1, "a.txt", "x")],
        );
        assert_eq!(actions, [Action::Forget { node: id(1) }]);
    }

    #[test]
    fn a_file_and_a_folder_of_the_same_name_are_blocked() {
        let layout = home(vec![node(1, None, Folder, "a")]);
        let actions = plan(&layout, &scan(vec![("a", file("x", 0))]), &[]);
        assert!(matches!(actions.as_slice(), [Action::Blocked { .. }]));
    }

    #[test]
    fn an_unreadable_file_is_blocked_rather_than_deleted() {
        let layout = home(vec![with_sha(node(1, None, File, "a.txt"), "x")]);
        let unreadable = LocalEntry {
            sha256: None,
            ..file("x", 5)
        };
        let actions = plan(
            &layout,
            &scan(vec![("a.txt", unreadable)]),
            &[base(1, "a.txt", "x")],
        );
        assert!(matches!(actions.as_slice(), [Action::Blocked { .. }]));
    }
}
