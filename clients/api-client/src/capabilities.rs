//! What the server supports (`/capabilities`), read once a session starts.

use protocol::{Capabilities, Me, paths};

use crate::{ApiError, Client};

/// The upload request size used when the server's capabilities could not be read. It stays
/// below nginx's default body limit of 1 MiB, so uploads still get through a proxy nobody
/// configured for them.
pub const FALLBACK_UPLOAD_CHUNK_BYTES: u64 = 512 * 1024;

impl Client {
    /// Reads what the server supports.
    pub async fn server_capabilities(&self) -> Result<Capabilities, ApiError> {
        self.send_json(|| self.http().get(self.url(paths::CAPABILITIES)))
            .await
    }

    /// The largest request body a file upload sends: files up to this size go up in one
    /// request, larger ones in chunks of at most this size. It is the server's
    /// [`Capabilities::upload_chunk_bytes`], or [`FALLBACK_UPLOAD_CHUNK_BYTES`] when the server
    /// did not report one.
    pub fn upload_chunk_bytes(&self) -> u64 {
        self.capabilities()
            .map_or(FALLBACK_UPLOAD_CHUNK_BYTES, |capabilities| {
                capabilities.upload_chunk_bytes
            })
            .max(1)
    }

    /// Finishes a sign-in: reads the server's capabilities, then returns the user. A server
    /// that cannot report them leaves the client on the fallbacks instead of failing the
    /// sign-in.
    pub(crate) async fn signed_in(&self) -> Result<Me, ApiError> {
        let capabilities = self.server_capabilities().await.ok();
        self.set_capabilities(capabilities);
        self.me().await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    async fn server_signing_in() -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(paths::LOGIN))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "token", "refresh_token": "refresh", "expires_in": 900
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(paths::ME))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "username": "alice",
                "lost_and_found_id": "00000000-0000-0000-0000-0000000000f0"
            })))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn signing_in_reads_the_servers_upload_chunk_size() {
        let server = server_signing_in().await;
        Mock::given(method("GET"))
            .and(path(paths::CAPABILITIES))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "upload_chunk_bytes": 4096 })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::new(server.uri());

        client.login("alice", "pw").await.expect("login");

        assert_eq!(client.upload_chunk_bytes(), 4096);
    }

    #[tokio::test]
    async fn a_server_without_capabilities_leaves_the_fallback() {
        let server = server_signing_in().await;
        let client = Client::new(server.uri());

        client.login("alice", "pw").await.expect("login");

        assert_eq!(client.upload_chunk_bytes(), FALLBACK_UPLOAD_CHUNK_BYTES);
    }

    #[test]
    fn a_new_client_uses_the_fallback() {
        assert_eq!(
            Client::new("http://localhost").upload_chunk_bytes(),
            FALLBACK_UPLOAD_CHUNK_BYTES
        );
    }
}
