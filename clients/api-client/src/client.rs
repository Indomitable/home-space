use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::sync::Mutex as AsyncMutex;

use protocol::{
    CreateFolderRequest, ErrorBody, ListQuery, LoginRequest, Me, Node, NodeId, RefreshRequest,
    TicketRequest, TicketResponse, TokenPair, UpdateRequest, paths,
};

use crate::ApiError;

/// Called with the new refresh token after every rotation, or `None` once the session ends.
pub type TokenListener = Arc<dyn Fn(Option<&str>) + Send + Sync>;

/// Client for one server; cheap to clone, clones share the session.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    shared: Arc<Shared>,
}

struct Shared {
    session: Mutex<Option<TokenPair>>,
    refresh_gate: AsyncMutex<()>,
    listener: Option<TokenListener>,
}

impl Client {
    /// Creates a signed-out client for the server at `base_url` (no trailing slash).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::build(base_url.into(), None)
    }

    /// Like [`Client::new`], notifying `listener` whenever the refresh token changes.
    pub fn with_listener(base_url: impl Into<String>, listener: TokenListener) -> Self {
        Self::build(base_url.into(), Some(listener))
    }

    fn build(base_url: String, listener: Option<TokenListener>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url,
            shared: Arc::new(Shared {
                session: Mutex::new(None),
                refresh_gate: AsyncMutex::new(()),
                listener,
            }),
        }
    }

    /// Absolute URL for a server path.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Websocket URL for the events stream, authenticated by `ticket`.
    pub fn events_url(&self, ticket: &str) -> String {
        let base = self
            .base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        format!("{base}{}?{}={ticket}", paths::WS, paths::TICKET_PARAM)
    }

    /// URL that downloads a file when opened, authenticated by `ticket`.
    pub fn download_url(&self, id: NodeId, ticket: &str) -> String {
        format!(
            "{}?{}={ticket}",
            self.url(&paths::file_content(id)),
            paths::TICKET_PARAM
        )
    }

    /// Current access token, if signed in.
    pub fn access_token(&self) -> Option<String> {
        self.session()
            .as_ref()
            .map(|session| session.access_token.clone())
    }

    /// Exchanges the refresh token for a new session unless another caller already did
    /// since `stale_access_token` was read.
    pub async fn refresh_session(&self, stale_access_token: &str) -> Result<(), ApiError> {
        let _gate = self.shared.refresh_gate.lock().await;
        let current = self.session().clone().ok_or(ApiError::Unauthorized)?;
        if current.access_token == stale_access_token {
            self.rotate(&current.refresh_token).await
        } else {
            Ok(())
        }
    }

    /// Signs in with a username and password.
    pub async fn login(&self, username: &str, password: &str) -> Result<Me, ApiError> {
        let request = LoginRequest {
            username: username.to_owned(),
            password: password.to_owned(),
        };
        let response = self
            .http
            .post(self.url(paths::LOGIN))
            .json(&request)
            .send()
            .await?;
        let pair: TokenPair = decode(response).await?;
        self.store(pair);
        self.me().await
    }

    /// Resumes a session from a saved refresh token.
    pub async fn restore(&self, refresh_token: &str) -> Result<Me, ApiError> {
        {
            let _gate = self.shared.refresh_gate.lock().await;
            self.rotate(refresh_token).await?;
        }
        self.me().await
    }

    /// Ends the session and revokes its refresh token.
    pub async fn logout(&self) -> Result<(), ApiError> {
        let session = self.session().clone();
        self.clear();
        if let Some(session) = session {
            let request = RefreshRequest {
                refresh_token: session.refresh_token,
            };
            let response = self
                .http
                .post(self.url(paths::LOGOUT))
                .json(&request)
                .send()
                .await?;
            expect_success(response).await?;
        }
        Ok(())
    }

    /// The signed-in user.
    pub async fn me(&self) -> Result<Me, ApiError> {
        self.send_json(|| self.http.get(self.url(paths::ME))).await
    }

    /// Lists a folder, or the root when `parent` is `None`.
    pub async fn list(&self, parent: Option<NodeId>) -> Result<Vec<Node>, ApiError> {
        let query = ListQuery { parent_id: parent };
        self.send_json(|| self.http.get(self.url(paths::NODES)).query(&query))
            .await
    }

    /// Metadata of one node.
    pub async fn get(&self, id: NodeId) -> Result<Node, ApiError> {
        self.send_json(|| self.http.get(self.url(&paths::node(id))))
            .await
    }

    /// Ancestors of a node from the root down to its parent.
    pub async fn ancestors(&self, id: NodeId) -> Result<Vec<Node>, ApiError> {
        self.send_json(|| self.http.get(self.url(&paths::ancestors(id))))
            .await
    }

    /// Creates a folder under `parent` (or the root).
    pub async fn create_folder(
        &self,
        parent: Option<NodeId>,
        name: &str,
    ) -> Result<Node, ApiError> {
        let request = CreateFolderRequest {
            parent_id: parent,
            name: name.to_owned(),
        };
        self.send_json(|| self.http.post(self.url(paths::FOLDERS)).json(&request))
            .await
    }

    /// Renames a node.
    pub async fn rename(&self, id: NodeId, name: &str) -> Result<Node, ApiError> {
        self.update(
            id,
            &UpdateRequest {
                name: Some(name.to_owned()),
                parent_id: None,
            },
        )
        .await
    }

    /// Moves a node into `parent`, or to the root when `None`.
    pub async fn move_to(&self, id: NodeId, parent: Option<NodeId>) -> Result<Node, ApiError> {
        self.update(
            id,
            &UpdateRequest {
                name: None,
                parent_id: Some(parent),
            },
        )
        .await
    }

    /// Deletes a node and everything below it.
    pub async fn delete(&self, id: NodeId) -> Result<(), ApiError> {
        let response = self
            .send(|| self.http.delete(self.url(&paths::node(id))))
            .await?;
        expect_success(response).await
    }

    /// Requests a short-lived ticket for a websocket or a browser download.
    pub async fn ticket(&self, request: &TicketRequest) -> Result<TicketResponse, ApiError> {
        self.send_json(|| self.http.post(self.url(paths::TICKETS)).json(request))
            .await
    }

    async fn update(&self, id: NodeId, request: &UpdateRequest) -> Result<Node, ApiError> {
        self.send_json(|| self.http.patch(self.url(&paths::node(id))).json(request))
            .await
    }

    async fn send_json<T, F>(&self, build: F) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
        F: Fn() -> RequestBuilder,
    {
        decode(self.send(build).await?).await
    }

    async fn send<F>(&self, build: F) -> Result<Response, ApiError>
    where
        F: Fn() -> RequestBuilder,
    {
        let token = self.access_token().ok_or(ApiError::Unauthorized)?;
        let response = build().bearer_auth(&token).send().await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            self.refresh_session(&token).await?;
            let token = self.access_token().ok_or(ApiError::Unauthorized)?;
            Ok(build().bearer_auth(token).send().await?)
        } else {
            Ok(response)
        }
    }

    async fn rotate(&self, refresh_token: &str) -> Result<(), ApiError> {
        let request = RefreshRequest {
            refresh_token: refresh_token.to_owned(),
        };
        let response = self
            .http
            .post(self.url(paths::REFRESH))
            .json(&request)
            .send()
            .await?;
        if response.status() == StatusCode::UNAUTHORIZED {
            self.clear();
            Err(ApiError::Unauthorized)
        } else {
            self.store(decode(response).await?);
            Ok(())
        }
    }

    fn store(&self, pair: TokenPair) {
        let refresh_token = pair.refresh_token.clone();
        *self.session() = Some(pair);
        self.notify(Some(&refresh_token));
    }

    fn clear(&self) {
        *self.session() = None;
        self.notify(None);
    }

    fn notify(&self, refresh_token: Option<&str>) {
        if let Some(listener) = &self.shared.listener {
            listener(refresh_token);
        }
    }

    fn session(&self) -> MutexGuard<'_, Option<TokenPair>> {
        self.shared
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    if response.status().is_success() {
        Ok(response.json().await?)
    } else {
        Err(error_from(response).await)
    }
}

async fn expect_success(response: Response) -> Result<(), ApiError> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(error_from(response).await)
    }
}

async fn error_from(response: Response) -> ApiError {
    let status = response.status();
    match response.json::<ErrorBody>().await {
        Ok(_) if status == StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        Ok(body) => ApiError::Api {
            code: body.error,
            message: body.message,
        },
        Err(_) if status == StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        Err(err) => ApiError::Decode(format!("{status}: {err}")),
    }
}
