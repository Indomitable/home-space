//! The Rust client exported to the mobile apps through UniFFI.
//!
//! The types here are the FFI surface: ids are strings and times are `SystemTime`, so the
//! generated Kotlin and Swift need no custom converters. Everything behind them (routes,
//! JSON, sessions, errors, sorting and formatting) is the same code the desktop and web
//! clients run.

use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use api_client::{ApiError, Pkce, VERIFIER_BYTES};
use client_core::{format, icon, listing};
use protocol::{DeviceInfo, NodeId, NodeKind, paths};

#[cfg(target_os = "android")]
mod android;

uniffi::setup_scaffolding!();

/// Where every async call runs. UniFFI polls a future on the caller's thread, which in the apps
/// is the UI thread, so without this the TLS handshake would run there too. On Android that
/// handshake blocks on network calls for revocation checks, which throws
/// `NetworkOnMainThreadException`.
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("start the async runtime")
});

/// Runs `work` on [`RUNTIME`]. It finishes even if the app stops waiting, so cancelling a call
/// can't lose a refresh token the server has already rotated.
async fn off_ui_thread<T: Send + 'static>(work: impl Future<Output = T> + Send + 'static) -> T {
    match RUNTIME.spawn(work).await {
        Ok(value) => value,
        Err(err) => std::panic::resume_unwind(err.into_panic()),
    }
}

/// The server's limit on a device name.
const MAX_DEVICE_NAME_CHARS: usize = 100;
#[cfg(target_os = "ios")]
const PLATFORM: &str = protocol::PLATFORM_IOS;
#[cfg(not(target_os = "ios"))]
const PLATFORM: &str = protocol::PLATFORM_ANDROID;

/// The end of an error's chain, which names what went wrong: from `request failed: error sending
/// request for url (...): client error (Connect): invalid peer certificate: Expired`, `Expired`.
fn last_cause(message: &str) -> &str {
    message.rsplit(": ").next().unwrap_or(message)
}

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
    /// A password sign-in was refused.
    #[error("invalid username or password")]
    InvalidCredentials,
    /// The server refused the login code of a single sign-on; it may have timed out.
    #[error("the sign-in expired")]
    SsoExpired,
    /// The identity provider refused the user, or the user is not in the required group.
    #[error("not allowed to sign in with this account")]
    SsoDenied,
    /// A single sign-on ended in any other failure.
    #[error("signing in failed")]
    SsoFailed,
    /// What was typed as a server address is not an http or https URL.
    #[error("the server address must start with http:// or https://")]
    InvalidServerAddress,
}

impl From<api_client::ApiError> for CoreError {
    fn from(err: api_client::ApiError) -> Self {
        if err == api_client::ApiError::Unauthorized {
            Self::Unauthorized
        } else if err.is_unreachable() {
            Self::Unreachable {
                detail: last_cause(&err.to_string()).to_owned(),
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

/// How a server lets users sign in; at least one of the two is on.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SignInOptions {
    pub password: bool,
    /// Name of the identity provider for the sign-in button, when single sign-on is on.
    pub sso_name: Option<String>,
}

/// A started single sign-on: open `url` in a browser and keep `verifier` for
/// [`CoreClient::finish_sso`].
#[derive(Debug, Clone, uniffi::Record)]
pub struct SsoRequest {
    pub url: String,
    pub verifier: String,
}

/// This install of the app, as the server's device list shows it.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DeviceIdentity {
    /// A UUID the app generates once and keeps.
    pub id: String,
    /// Such as the phone's model.
    pub name: String,
}

/// Told about the refresh token, which the app keeps to resume the session on its next start.
///
/// Called on the thread that made the request and awaited by it. Refresh tokens are single use
/// and replaying an old one ends the session, so save the token before returning. It must not
/// throw.
#[uniffi::export(with_foreign)]
pub trait SessionListener: Send + Sync {
    /// `token` is the one to save now, or `None` when the session ended.
    fn refresh_token_changed(&self, token: Option<String>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum EntryKind {
    Folder,
    File,
}

/// Which icon to show for an entry; the apps bundle one asset for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum EntryIcon {
    Folder,
    Image,
    Document,
    Other,
}

/// A file or folder, with the labels the desktop and web clients show for it.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Entry {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: EntryKind,
    pub icon: EntryIcon,
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
        let entry_icon = match icon::node_icon(&node) {
            icon::NodeIcon::Folder => EntryIcon::Folder,
            icon::NodeIcon::Image => EntryIcon::Image,
            icon::NodeIcon::Document => EntryIcon::Document,
            icon::NodeIcon::Other => EntryIcon::Other,
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
            icon: entry_icon,
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
    /// The signed-in user's Lost & Found, which Home leaves out while it is empty.
    lost_and_found: Mutex<Option<NodeId>>,
}

#[uniffi::export]
impl CoreClient {
    /// A signed-out client for the server at `base_url`, e.g. `http://10.0.2.2:8080`.
    #[uniffi::constructor]
    pub fn new(base_url: String) -> Self {
        Self::wrapping(api_client::Client::new(base_url.trim_end_matches('/')))
    }

    /// A signed-out client for `base_url` that signs in as `device` and reports its refresh
    /// token to `listener`.
    #[uniffi::constructor]
    pub fn with_session(
        base_url: String,
        device: DeviceIdentity,
        listener: Arc<dyn SessionListener>,
    ) -> Result<Self, CoreError> {
        let device = device_info(device)?;
        let inner = api_client::Client::with_listener(
            base_url.trim_end_matches('/'),
            Arc::new(move |token: Option<&str>| {
                listener.refresh_token_changed(token.map(str::to_owned));
            }),
        )
        .with_device(device);
        Ok(Self::wrapping(inner))
    }

    pub fn base_url(&self) -> String {
        self.inner.base_url().to_owned()
    }

    /// How this server lets users sign in; needs no session.
    pub async fn providers(&self) -> Result<SignInOptions, CoreError> {
        let inner = self.inner.clone();
        let providers = off_ui_thread(async move { inner.providers().await }).await?;
        Ok(SignInOptions {
            password: providers.password,
            sso_name: providers.oidc.map(|oidc| oidc.name),
        })
    }

    /// Whether the server and its database are up.
    pub async fn health(&self) -> Result<ServerHealth, CoreError> {
        let inner = self.inner.clone();
        let health = off_ui_thread(async move { inner.health().await }).await?;
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
        let inner = self.inner.clone();
        let me = off_ui_thread(async move { inner.login(&username, &password).await })
            .await
            .map_err(|err| match err {
                ApiError::Unauthorized => CoreError::InvalidCredentials,
                other => other.into(),
            })?;
        Ok(self.signed_in(me))
    }

    /// Starts a single sign-on. The browser comes back to [`paths::MOBILE_OIDC_REDIRECT`].
    pub fn begin_sso(&self) -> Result<SsoRequest, CoreError> {
        let mut bytes = [0u8; VERIFIER_BYTES];
        rand::fill(&mut bytes);
        let pkce = Pkce::from_random(bytes);
        let url = self
            .inner
            .oidc_start_url(paths::MOBILE_OIDC_REDIRECT, &pkce)?;
        Ok(SsoRequest {
            url,
            verifier: pkce.verifier().to_owned(),
        })
    }

    /// Completes a single sign-on from the URL the browser came back to.
    pub async fn finish_sso(
        &self,
        callback_url: String,
        verifier: String,
    ) -> Result<SignedInUser, CoreError> {
        let code = login_code(&callback_url)?;
        let inner = self.inner.clone();
        let pkce = Pkce::from_verifier(verifier);
        let me = off_ui_thread(async move { inner.oidc_exchange(&code, &pkce).await })
            .await
            .map_err(|err| match err {
                ApiError::Unauthorized => CoreError::SsoExpired,
                other => other.into(),
            })?;
        Ok(self.signed_in(me))
    }

    /// Resumes a session from a saved refresh token. `Unauthorized` means that session is gone;
    /// after any other failure the token is still good to retry with. Call it once per token:
    /// it rotates the token, and replaying the old one ends the session.
    pub async fn restore(&self, refresh_token: String) -> Result<SignedInUser, CoreError> {
        let inner = self.inner.clone();
        let me = off_ui_thread(async move { inner.restore(&refresh_token).await }).await?;
        Ok(self.signed_in(me))
    }

    /// Lists a folder, or Home when `parent_id` is `None`, folders first then by name. Home
    /// leaves out Lost & Found while it is empty.
    pub async fn list(&self, parent_id: Option<String>) -> Result<Vec<Entry>, CoreError> {
        let parent = parent_id.as_deref().map(parse_id).transpose()?;
        let inner = self.inner.clone();
        let lost_and_found = *self.lost_and_found.lock().expect("lost and found lock");
        let mut nodes = off_ui_thread(async move {
            match (parent, lost_and_found) {
                (None, Some(found)) => inner.list_home(found).await,
                _ => inner.list(parent).await,
            }
        })
        .await?;
        listing::sort_nodes(&mut nodes, listing::Sort::default());
        Ok(nodes.into_iter().map(Entry::from).collect())
    }

    pub async fn logout(&self) -> Result<(), CoreError> {
        let inner = self.inner.clone();
        Ok(off_ui_thread(async move { inner.logout().await }).await?)
    }
}

impl CoreClient {
    fn wrapping(inner: api_client::Client) -> Self {
        Self {
            inner,
            lost_and_found: Mutex::new(None),
        }
    }

    fn signed_in(&self, me: protocol::Me) -> SignedInUser {
        *self.lost_and_found.lock().expect("lost and found lock") = Some(me.lost_and_found_id);
        me.into()
    }
}

impl From<protocol::Me> for SignedInUser {
    fn from(me: protocol::Me) -> Self {
        Self {
            id: me.id.to_string(),
            username: me.username,
            lost_and_found_id: me.lost_and_found_id.to_string(),
        }
    }
}

fn device_info(device: DeviceIdentity) -> Result<DeviceInfo, CoreError> {
    let id = device.id.parse().map_err(|_| CoreError::InvalidId {
        id: device.id.clone(),
    })?;
    let name: String = device
        .name
        .trim()
        .chars()
        .take(MAX_DEVICE_NAME_CHARS)
        .collect();
    Ok(DeviceInfo {
        id,
        name: if name.is_empty() {
            PLATFORM.to_owned()
        } else {
            name
        },
        platform: PLATFORM.to_owned(),
    })
}

/// The login code in the URL the browser came back to, or why there is none.
fn login_code(callback_url: &str) -> Result<String, CoreError> {
    let mut url = url::Url::parse(callback_url).map_err(|_| CoreError::SsoFailed)?;
    let (mut code, mut error) = (None, None);
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "error" => error = Some(value.into_owned()),
            _ => {}
        }
    }
    url.set_query(None);
    if url.as_str() != paths::MOBILE_OIDC_REDIRECT {
        return Err(CoreError::SsoFailed);
    }
    match (code, error) {
        (Some(code), _) if !code.is_empty() => Ok(code),
        (_, Some(error)) if error == "access_denied" => Err(CoreError::SsoDenied),
        _ => Err(CoreError::SsoFailed),
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

/// Tidies what was typed as a server address: no surrounding space or trailing slash.
#[uniffi::export]
pub fn normalize_server_url(input: String) -> Result<String, CoreError> {
    let trimmed = input.trim().trim_end_matches('/');
    let url = url::Url::parse(trimmed).map_err(|_| CoreError::InvalidServerAddress)?;
    let is_web = matches!(url.scheme(), "http" | "https");
    if is_web && url.host_str().is_some() {
        Ok(trimmed.to_owned())
    } else {
        Err(CoreError::InvalidServerAddress)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use wiremock::matchers::{body_partial_json, method, path, query_param};
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

    const DEVICE_ID: &str = "00000000-0000-0000-0000-0000000000aa";

    #[test]
    fn entries_carry_the_icon_for_their_kind_and_mime() {
        let icon = |kind: &str, mime: &str| {
            let mut json = node("a", kind, 0);
            json["mime"] = mime.into();
            let node: protocol::Node = serde_json::from_value(json).expect("node");
            Entry::from(node).icon
        };
        assert_eq!(icon("folder", ""), EntryIcon::Folder);
        assert_eq!(icon("file", "image/png"), EntryIcon::Image);
        assert_eq!(icon("file", "application/pdf"), EntryIcon::Document);
        assert_eq!(icon("file", "application/zip"), EntryIcon::Other);
    }

    /// Every token a client reported, in order.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<Option<String>>>);

    impl SessionListener for Recorder {
        fn refresh_token_changed(&self, token: Option<String>) {
            self.0.lock().unwrap().push(token);
        }
    }

    fn device() -> DeviceIdentity {
        DeviceIdentity {
            id: DEVICE_ID.to_owned(),
            name: "Pixel".to_owned(),
        }
    }

    fn session_client(server: &MockServer) -> (CoreClient, Arc<Recorder>) {
        let recorder = Arc::new(Recorder::default());
        let client =
            CoreClient::with_session(server.uri(), device(), recorder.clone()).expect("client");
        (client, recorder)
    }

    fn tokens(refresh_token: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "access", "refresh_token": refresh_token, "expires_in": 900,
        }))
    }

    fn unauthorized() -> ResponseTemplate {
        ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "unauthorized", "message": "invalid credentials",
        }))
    }

    async fn mount_me(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path(protocol::paths::ME))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000001",
                "username": "me",
                "lost_and_found_id": "00000000-0000-0000-0000-000000000002",
            })))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn providers_reports_both_modes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(protocol::paths::PROVIDERS))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "password": false, "oidc": {"name": "Keycloak"},
            })))
            .mount(&server)
            .await;

        let options = CoreClient::new(server.uri()).providers().await.unwrap();
        assert_eq!(
            options,
            SignInOptions {
                password: false,
                sso_name: Some("Keycloak".to_owned()),
            }
        );
    }

    #[tokio::test]
    async fn a_refused_password_is_invalid_credentials() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGIN))
            .respond_with(unauthorized())
            .mount(&server)
            .await;

        let (client, recorder) = session_client(&server);
        let err = client.login("me".into(), "no".into()).await.unwrap_err();
        assert!(matches!(err, CoreError::InvalidCredentials), "{err:?}");
        assert!(recorder.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn login_names_the_device_and_reports_the_token_then_logout_clears_it() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGIN))
            .and(body_partial_json(serde_json::json!({
                "device": {"id": DEVICE_ID, "name": "Pixel"},
            })))
            .respond_with(tokens("first"))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGOUT))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        mount_me(&server).await;

        let (client, recorder) = session_client(&server);
        let user = client.login("me".into(), "secret".into()).await.unwrap();
        assert_eq!(user.username, "me");
        client.logout().await.unwrap();
        assert_eq!(
            *recorder.0.lock().unwrap(),
            [Some("first".to_owned()), None]
        );
    }

    /// The thread each token was reported on, which is where the request ran.
    #[derive(Default)]
    struct ThreadRecorder(Mutex<Vec<std::thread::ThreadId>>);

    impl SessionListener for ThreadRecorder {
        fn refresh_token_changed(&self, _token: Option<String>) {
            self.0.lock().unwrap().push(std::thread::current().id());
        }
    }

    #[tokio::test]
    async fn requests_do_not_run_on_the_callers_thread() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGIN))
            .respond_with(tokens("first"))
            .mount(&server)
            .await;
        mount_me(&server).await;

        let recorder = Arc::new(ThreadRecorder::default());
        let client =
            CoreClient::with_session(server.uri(), device(), recorder.clone()).expect("client");
        client.login("me".into(), "secret".into()).await.unwrap();

        let threads = recorder.0.lock().unwrap();
        assert_eq!(threads.len(), 1);
        assert_ne!(threads[0], std::thread::current().id());
    }

    #[test]
    fn unreachable_errors_keep_only_the_cause() {
        assert_eq!(
            last_cause(
                "request failed: error sending request for url (https://x/y): \
                 client error (Connect): invalid peer certificate: Expired"
            ),
            "Expired"
        );
        assert_eq!(last_cause("HTTP 502 Bad Gateway"), "HTTP 502 Bad Gateway");
    }

    #[tokio::test]
    async fn restore_reports_the_rotated_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::REFRESH))
            .and(body_partial_json(
                serde_json::json!({"refresh_token": "saved"}),
            ))
            .respond_with(tokens("rotated"))
            .mount(&server)
            .await;
        mount_me(&server).await;

        let (client, recorder) = session_client(&server);
        let user = client.restore("saved".into()).await.unwrap();
        assert_eq!(user.username, "me");
        assert_eq!(*recorder.0.lock().unwrap(), [Some("rotated".to_owned())]);
    }

    #[tokio::test]
    async fn restoring_an_ended_session_is_unauthorized() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::REFRESH))
            .respond_with(unauthorized())
            .mount(&server)
            .await;

        let (client, _) = session_client(&server);
        let err = client.restore("saved".into()).await.unwrap_err();
        assert!(matches!(err, CoreError::Unauthorized), "{err:?}");
    }

    #[tokio::test]
    async fn restoring_while_offline_keeps_the_saved_token() {
        let recorder = Arc::new(Recorder::default());
        let client =
            CoreClient::with_session("http://127.0.0.1:9".into(), device(), recorder.clone())
                .unwrap();
        let err = client.restore("saved".into()).await.unwrap_err();
        assert!(matches!(err, CoreError::Unreachable { .. }), "{err:?}");
        assert!(recorder.0.lock().unwrap().is_empty());
    }

    #[test]
    fn begin_sso_asks_for_the_mobile_redirect_with_a_fresh_verifier() {
        let client = CoreClient::new("https://files.example.com".into());
        let first = client.begin_sso().unwrap();
        let second = client.begin_sso().unwrap();
        assert_ne!(first.verifier, second.verifier);

        let url = url::Url::parse(&first.url).unwrap();
        assert_eq!(url.path(), protocol::paths::OIDC_START);
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["redirect_uri"], protocol::paths::MOBILE_OIDC_REDIRECT);
        assert_eq!(
            query["code_challenge"],
            Pkce::from_verifier(first.verifier).challenge()
        );
    }

    #[tokio::test]
    async fn finish_sso_exchanges_the_code_with_the_verifier() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::OIDC_EXCHANGE))
            .and(body_partial_json(serde_json::json!({
                "code": "login code",
                "code_verifier": "the-verifier",
                "device": {"id": DEVICE_ID},
            })))
            .respond_with(tokens("first"))
            .mount(&server)
            .await;
        mount_me(&server).await;

        let (client, recorder) = session_client(&server);
        let user = client
            .finish_sso(
                "app.homespace://oidc/callback?code=login%20code".into(),
                "the-verifier".into(),
            )
            .await
            .unwrap();
        assert_eq!(user.username, "me");
        assert_eq!(*recorder.0.lock().unwrap(), [Some("first".to_owned())]);
    }

    #[tokio::test]
    async fn a_refused_login_code_is_expired() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::OIDC_EXCHANGE))
            .respond_with(unauthorized())
            .mount(&server)
            .await;

        let (client, _) = session_client(&server);
        let err = client
            .finish_sso(
                "app.homespace://oidc/callback?code=old".into(),
                "the-verifier".into(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::SsoExpired), "{err:?}");
    }

    #[test]
    fn reads_the_callback_url() {
        assert_eq!(
            login_code("app.homespace://oidc/callback?code=a%20b").unwrap(),
            "a b"
        );
        assert!(matches!(
            login_code("app.homespace://oidc/callback?error=access_denied"),
            Err(CoreError::SsoDenied)
        ));
        for url in [
            "app.homespace://oidc/callback?error=login_failed",
            "app.homespace://oidc/callback?code=",
            "app.homespace://oidc/callback",
            "app.homespace://oidc/other?code=x",
            "app.homespace://oidc/callback?code=x#frag",
            "https://evil.example.com/?code=x",
            "not a url",
        ] {
            assert!(
                matches!(login_code(url), Err(CoreError::SsoFailed)),
                "{url}"
            );
        }
    }

    #[test]
    fn device_names_fit_the_server_limit() {
        let long = DeviceIdentity {
            id: DEVICE_ID.to_owned(),
            name: "é".repeat(150),
        };
        assert_eq!(device_info(long).unwrap().name.chars().count(), 100);

        let blank = DeviceIdentity {
            id: DEVICE_ID.to_owned(),
            name: "  ".to_owned(),
        };
        assert_eq!(device_info(blank).unwrap().name, PLATFORM);

        let bad_id = DeviceIdentity {
            id: "nope".to_owned(),
            name: "Pixel".to_owned(),
        };
        assert!(matches!(
            device_info(bad_id),
            Err(CoreError::InvalidId { .. })
        ));
    }

    #[test]
    fn normalizes_server_addresses() {
        assert_eq!(
            normalize_server_url("  https://files.example.com/ \n".into()).unwrap(),
            "https://files.example.com"
        );
        assert_eq!(
            normalize_server_url("http://10.0.2.2:8080".into()).unwrap(),
            "http://10.0.2.2:8080"
        );
        for input in [
            "",
            "files.example.com",
            "ftp://files.example.com",
            "http://",
        ] {
            assert!(
                matches!(
                    normalize_server_url(input.into()),
                    Err(CoreError::InvalidServerAddress)
                ),
                "{input}"
            );
        }
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

    /// The names Home lists for a signed-in user whose Lost & Found holds `contents`.
    async fn home_names(contents: serde_json::Value) -> Vec<String> {
        const LOST_AND_FOUND: &str = "00000000-0000-0000-0000-000000000002";
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(protocol::paths::LOGIN))
            .respond_with(tokens("refresh"))
            .mount(&server)
            .await;
        mount_me(&server).await;
        Mock::given(method("GET"))
            .and(path(protocol::paths::NODES))
            .and(query_param("parent_id", LOST_AND_FOUND))
            .respond_with(ResponseTemplate::new(200).set_body_json(contents))
            .mount(&server)
            .await;
        let mut found = node("Lost & Found", "folder", 0);
        found["id"] = LOST_AND_FOUND.into();
        Mock::given(method("GET"))
            .and(path(protocol::paths::NODES))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([found, node("Photos", "folder", 0)])),
            )
            .mount(&server)
            .await;

        let client = CoreClient::new(server.uri());
        client.login("me".into(), "secret".into()).await.unwrap();
        let entries = client.list(None).await.unwrap();
        entries.into_iter().map(|entry| entry.name).collect()
    }

    #[tokio::test]
    async fn home_leaves_out_an_empty_lost_and_found() {
        assert_eq!(home_names(serde_json::json!([])).await, ["Photos"]);
    }

    #[tokio::test]
    async fn home_shows_a_lost_and_found_with_items() {
        let contents = serde_json::json!([node("a.txt", "file", 10)]);
        assert_eq!(home_names(contents).await, ["Lost & Found", "Photos"]);
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
