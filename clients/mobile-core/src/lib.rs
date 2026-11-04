//! The Rust client exported to the mobile apps through UniFFI.
//!
//! The types here are the FFI surface: ids are strings and times are `SystemTime`, so the
//! generated Kotlin and Swift need no custom converters. Everything behind them (routes,
//! JSON, sessions, errors, sorting and formatting) is the same code the desktop and web
//! clients run.

use std::time::SystemTime;

use client_core::{format, listing};
use protocol::{NodeId, NodeKind};

uniffi::setup_scaffolding!();

/// Failure of a call, as the apps see it.
///
/// No field may be called `message`: Kotlin exceptions already have one, and the generated
/// class would not compile.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    #[error("not signed in")]
    Unauthorized,
    /// The server could not be reached, or a gateway answered that it is down.
    #[error("{detail}")]
    Unreachable { detail: String },
    /// Any other failure; `code` is short and stable, such as `conflict` or `502`.
    #[error("{detail}")]
    Failed { code: String, detail: String },
    #[error("invalid id: {id}")]
    InvalidId { id: String },
}

impl From<api_client::ApiError> for CoreError {
    fn from(err: api_client::ApiError) -> Self {
        if err == api_client::ApiError::Unauthorized {
            Self::Unauthorized
        } else if err.is_unreachable() {
            Self::Unreachable {
                detail: err.to_string(),
            }
        } else {
            Self::Failed {
                code: err.code(),
                detail: err.to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ServerHealth {
    pub status: String,
    pub is_ok: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct SignedInUser {
    pub id: String,
    pub username: String,
    pub lost_and_found_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum EntryKind {
    Folder,
    File,
}

/// A file or folder, with the labels the desktop and web clients show for it.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Entry {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: EntryKind,
    pub name: String,
    pub size: i64,
    pub mime: String,
    pub version: i32,
    pub updated_at: SystemTime,
    /// Empty for folders, otherwise e.g. `1.5 MB`.
    pub size_label: String,
    /// `YYYY-MM-DD HH:MM` in UTC.
    pub updated_label: String,
}

impl From<protocol::Node> for Entry {
    fn from(node: protocol::Node) -> Self {
        let kind = match node.kind {
            NodeKind::Folder => EntryKind::Folder,
            NodeKind::File => EntryKind::File,
        };
        Self {
            id: node.id.to_string(),
            parent_id: node.parent_id.map(|id| id.to_string()),
            size_label: match kind {
                EntryKind::Folder => String::new(),
                EntryKind::File => format::human_size(node.size),
            },
            updated_label: format::short_date(node.updated_at),
            updated_at: node.updated_at.into(),
            kind,
            name: node.name,
            size: node.size,
            mime: node.mime,
            version: node.version,
        }
    }
}

/// Client for one server; the apps keep one per signed-in server.
#[derive(uniffi::Object)]
pub struct CoreClient {
    inner: api_client::Client,
}

// reqwest needs a Tokio reactor, so every async call runs on UniFFI's Tokio runtime.
#[uniffi::export(async_runtime = "tokio")]
impl CoreClient {
    /// A signed-out client for the server at `base_url`, e.g. `http://10.0.2.2:8080`.
    #[uniffi::constructor]
    pub fn new(base_url: String) -> Self {
        Self {
            inner: api_client::Client::new(base_url.trim_end_matches('/')),
        }
    }

    pub fn base_url(&self) -> String {
        self.inner.base_url().to_owned()
    }

    /// Whether the server and its database are up.
    pub async fn health(&self) -> Result<ServerHealth, CoreError> {
        let health = self.inner.health().await?;
        Ok(ServerHealth {
            is_ok: health.is_ok(),
            status: health.status,
        })
    }

    /// Signs in with a password; the session stays in this client.
    pub async fn login(
        &self,
        username: String,
        password: String,
    ) -> Result<SignedInUser, CoreError> {
        let me = self.inner.login(&username, &password).await?;
        Ok(SignedInUser {
            id: me.id.to_string(),
            username: me.username,
            lost_and_found_id: me.lost_and_found_id.to_string(),
        })
    }

    /// Lists a folder, or Home when `parent_id` is `None`, folders first then by name.
    pub async fn list(&self, parent_id: Option<String>) -> Result<Vec<Entry>, CoreError> {
        let parent = parent_id.as_deref().map(parse_id).transpose()?;
        let mut nodes = self.inner.list(parent).await?;
        listing::sort_nodes(&mut nodes, listing::Sort::default());
        Ok(nodes.into_iter().map(Entry::from).collect())
    }

    pub async fn logout(&self) -> Result<(), CoreError> {
        Ok(self.inner.logout().await?)
    }
}

fn parse_id(id: &str) -> Result<NodeId, CoreError> {
    id.parse()
        .map_err(|_| CoreError::InvalidId { id: id.to_owned() })
}

/// Formats a byte count the way the desktop and web clients do, e.g. `1.5 MB`.
#[uniffi::export]
pub fn human_size(bytes: i64) -> String {
    format::human_size(bytes)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn node(name: &str, kind: &str, size: i64) -> serde_json::Value {
        serde_json::json!({
            "id": "00000000-0000-0000-0000-00000000000".to_owned() + &name.len().to_string(),
            "parent_id": null,
            "kind": kind,
            "name": name,
            "size": size,
            "sha256": null,
            "mime": "application/octet-stream",
            "version": 1,
            "created_at": "2026-11-01T10:00:00Z",
            "updated_at": "2026-11-02T11:30:00Z",
        })
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(protocol::paths::HEALTH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&server)
            .await;

        let health = CoreClient::new(server.uri()).health().await.unwrap();
        assert!(health.is_ok);
    }

    #[tokio::test]
    async fn list_sorts_folders_first_and_labels_sizes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(protocol::paths::NODES))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                node("b.txt", "file", 1536),
                node("Photos", "folder", 0),
                node("a.txt", "file", 10),
            ])))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGIN))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access", "refresh_token": "refresh", "expires_in": 900,
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(protocol::paths::ME))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "username": "me",
                "lost_and_found_id": "00000000-0000-0000-0000-000000000002",
            })))
            .mount(&server)
            .await;

        let client = CoreClient::new(server.uri());
        let user = client.login("me".into(), "secret".into()).await.unwrap();
        assert_eq!(user.username, "me");
        let entries = client.list(None).await.unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Photos", "a.txt", "b.txt"]);
        assert_eq!(entries[0].size_label, "");
        assert_eq!(entries[2].size_label, "1.5 KB");
        assert_eq!(entries[2].updated_label, "2026-11-02 11:30");
    }

    #[tokio::test]
    async fn unreachable_server_is_its_own_error() {
        let err = CoreClient::new("http://127.0.0.1:9".into())
            .health()
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Unreachable { .. }), "{err:?}");
    }

    #[tokio::test]
    async fn malformed_id_is_rejected_before_any_request() {
        let err = CoreClient::new("http://127.0.0.1:9".into())
            .list(Some("nope".into()))
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::InvalidId { .. }));
    }
}
