//! A pass against a fake of the server's upload API. How the server itself treats the flags is
//! tested in `server/crates/api/tests/uploads.rs`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use wiremock::{Mock, MockServer, Request, ResponseTemplate, matchers::any};

use super::run::TARGET_GONE;
use super::*;
use crate::{CoreClient, DeviceIdentity, SessionListener};

const TARGET: &str = "0193a4a0-0000-7000-8000-00000000000a";
const UPLOAD: &str = "0193a4a0-0000-7000-8000-0000000000bb";
const CHUNK: usize = 4;

/// One upload the fake was asked to start.
#[derive(Clone, Debug)]
struct Started {
    body: Value,
    received: Vec<u8>,
    cancelled: bool,
}

#[derive(Default)]
struct Fake {
    uploads: Vec<Started>,
    /// Folders made, as (parent, name).
    made: Vec<(Option<String>, String)>,
    /// The name to give the next stored file, as `keep_both` might.
    rename_to: Option<String>,
    target_gone: bool,
    /// Answer everything with this, like a gateway whose server is down.
    down: bool,
    requests: usize,
}

fn node(id: &str, name: &str, kind: &str) -> Value {
    json!({
        "id": id, "parent_id": null, "kind": kind, "name": name, "size": 1, "sha256": null,
        "mime": "application/octet-stream", "version": 1,
        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
    })
}

fn not_found() -> ResponseTemplate {
    ResponseTemplate::new(404).set_body_json(json!({ "error": "not_found", "message": "gone" }))
}

impl Fake {
    fn respond(&mut self, request: &Request) -> ResponseTemplate {
        let path = request.url.path().to_owned();
        let method = request.method.as_str().to_owned();
        if path.ends_with(protocol::paths::LOGIN) {
            return ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access", "refresh_token": "refresh", "expires_in": 900,
            }));
        }
        if path.ends_with(protocol::paths::ME) {
            return ResponseTemplate::new(200).set_body_json(json!({
                "id": "00000000-0000-0000-0000-000000000001", "username": "me",
                "lost_and_found_id": "00000000-0000-0000-0000-000000000002",
            }));
        }
        self.requests += 1;
        if self.down {
            return ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>");
        }
        if path.ends_with(&format!("/nodes/{TARGET}")) {
            return if self.target_gone {
                not_found()
            } else {
                ResponseTemplate::new(200).set_body_json(node(TARGET, "Photos", "folder"))
            };
        }
        if path.ends_with("/folders") {
            let body: Value = serde_json::from_slice(&request.body).expect("body");
            let name = body["name"].as_str().expect("name").to_owned();
            let parent = body["parent_id"].as_str().map(str::to_owned);
            self.made.push((parent, name.clone()));
            let id = format!("0193a4a0-0000-7000-8000-{:012x}", 0x100 + self.made.len());
            return ResponseTemplate::new(201).set_body_json(node(&id, &name, "folder"));
        }
        if path.ends_with("/uploads") && method == "POST" {
            let body: Value = serde_json::from_slice(&request.body).expect("body");
            let size = body["size"].as_u64().expect("size");
            self.uploads.push(Started {
                body,
                received: Vec::new(),
                cancelled: false,
            });
            return ResponseTemplate::new(201).set_body_json(json!({
                "id": self.upload_id(self.uploads.len() - 1), "offset": 0, "size": size,
                "chunk_size": CHUNK,
            }));
        }
        if let Some(index) = self.upload_index(&path) {
            return self.on_upload(index, &method, request);
        }
        not_found()
    }

    fn upload_id(&self, index: usize) -> String {
        format!("{}{:x}", &UPLOAD[..UPLOAD.len() - 1], index)
    }

    fn upload_index(&self, path: &str) -> Option<usize> {
        (0..self.uploads.len()).find(|index| path.ends_with(&self.upload_id(*index)))
    }

    fn on_upload(&mut self, index: usize, method: &str, request: &Request) -> ResponseTemplate {
        let id = self.upload_id(index);
        let rename = self.rename_to.clone();
        let upload = &mut self.uploads[index];
        let size = upload.body["size"].as_u64().expect("size");
        match method {
            "GET" => ResponseTemplate::new(200).set_body_json(json!({
                "id": id, "offset": upload.received.len(), "size": size, "chunk_size": CHUNK,
            })),
            "DELETE" => {
                upload.cancelled = true;
                ResponseTemplate::new(204)
            }
            _ => {
                upload.received.extend_from_slice(&request.body);
                let offset = upload.received.len() as u64;
                let name = rename
                    .unwrap_or_else(|| upload.body["name"].as_str().expect("name").to_owned());
                let stored = (offset == size)
                    .then(|| node("0193a4a0-0000-7000-8000-0000000000cc", &name, "file"));
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "offset": offset, "node": stored }))
            }
        }
    }
}

type ReadHook = Box<dyn Fn(usize, &Sources) + Send + Sync>;

#[derive(Default)]
struct Sources {
    files: Mutex<HashMap<String, (LocalItem, Vec<u8>)>>,
    reads: AtomicUsize,
    /// Called before each read with how many came before it.
    on_read: Mutex<Option<ReadHook>>,
    list_error: Mutex<Option<String>>,
}

impl Sources {
    fn put(&self, key: &str, dir: &str, content: &[u8], age: Duration) {
        let item = LocalItem {
            key: key.to_owned(),
            relative_dir: dir.to_owned(),
            name: format!("{key}.jpg"),
            size: content.len() as u64,
            modified_at: SystemTime::now() - age,
            mime: Some("image/jpeg".to_owned()),
        };
        self.files
            .lock()
            .unwrap()
            .insert(key.to_owned(), (item, content.to_vec()));
    }
}

const OLD: Duration = Duration::from_secs(60);

impl LocalSources for Sources {
    fn list(&self, _: SourceKind, _: String) -> Result<Vec<LocalItem>, CoreError> {
        if let Some(detail) = self.list_error.lock().unwrap().clone() {
            return Err(CoreError::Failed {
                code: "permission".to_owned(),
                detail,
            });
        }
        let mut items: Vec<LocalItem> = self
            .files
            .lock()
            .unwrap()
            .values()
            .map(|(item, _)| item.clone())
            .collect();
        items.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(items)
    }

    fn stat(&self, _: SourceKind, _: String, key: String) -> Result<Option<LocalItem>, CoreError> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .get(&key)
            .map(|(item, _)| item.clone()))
    }

    fn read(
        &self,
        _: SourceKind,
        _: String,
        key: String,
        offset: u64,
        len: u64,
    ) -> Result<Vec<u8>, CoreError> {
        let count = self.reads.fetch_add(1, Ordering::SeqCst);
        if let Some(hook) = self.on_read.lock().unwrap().as_ref() {
            hook(count, self);
        }
        let files = self.files.lock().unwrap();
        let (_, content) = files.get(&key).expect("file");
        let start = (offset as usize).min(content.len());
        let end = (start + len as usize).min(content.len());
        Ok(content[start..end].to_vec())
    }
}

struct Quiet;

impl AutoUploadListener for Quiet {
    fn changed(&self) {}
}

impl SessionListener for Quiet {
    fn refresh_token_changed(&self, _: Option<String>) {}
}

struct World {
    _server: MockServer,
    fake: Arc<Mutex<Fake>>,
    sources: Arc<Sources>,
    auto: Arc<AutoUpload>,
    _client: Arc<CoreClient>,
}

async fn world() -> World {
    let server = MockServer::start().await;
    let fake = Arc::new(Mutex::new(Fake::default()));
    let responder = fake.clone();
    Mock::given(any())
        .respond_with(move |request: &Request| responder.lock().unwrap().respond(request))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("dir").keep();
    let device = DeviceIdentity {
        id: "0193a4a0-0000-7000-8000-0000000000dd".to_owned(),
        name: "Pixel".to_owned(),
    };
    let client = Arc::new(
        CoreClient::with_session(
            server.uri(),
            device,
            Arc::new(Quiet),
            dir.join("files").to_string_lossy().into_owned(),
        )
        .expect("client"),
    );
    client
        .login("me".to_owned(), "pw".to_owned())
        .await
        .expect("login");
    let sources = Arc::new(Sources::default());
    let auto = AutoUpload::open(
        client.clone(),
        dir.join("sync").to_string_lossy().into_owned(),
        sources.clone(),
        Arc::new(Quiet),
    )
    .expect("open");
    fake.lock().unwrap().requests = 0;
    World {
        _server: server,
        fake,
        sources,
        auto,
        _client: client,
    }
}

impl World {
    fn add(&self, upload_existing: bool) -> UploadSource {
        self.auto
            .add_source(
                SourceKind::Folder,
                "handle".to_owned(),
                "Camera".to_owned(),
                Some(TARGET.to_owned()),
                "Photos".to_owned(),
                upload_existing,
            )
            .expect("add")
    }

    fn uploads(&self) -> Vec<Started> {
        self.fake.lock().unwrap().uploads.clone()
    }

    fn source(&self) -> UploadSource {
        self.auto.sources().expect("sources").remove(0)
    }
}

#[tokio::test]
async fn settings_default_to_wifi_only_and_no_background() {
    let w = world().await;
    assert_eq!(
        w.auto.settings().unwrap(),
        SyncSettings {
            wifi_only: true,
            background: false,
        }
    );
    let changed = SyncSettings {
        wifi_only: false,
        background: true,
    };
    w.auto.set_settings(changed).unwrap();
    assert_eq!(w.auto.settings().unwrap(), changed);
}

#[tokio::test]
async fn a_new_file_goes_up_as_an_auto_upload_that_keeps_both() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"hello world", OLD);

    let summary = w.auto.run_pass(false).await.expect("pass");

    assert_eq!(summary.uploaded, 1);
    let uploads = w.uploads();
    assert_eq!(uploads[0].received, b"hello world");
    let body = &uploads[0].body;
    assert_eq!(body["name"], "a.jpg");
    assert_eq!(body["parent_id"], TARGET);
    assert_eq!(body["keep_both"], true);
    assert_eq!(body["auto_upload"], true);
    assert_eq!(body["mime"], "image/jpeg");
    assert!(body["modified_at"].is_string());
    assert!(w.source().last_synced_at.is_some());
}

#[tokio::test]
async fn an_unchanged_file_is_not_sent_again() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"hello", OLD);
    w.auto.run_pass(false).await.expect("first");

    let again = w.auto.run_pass(false).await.expect("second");

    assert_eq!(again.uploaded, 0);
    assert_eq!(w.uploads().len(), 1);
}

#[tokio::test]
async fn an_edit_goes_to_the_name_the_server_gave_and_may_become_a_version() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"first", OLD);
    w.fake.lock().unwrap().rename_to = Some("a (1).jpg".to_owned());
    w.auto.run_pass(false).await.expect("first");

    w.sources
        .put("a", "", b"edited", OLD - Duration::from_secs(1));
    let summary = w.auto.run_pass(false).await.expect("second");

    assert_eq!(summary.uploaded, 1);
    let edit = &w.uploads()[1];
    assert_eq!(edit.body["name"], "a (1).jpg");
    assert_eq!(edit.received, b"edited");
    // Absent means false: a taken name makes a new version, and a free one a new file.
    assert_eq!(edit.body.get("keep_both"), None);
    assert_eq!(edit.body["auto_upload"], true);
}

#[tokio::test]
async fn existing_files_can_be_left_out_until_they_change() {
    let w = world().await;
    w.add(false);
    w.sources.put("old", "", b"old", OLD);

    let first = w.auto.run_pass(false).await.expect("first");
    assert_eq!(first.uploaded, 0);
    assert!(w.uploads().is_empty());

    w.sources.put("new", "", b"new", OLD);
    w.sources
        .put("old", "", b"edited", OLD - Duration::from_secs(1));
    let second = w.auto.run_pass(false).await.expect("second");
    assert_eq!(second.uploaded, 2);
    let names: Vec<_> = w.uploads().iter().map(|u| u.body["name"].clone()).collect();
    assert!(names.contains(&json!("new.jpg")));
    assert!(names.contains(&json!("old.jpg")));
}

#[tokio::test]
async fn subfolders_are_made_once_below_the_target() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "2026/05", b"aa", OLD);
    w.sources.put("b", "2026/05", b"bb", OLD);

    w.auto.run_pass(false).await.expect("pass");

    let made = w.fake.lock().unwrap().made.clone();
    assert_eq!(made.len(), 2, "{made:?}");
    assert_eq!(made[0], (Some(TARGET.to_owned()), "2026".to_owned()));
    assert_eq!(made[1].1, "05");
    assert!(w.uploads()[0].body["parent_id"] != TARGET);
}

#[tokio::test]
async fn stopping_takes_effect_between_chunks_and_the_next_pass_continues() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"0123456789", OLD);
    let auto = w.auto.clone();
    *w.sources.on_read.lock().unwrap() = Some(Box::new(move |count, _| {
        if count == 0 {
            auto.stop();
        }
    }));

    // The read that was under way when told to stop still goes; the next one doesn't.
    let stopped = w.auto.run_pass(false).await.expect("stopped");
    assert!(stopped.stopped);
    assert_eq!(stopped.uploaded, 0);
    assert_eq!(w.uploads()[0].received, b"0123");

    *w.sources.on_read.lock().unwrap() = None;
    let resumed = w.auto.run_pass(false).await.expect("resumed");
    assert_eq!(resumed.uploaded, 1);
    let uploads = w.uploads();
    assert_eq!(uploads.len(), 1, "it continued instead of starting over");
    assert_eq!(uploads[0].received, b"0123456789");
}

#[tokio::test]
async fn a_file_changed_during_its_upload_is_not_stored() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"0123456789", OLD);
    *w.sources.on_read.lock().unwrap() = Some(Box::new(|count, sources| {
        if count == 0 {
            sources.put("a", "", b"different!", OLD - Duration::from_secs(1));
        }
    }));

    let summary = w.auto.run_pass(false).await.expect("pass");

    assert_eq!(summary.uploaded, 0);
    assert_eq!(summary.failed, 1);
    assert!(w.uploads()[0].cancelled);
    assert_eq!(w.source().pending, 1);
    assert_eq!(w.source().last_synced_at, None);
}

#[tokio::test]
async fn a_deleted_target_pauses_the_source_until_another_is_chosen() {
    let w = world().await;
    let source = w.add(true);
    w.sources.put("a", "", b"data", OLD);
    w.fake.lock().unwrap().target_gone = true;

    let summary = w.auto.run_pass(false).await.expect("pass");

    assert_eq!(summary.uploaded, 0);
    assert!(w.uploads().is_empty());
    assert_eq!(w.source().problem.as_deref(), Some(TARGET_GONE));
    assert!(
        w.fake.lock().unwrap().made.is_empty(),
        "not silently made again"
    );

    w.auto
        .set_source_target(source.id, None, "Home".to_owned())
        .expect("target");
    assert_eq!(w.source().problem, None);
    let after = w.auto.run_pass(false).await.expect("after");
    assert_eq!(after.uploaded, 1);
    assert_eq!(w.uploads()[0].body.get("parent_id"), None);
}

#[tokio::test]
async fn a_source_that_cant_be_read_reports_it_and_recovers() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"data", OLD);
    *w.sources.list_error.lock().unwrap() = Some("permission revoked".to_owned());

    w.auto.run_pass(false).await.expect("pass");
    assert_eq!(w.source().problem.as_deref(), Some("permission revoked"));

    *w.sources.list_error.lock().unwrap() = None;
    let after = w.auto.run_pass(false).await.expect("after");
    assert_eq!(after.uploaded, 1);
    assert_eq!(w.source().problem, None);
}

#[tokio::test]
async fn an_unreachable_server_is_one_outage_that_waits() {
    let w = world().await;
    w.add(true);
    w.sources.put("a", "", b"data", OLD);
    w.fake.lock().unwrap().down = true;

    let summary = w.auto.run_pass(false).await.expect("pass");
    assert!(summary.outage);
    assert_eq!(summary.failed, 0, "not reported per file");

    let before = w.fake.lock().unwrap().requests;
    let waiting = w.auto.run_pass(false).await.expect("waiting");
    assert!(waiting.outage);
    assert_eq!(w.fake.lock().unwrap().requests, before, "no request sent");

    w.fake.lock().unwrap().down = false;
    let retried = w.auto.run_pass(true).await.expect("retried");
    assert_eq!(retried.uploaded, 1);
}

#[tokio::test]
async fn removing_a_source_cancels_its_upload_and_wipe_forgets_everything() {
    let w = world().await;
    let source = w.add(true);
    w.sources.put("a", "", b"0123456789", OLD);
    let auto = w.auto.clone();
    *w.sources.on_read.lock().unwrap() = Some(Box::new(move |count, _| {
        if count == 0 {
            auto.stop();
        }
    }));
    w.auto.run_pass(false).await.expect("stopped");

    w.auto.remove_source(source.id).await.expect("remove");
    assert!(w.uploads()[0].cancelled);
    assert!(w.auto.sources().unwrap().is_empty());

    w.add(true);
    w.auto.wipe().await.expect("wipe");
    assert!(w.auto.sources().unwrap().is_empty());
    assert!(w.auto.settings().unwrap().wifi_only);
}
