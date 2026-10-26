use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_bytes, header, method, path, query_param},
};

use api_client::{ApiError, Client};
use protocol::{ErrorCode, NodeId};

const USER_ID: &str = "00000000-0000-0000-0000-000000000001";
const NODE_ID: &str = "00000000-0000-0000-0000-000000000002";
const PARENT_ID: &str = "00000000-0000-0000-0000-000000000003";

fn pair(access: &str, refresh: &str) -> Value {
    json!({ "access_token": access, "refresh_token": refresh, "expires_in": 900 })
}

fn node_json() -> Value {
    json!({
        "id": NODE_ID,
        "parent_id": PARENT_ID,
        "name": "a.txt",
        "kind": "file",
        "size": 5,
        "sha256": null,
        "mime": "text/plain",
        "version": 1,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    })
}

fn node_id(raw: &str) -> NodeId {
    NodeId(raw.parse().expect("uuid"))
}

async fn logged_in(server: &MockServer) -> Client {
    let client = Client::new(server.uri());
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pair("old", "refresh-1")))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "id": USER_ID, "username": "alice" })),
        )
        .mount(server)
        .await;
    client.login("alice", "pw").await.expect("login");
    client
}

#[tokio::test]
async fn upload_sends_name_parent_and_body() {
    let server = MockServer::start().await;
    let client = logged_in(&server).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/files"))
        .and(query_param("name", "a.txt"))
        .and(query_param("parent_id", PARENT_ID))
        .and(header("authorization", "Bearer old"))
        .and(body_bytes(b"hello".to_vec()))
        .respond_with(ResponseTemplate::new(201).set_body_json(node_json()))
        .expect(1)
        .mount(&server)
        .await;

    let node = client
        .upload(Some(node_id(PARENT_ID)), "a.txt", || {
            reqwest::Body::from("hello")
        })
        .await
        .expect("upload");

    assert_eq!(node.name, "a.txt");
}

#[tokio::test]
async fn upload_is_retried_with_a_fresh_body_after_a_refresh() {
    let server = MockServer::start().await;
    let client = logged_in(&server).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/files"))
        .and(header("authorization", "Bearer old"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pair("new", "refresh-2")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/files"))
        .and(header("authorization", "Bearer new"))
        .and(body_bytes(b"hello".to_vec()))
        .respond_with(ResponseTemplate::new(201).set_body_json(node_json()))
        .expect(1)
        .mount(&server)
        .await;

    client
        .upload(None, "a.txt", || reqwest::Body::from("hello"))
        .await
        .expect("upload");
}

#[tokio::test]
async fn download_returns_the_file_bytes() {
    let server = MockServer::start().await;
    let client = logged_in(&server).await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/files/{NODE_ID}/content")))
        .and(header("authorization", "Bearer old"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .mount(&server)
        .await;

    let response = client.download(node_id(NODE_ID)).await.expect("download");

    assert_eq!(response.bytes().await.expect("bytes").as_ref(), b"hello");
}

#[tokio::test]
async fn download_of_a_missing_file_is_an_api_error() {
    let server = MockServer::start().await;
    let client = logged_in(&server).await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/files/{NODE_ID}/content")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": "not_found",
            "message": "node not found",
        })))
        .mount(&server)
        .await;

    let err = client
        .download(node_id(NODE_ID))
        .await
        .expect_err("missing");

    assert!(matches!(
        err,
        ApiError::Api {
            code: ErrorCode::NotFound,
            ..
        }
    ));
}
