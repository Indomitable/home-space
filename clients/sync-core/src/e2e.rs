//! The sync engine against a real server; ignored unless asked for:
//!
//! ```sh
//! HOME_SPACE_E2E_SERVER=http://127.0.0.1:8080 HOME_SPACE_E2E_USER=alice \
//! HOME_SPACE_E2E_PASSWORD=... cargo test -p sync-core e2e -- --ignored --test-threads 1
//! ```
//!
//! Each run works in a fresh root folder of that account. A second client stands for the
//! other user (another device, or the web client) in the conflict scenarios.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use futures::channel::mpsc;

use api_client::Client;
use protocol::{Node, NodeId, NodeKind};

use std::sync::Arc;

use super::{Command, Engine, Mapping, Platform, PrivateFiles, Update, store::Store};

/// The desktop client's rules: the user owns the local folders.
struct UserFolders;

impl Platform for UserFolders {
    fn app_owned_copies(&self) -> bool {
        false
    }
}

const PASSES: usize = 4;

struct Setup {
    engine: Engine,
    other: Client,
    output: mpsc::Sender<Update>,
    _updates: mpsc::Receiver<Update>,
    commands: tokio::sync::mpsc::UnboundedReceiver<Command>,
    _commands: tokio::sync::mpsc::UnboundedSender<Command>,
    remote: NodeId,
    local: PathBuf,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

async fn client() -> Client {
    let client = Client::new(env("HOME_SPACE_E2E_SERVER"));
    client
        .login(&env("HOME_SPACE_E2E_USER"), &env("HOME_SPACE_E2E_PASSWORD"))
        .await
        .expect("login");
    client
}

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "home-space-e2e-{name}-{}-{}",
        std::process::id(),
        stamp()
    ));
    fs::create_dir_all(&path).expect("local folder");
    path
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
}

async fn setup(name: &str) -> Setup {
    let mine = client().await;
    let other = client().await;
    let remote = other
        .create_folder(None, &format!("sync-e2e-{name}-{}", stamp()))
        .await
        .expect("remote folder")
        .id;
    let local = temp_dir(name);
    let (mut output, updates) = mpsc::channel(4096);
    let (sender, commands) = tokio::sync::mpsc::unbounded_channel();
    let mut engine = Engine::load(
        mine,
        Store::in_memory().expect("store"),
        Arc::new(UserFolders),
    )
    .expect("engine");
    engine
        .handle(
            Command::Map(Mapping {
                remote: Some(remote),
                local: Some(local.clone()),
            }),
            &mut output,
        )
        .await;
    Setup {
        engine,
        other,
        output,
        commands,
        _commands: sender,
        _updates: updates,
        remote,
        local,
    }
}

impl Setup {
    /// Runs passes until the engine has nothing left to do.
    async fn sync(&mut self) {
        for _ in 0..PASSES {
            match self.engine.pass(&mut self.output, &mut self.commands).await {
                Ok(None) => return,
                Ok(Some(_)) => {}
                Err(_) => panic!("signed out"),
            }
        }
    }

    async fn remote_file(&self, name: &str) -> Option<Node> {
        self.other
            .list(Some(self.remote))
            .await
            .expect("list")
            .into_iter()
            .find(|node| node.kind == NodeKind::File && node.name == name)
    }

    async fn upload(&self, parent: NodeId, name: &str, content: &'static str) -> Node {
        self.other
            .upload(Some(parent), name, || reqwest::Body::from(content))
            .await
            .expect("upload")
    }

    async fn content(&self, id: NodeId) -> String {
        self.other
            .download(id)
            .await
            .expect("download")
            .text()
            .await
            .expect("text")
    }

    async fn version_content(&self, id: NodeId, version: i32) -> String {
        self.other
            .download_version(id, version)
            .await
            .expect("version")
            .text()
            .await
            .expect("text")
    }
}

fn write(path: &Path, content: &str, age: Duration) {
    fs::write(path, content).expect("write");
    let file = fs::File::options().write(true).open(path).expect("open");
    file.set_modified(SystemTime::now() - age).expect("mtime");
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("read")
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn files_sync_both_ways() {
    let mut s = setup("both-ways").await;
    s.upload(s.remote, "from-server.txt", "server").await;
    write(&s.local.join("from-here.txt"), "here", Duration::ZERO);
    s.sync().await;
    assert_eq!(read(&s.local.join("from-server.txt")), "server");
    let uploaded = s.remote_file("from-here.txt").await.expect("uploaded");
    assert_eq!(s.content(uploaded.id).await, "here");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_folder_moved_here_is_moved_on_the_server_not_uploaded_again() {
    let mut s = setup("folder-move").await;
    let folder = s
        .other
        .create_folder(Some(s.remote), "old")
        .await
        .expect("folder");
    let file = s.upload(folder.id, "a.txt", "content").await;
    s.sync().await;

    fs::rename(s.local.join("old"), s.local.join("new")).expect("rename");
    s.sync().await;

    let moved = s.other.get(folder.id).await.expect("folder kept");
    assert_eq!(moved.name, "new");
    let inside = s.other.list(Some(folder.id)).await.expect("list");
    assert_eq!(
        inside.iter().map(|node| node.id).collect::<Vec<_>>(),
        [file.id]
    );
    assert_eq!(s.content(file.id).await, "content");
}

/// Scenario 1: the same new file on both sides; the later one is current, the other a
/// version.
#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_file_created_on_both_sides_keeps_both_versions() {
    let mut s = setup("created-twice").await;
    s.upload(s.remote, "a.txt", "server").await;
    write(
        &s.local.join("a.txt"),
        "older here",
        Duration::from_secs(3600),
    );
    s.sync().await;
    let file = s.remote_file("a.txt").await.expect("file");
    assert_eq!(s.content(file.id).await, "server");
    assert_eq!(s.version_content(file.id, 1).await, "older here");
    assert_eq!(read(&s.local.join("a.txt")), "server");
}

/// Scenario 2: deleted on the server while changed here: the deleted one stays in the
/// recycle bin and the changed one comes back as a new file.
#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_file_changed_here_after_a_remote_delete_is_uploaded_anew() {
    let mut s = setup("deleted-changed").await;
    let original = s.upload(s.remote, "b.txt", "original").await;
    s.sync().await;
    s.other.delete(original.id).await.expect("delete");
    write(&s.local.join("b.txt"), "changed here", Duration::ZERO);
    s.sync().await;
    let revived = s.remote_file("b.txt").await.expect("uploaded anew");
    assert_ne!(revived.id, original.id);
    assert_eq!(s.content(revived.id).await, "changed here");
    let binned = s.other.trash().await.expect("trash");
    assert!(binned.iter().any(|item| item.node.id == original.id));
}

/// Scenario 3: both sides changed the file; the later change is current.
#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_file_changed_on_both_sides_keeps_the_later_change() {
    let mut s = setup("changed-twice").await;
    let file = s.upload(s.remote, "c.txt", "original").await;
    s.sync().await;
    s.other
        .replace_content(file.id, None, || reqwest::Body::from("server change"))
        .await
        .expect("replace");
    write(
        &s.local.join("c.txt"),
        "earlier change here",
        Duration::from_secs(3600),
    );
    s.sync().await;
    assert_eq!(s.content(file.id).await, "server change");
    assert_eq!(read(&s.local.join("c.txt")), "server change");
    let versions = s.other.versions(file.id).await.expect("versions");
    let mut kept = Vec::new();
    for version in versions {
        kept.push(s.version_content(file.id, version.version).await);
    }
    assert!(kept.contains(&"earlier change here".to_owned()), "{kept:?}");

    write(&s.local.join("c.txt"), "latest change here", Duration::ZERO);
    s.sync().await;
    assert_eq!(s.content(file.id).await, "latest change here");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_subfolder_mapped_elsewhere_syncs_only_there() {
    let mut s = setup("nested").await;
    let photos = s
        .other
        .create_folder(Some(s.remote), "Photos")
        .await
        .expect("photos");
    s.upload(photos.id, "p.jpg", "pixels").await;
    let elsewhere = temp_dir("nested-photos");
    s.engine
        .handle(
            Command::Map(Mapping {
                remote: Some(photos.id),
                local: Some(elsewhere.clone()),
            }),
            &mut s.output,
        )
        .await;
    s.sync().await;
    assert_eq!(read(&elsewhere.join("p.jpg")), "pixels");
    assert!(!s.local.join("Photos").exists());
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn verify_uploads_a_change_that_kept_size_and_modification_time() {
    let mut s = setup("verify-hash").await;
    let file = s.upload(s.remote, "d.txt", "aaaa").await;
    s.sync().await;
    let path = s.local.join("d.txt");
    let mtime = fs::metadata(&path)
        .and_then(|meta| meta.modified())
        .expect("mtime");
    fs::write(&path, "bbbb").expect("write");
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(mtime))
        .expect("restore mtime");
    s.sync().await;
    assert_eq!(s.content(file.id).await, "aaaa");

    s.engine
        .handle(Command::Verify(Some(s.remote)), &mut s.output)
        .await;
    s.sync().await;

    assert_eq!(s.content(file.id).await, "bbbb");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn verify_restores_a_mirror_that_lost_a_node() {
    let mut s = setup("verify-mirror").await;
    let file = s.upload(s.remote, "e.txt", "kept").await;
    s.sync().await;
    s.engine.mirror.nodes.remove(&file.id);

    s.engine
        .handle(Command::Verify(Some(s.remote)), &mut s.output)
        .await;
    s.sync().await;

    assert_eq!(read(&s.local.join("e.txt")), "kept");
    assert_eq!(s.content(file.id).await, "kept");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_server_file_with_a_name_the_sync_ignores_is_left_alone() {
    let mut s = setup("ignored-name").await;
    let file = s.upload(s.remote, "notes.tmp", "keep me").await;
    s.sync().await;
    s.sync().await;
    assert_eq!(s.content(file.id).await, "keep me");
    assert!(s.remote_file("notes.tmp").await.is_some());
    assert!(!s.local.join("notes.tmp").exists());
}

/// An engine under the app's own rules, with `remote` as its rescue folder, and nothing mapped.
async fn app_owned(name: &str) -> Setup {
    let mine = client().await;
    let other = client().await;
    let remote = other
        .create_folder(None, &format!("sync-e2e-{name}-{}", stamp()))
        .await
        .expect("remote folder")
        .id;
    let local = temp_dir(name);
    let (output, updates) = mpsc::channel(4096);
    let (sender, commands) = tokio::sync::mpsc::unbounded_channel();
    let platform = PrivateFiles {
        rescue: Some(remote),
    };
    let engine =
        Engine::load(mine, Store::in_memory().expect("store"), Arc::new(platform)).expect("engine");
    Setup {
        engine,
        other,
        output,
        commands,
        _commands: sender,
        _updates: updates,
        remote,
        local,
    }
}

impl Setup {
    async fn pin(&mut self, node: &Node) -> PathBuf {
        let root = self.local.join(node.id.to_string());
        fs::create_dir_all(&root).expect("root");
        let known = (node.kind == NodeKind::File).then(|| node.clone().into());
        let mapping = Mapping {
            remote: Some(node.id),
            local: Some(root.clone()),
        };
        self.engine
            .handle(Command::Pin { mapping, known }, &mut self.output)
            .await;
        root
    }
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_pinned_file_syncs_both_ways_and_follows_a_rename() {
    let mut s = app_owned("pinned-file").await;
    let file = s.upload(s.remote, "a.txt", "one").await;
    let root = s.pin(&file).await;
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "one");

    s.upload(s.remote, "a.txt", "two").await;
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "two");

    write(&root.join("a.txt"), "three", Duration::ZERO);
    s.other
        .relocate(file.id, Some(s.remote), "b.txt")
        .await
        .expect("rename");
    s.sync().await;
    assert!(!root.join("a.txt").exists());
    assert_eq!(read(&root.join("b.txt")), "three");
    assert_eq!(s.content(file.id).await, "three");
    assert_eq!(s.other.list(Some(s.remote)).await.expect("list").len(), 1);
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_file_missing_from_a_pinned_folder_comes_back_and_stays_on_the_server() {
    let mut s = app_owned("restore").await;
    let folder = s
        .other
        .create_folder(Some(s.remote), "dir")
        .await
        .expect("folder");
    let file = s.upload(folder.id, "a.txt", "content").await;
    let root = s.pin(&folder).await;
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "content");

    fs::remove_file(root.join("a.txt")).expect("remove");
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "content");
    assert_eq!(s.content(file.id).await, "content");

    fs::remove_dir_all(&root).expect("remove the whole folder");
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "content");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn deleting_a_pinned_file_on_the_server_removes_the_copy() {
    let mut s = app_owned("deleted").await;
    let file = s.upload(s.remote, "a.txt", "content").await;
    let root = s.pin(&file).await;
    s.sync().await;
    assert!(root.join("a.txt").exists());

    s.other.delete(file.id).await.expect("delete");
    s.sync().await;
    assert!(!root.exists());
    assert!(s.engine.mappings.is_empty());
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn edits_to_a_pinned_file_deleted_on_the_server_go_to_the_rescue_folder() {
    let mut s = app_owned("rescued").await;
    let folder = s
        .other
        .create_folder(Some(s.remote), "dir")
        .await
        .expect("folder");
    let file = s.upload(folder.id, "a.txt", "content").await;
    let root = s.pin(&file).await;
    s.sync().await;

    write(&root.join("a.txt"), "edited", Duration::ZERO);
    s.other.delete(file.id).await.expect("delete");
    s.sync().await;

    assert!(!root.exists());
    let rescued = s.remote_file("a.txt").await.expect("rescued");
    assert_eq!(s.content(rescued.id).await, "edited");
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn purging_removes_the_folder_and_leaves_the_server_alone() {
    let mut s = app_owned("purge").await;
    let file = s.upload(s.remote, "a.txt", "content").await;
    let root = s.pin(&file).await;
    s.sync().await;

    s.engine
        .handle(Command::Purge(file.id), &mut s.output)
        .await;
    s.sync().await;

    assert!(!root.exists());
    assert_eq!(s.content(file.id).await, "content");
    assert!(s.engine.store.bases().expect("bases").is_empty());
}

#[tokio::test]
#[ignore = "needs a running server, see the module docs"]
async fn a_closed_gate_stops_transfers_and_the_upload_continues_afterwards() {
    let mut s = app_owned("gate").await;
    let file = s.upload(s.remote, "a.txt", "content").await;
    let root = s.pin(&file).await;
    let gate = super::Gate::default();
    s.engine.set_gate(gate.clone());
    gate.set_closed(true);
    s.sync().await;
    assert!(!root.join("a.txt").exists());

    gate.set_closed(false);
    s.sync().await;
    assert_eq!(read(&root.join("a.txt")), "content");
}
