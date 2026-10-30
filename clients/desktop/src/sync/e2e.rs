//! The sync engine against a real server; ignored unless asked for:
//!
//! ```sh
//! HOME_SPACE_E2E_SERVER=http://127.0.0.1:8080 HOME_SPACE_E2E_USER=alice \
//! HOME_SPACE_E2E_PASSWORD=... cargo test -p desktop e2e -- --ignored --test-threads 1
//! ```
//!
//! Each run works in a fresh root folder of that account. A second client stands for the
//! other user (another device, or the web client) in the conflict scenarios.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use iced::futures::channel::mpsc;

use api_client::Client;
use protocol::{Node, NodeId, NodeKind};

use super::{Command, Engine, Mapping, Update, store::Store};

const PASSES: usize = 4;

struct Setup {
    engine: Engine,
    other: Client,
    output: mpsc::Sender<Update>,
    _updates: mpsc::Receiver<Update>,
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
    let mut engine = Engine::load(mine, Store::in_memory().expect("store")).expect("engine");
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
        _updates: updates,
        remote,
        local,
    }
}

impl Setup {
    /// Runs passes until the engine has nothing left to do.
    async fn sync(&mut self) {
        for _ in 0..PASSES {
            match self.engine.pass(&mut self.output).await {
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
