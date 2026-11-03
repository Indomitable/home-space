//! Resumable chunked uploads (`/uploads`).

use protocol::{CreateUpload, UploadSession, paths};
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
use protocol::{ChunkResult, Node};

use crate::{ApiError, Client, client::expect_success};

/// Files larger than this are uploaded in chunks; smaller ones in a single request.
///
/// Matches the server's default chunk size and stays well below the 100 MB request limit of
/// proxies like Cloudflare.
pub const RESUMABLE_THRESHOLD: u64 = 32 * 1024 * 1024;

/// How often one chunk is retried before the upload gives up.
#[cfg(not(target_arch = "wasm32"))]
const CHUNK_RETRIES: u32 = 5;

impl Client {
    /// Starts a resumable upload.
    pub async fn create_upload(&self, request: &CreateUpload) -> Result<UploadSession, ApiError> {
        self.send_json(|| self.http().post(self.url(paths::UPLOADS)).json(request))
            .await
    }

    /// Where a resumable upload stands; the next chunk starts at its `offset`.
    pub async fn upload_status(&self, id: Uuid) -> Result<UploadSession, ApiError> {
        self.send_json(|| self.http().get(self.url(&paths::upload(id))))
            .await
    }

    /// Abandons a resumable upload and the bytes it has received.
    pub async fn cancel_upload(&self, id: Uuid) -> Result<(), ApiError> {
        let response = self
            .send(|| self.http().delete(self.url(&paths::upload(id))))
            .await?;
        expect_success(response).await
    }

    /// Sends the chunk of a resumable upload that starts at `offset`.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn upload_chunk(
        &self,
        id: Uuid,
        offset: u64,
        chunk: Vec<u8>,
    ) -> Result<ChunkResult, ApiError> {
        self.send_json(|| {
            self.http()
                .patch(self.url(&paths::upload(id)))
                .header(paths::UPLOAD_OFFSET, offset)
                .body(chunk.clone())
        })
        .await
    }

    /// Uploads `request.size` bytes in chunks, retrying each chunk from the server's offset
    /// after a failure, and returns the stored file.
    ///
    /// `read(offset, len)` returns the `len` bytes of the content starting at `offset`;
    /// `progress` gets the number of bytes the server has after every chunk.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn upload_resumable<R, Fut, P>(
        &self,
        request: &CreateUpload,
        read: R,
        progress: P,
    ) -> Result<Node, ApiError>
    where
        R: Fn(u64, usize) -> Fut,
        Fut: Future<Output = std::io::Result<Vec<u8>>>,
        P: Fn(u64),
    {
        let session = self.create_upload(request).await?;
        let result = self.feed(session, request.size, read, progress).await;
        if result.is_err() {
            let _ = self.cancel_upload(session.id).await;
        }
        result
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn feed<R, Fut, P>(
        &self,
        session: UploadSession,
        size: u64,
        read: R,
        progress: P,
    ) -> Result<Node, ApiError>
    where
        R: Fn(u64, usize) -> Fut,
        Fut: Future<Output = std::io::Result<Vec<u8>>>,
        P: Fn(u64),
    {
        let chunk_size = session.chunk_size.max(1);
        let mut offset = session.offset;
        let mut failures = 0;
        loop {
            let len =
                usize::try_from(chunk_size.min(size.saturating_sub(offset))).unwrap_or(usize::MAX);
            let chunk = read(offset, len)
                .await
                .map_err(|err| ApiError::Transport(format!("reading the file: {err}")))?;
            match self.upload_chunk(session.id, offset, chunk).await {
                Ok(result) => {
                    failures = 0;
                    progress(result.offset);
                    if let Some(node) = result.node {
                        return Ok(node);
                    }
                    offset = result.offset;
                }
                Err(err) if err.is_transient() && failures < CHUNK_RETRIES => {
                    failures += 1;
                    tokio::time::sleep(backoff(failures)).await;
                    if let Ok(status) = self.upload_status(session.id).await {
                        offset = status.offset;
                        progress(offset);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn backoff(failures: u32) -> std::time::Duration {
    std::time::Duration::from_millis(500 * 2u64.pow(failures.saturating_sub(1).min(5)))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use protocol::TokenPair;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, Request, Respond, ResponseTemplate,
        matchers::{method, path},
    };

    use protocol::ErrorCode;

    use super::*;

    const ID: &str = "0193a4a0-0000-7000-8000-000000000001";

    fn node_json(size: u64) -> serde_json::Value {
        json!({
            "id": "0193a4a0-0000-7000-8000-0000000000aa",
            "parent_id": null,
            "kind": "file",
            "name": "big.bin",
            "size": size,
            "sha256": null,
            "mime": "application/octet-stream",
            "version": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })
    }

    /// Accepts chunks in order, failing the second request once like a dropped connection.
    struct Chunks {
        size: u64,
        received: Arc<std::sync::Mutex<Vec<u8>>>,
        calls: AtomicUsize,
    }

    impl Respond for Chunks {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let offset: usize = request.headers["upload-offset"]
                .to_str()
                .expect("header")
                .parse()
                .expect("offset");
            let mut received = self.received.lock().expect("lock");
            if offset != received.len() {
                return ResponseTemplate::new(409)
                    .set_body_json(json!({ "error": "conflict", "message": "offset" }));
            }
            if call == 1 {
                return ResponseTemplate::new(502).set_body_string("<html>bad gateway</html>");
            }
            received.extend_from_slice(&request.body);
            let offset = received.len() as u64;
            let node = (offset == self.size).then(|| node_json(self.size));
            ResponseTemplate::new(200).set_body_json(json!({ "offset": offset, "node": node }))
        }
    }

    #[tokio::test]
    async fn resumable_upload_retries_a_failed_chunk_and_returns_the_node() {
        let server = MockServer::start().await;
        let content: Vec<u8> = (0..10u8).collect();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        Mock::given(method("POST"))
            .and(path(paths::UPLOADS))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({ "id": ID, "offset": 0, "size": 10, "chunk_size": 4 })),
            )
            .mount(&server)
            .await;
        let status_received = received.clone();
        Mock::given(method("GET"))
            .and(path(format!("{}/{ID}", paths::UPLOADS)))
            .respond_with(move |_: &Request| {
                let offset = status_received.lock().expect("lock").len();
                ResponseTemplate::new(200).set_body_json(
                    json!({ "id": ID, "offset": offset, "size": 10, "chunk_size": 4 }),
                )
            })
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(format!("{}/{ID}", paths::UPLOADS)))
            .respond_with(Chunks {
                size: 10,
                received: received.clone(),
                calls: AtomicUsize::new(0),
            })
            .mount(&server)
            .await;
        let client = Client::new(server.uri());
        client.store(TokenPair {
            access_token: "token".to_owned(),
            refresh_token: "refresh".to_owned(),
            expires_in: 900,
        });
        let request = CreateUpload {
            parent_id: None,
            name: "big.bin".to_owned(),
            size: 10,
            modified_at: None,
            mime: None,
            node_id: None,
        };
        let source = content.clone();

        let node = client
            .upload_resumable(
                &request,
                |offset, len| {
                    let start = offset as usize;
                    let chunk = source[start..start + len].to_vec();
                    async move { Ok(chunk) }
                },
                |_| (),
            )
            .await
            .expect("upload");

        assert_eq!(node.size, 10);
        assert_eq!(*received.lock().expect("lock"), content);
    }

    #[test]
    fn only_passing_failures_are_retried() {
        assert!(ApiError::is_transient(&ApiError::Transport(
            "reset".to_owned()
        )));
        assert!(ApiError::is_transient(&ApiError::Api {
            code: ErrorCode::Conflict,
            message: String::new(),
        }));
        assert!(!ApiError::is_transient(&ApiError::Api {
            code: ErrorCode::PayloadTooLarge,
            message: String::new(),
        }));
        assert!(!ApiError::is_transient(&ApiError::Unauthorized));
    }
}
