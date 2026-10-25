// Each integration test binary uses a different subset of these helpers.
#![allow(dead_code)]

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header::AUTHORIZATION},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use api::AppState;
use auth::{AuthConfig, AuthService};
use db::Db;
use files::{BlobStore, FileService};
use notifications::Hub;

pub const PASSWORD: &str = "password-123";

pub struct TestApp {
    pub router: Router,
    pub state: AppState,
    _storage: tempfile::TempDir,
}

pub struct Response {
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

pub async fn setup(pool: PgPool) -> TestApp {
    let db = Db::from_pool(pool);
    let storage = tempfile::tempdir().expect("tempdir");
    let store = BlobStore::open(storage.path()).await.expect("store");
    let config = AuthConfig {
        jwt_secret: b"integration-test-secret-integration".to_vec(),
        access_ttl: time::Duration::minutes(15),
        refresh_ttl: time::Duration::days(30),
    };
    let state = AppState {
        auth: AuthService::new(db.clone(), config).expect("auth"),
        files: FileService::new(db.clone(), store, 1024 * 1024),
        hub: Hub::new(),
        db,
    };
    TestApp {
        router: api::router(state.clone()),
        state,
        _storage: storage,
    }
}

impl TestApp {
    pub async fn send(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Body,
        content_type: Option<&str>,
    ) -> Response {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(content_type) = content_type {
            request = request.header("content-type", content_type);
        }
        let response = self
            .router
            .clone()
            .oneshot(request.body(body).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes()
            .to_vec();
        Response { status, body }
    }

    pub async fn json(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        payload: Value,
    ) -> Response {
        self.send(
            method,
            uri,
            token,
            Body::from(payload.to_string()),
            Some("application/json"),
        )
        .await
    }

    pub async fn get(&self, uri: &str, token: &str) -> Response {
        self.send(Method::GET, uri, Some(token), Body::empty(), None)
            .await
    }

    pub async fn user_token(&self, username: &str) -> String {
        self.state
            .auth
            .create_user(username, PASSWORD)
            .await
            .expect("create user");
        let response = self
            .json(
                Method::POST,
                "/api/v1/auth/login",
                None,
                json!({ "username": username, "password": PASSWORD }),
            )
            .await;
        response.json()["access_token"]
            .as_str()
            .expect("access token")
            .to_owned()
    }

    pub async fn upload(&self, token: &str, query: &str, content: &'static str) -> Response {
        self.upload_as(token, query, content, "application/octet-stream")
            .await
    }

    pub async fn upload_as(
        &self,
        token: &str,
        query: &str,
        content: &'static str,
        content_type: &str,
    ) -> Response {
        self.send(
            Method::POST,
            &format!("/api/v1/files?{query}"),
            Some(token),
            Body::from(content),
            Some(content_type),
        )
        .await
    }
}

pub const SHORT_WAIT: Duration = Duration::from_millis(300);
