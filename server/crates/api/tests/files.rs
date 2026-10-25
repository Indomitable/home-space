mod common;

use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;

use common::setup;

fn id_of(response: &common::Response) -> String {
    response.json()["id"].as_str().expect("id").to_owned()
}

#[sqlx::test(migrations = "../../migrations")]
async fn uploaded_file_downloads_with_same_content(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let created = app.upload(&token, "name=hello.txt", "hello world").await;
    assert_eq!(created.status, StatusCode::CREATED);

    let download = app
        .get(
            &format!("/api/v1/files/{}/content", id_of(&created)),
            &token,
        )
        .await;

    assert_eq!(download.body, b"hello world");
}

#[sqlx::test(migrations = "../../migrations")]
async fn listing_returns_children_of_a_folder(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let folder = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&token),
            json!({ "name": "docs" }),
        )
        .await;
    let folder_id = id_of(&folder);
    app.upload(&token, &format!("name=a.txt&parent_id={folder_id}"), "a")
        .await;

    let listing = app
        .get(&format!("/api/v1/nodes?parent_id={folder_id}"), &token)
        .await;
    let root = app.get("/api/v1/nodes", &token).await;

    assert_eq!(listing.json().as_array().map(Vec::len), Some(1));
    assert_eq!(root.json().as_array().map(Vec::len), Some(1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_names_conflict(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    app.upload(&token, "name=a.txt", "1").await;
    let second = app.upload(&token, "name=a.txt", "2").await;
    assert_eq!(second.status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_names_are_rejected(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let response = app.upload(&token, "name=..", "1").await;
    assert_eq!(response.status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn moving_a_folder_into_its_child_is_rejected(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let outer = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&token),
            json!({ "name": "outer" }),
        )
        .await;
    let inner = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&token),
            json!({ "name": "inner", "parent_id": id_of(&outer) }),
        )
        .await;

    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/nodes/{}", id_of(&outer)),
            Some(&token),
            json!({ "parent_id": id_of(&inner) }),
        )
        .await;

    assert_eq!(response.status, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn rename_and_move_to_root_work(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let folder = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&token),
            json!({ "name": "docs" }),
        )
        .await;
    let file = app
        .upload(
            &token,
            &format!("name=a.txt&parent_id={}", id_of(&folder)),
            "a",
        )
        .await;

    let moved = app
        .json(
            Method::PATCH,
            &format!("/api/v1/nodes/{}", id_of(&file)),
            Some(&token),
            json!({ "name": "b.txt", "parent_id": null }),
        )
        .await;

    assert_eq!(moved.json()["name"], "b.txt");
    assert_eq!(moved.json()["parent_id"], Value::Null);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_removes_the_node(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let file = app.upload(&token, "name=a.txt", "a").await;
    let uri = format!("/api/v1/nodes/{}", id_of(&file));

    let deleted = app
        .send(Method::DELETE, &uri, Some(&token), Body::empty(), None)
        .await;
    let after = app.get(&uri, &token).await;

    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(after.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn oversized_upload_returns_413(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let big = "x".repeat(1024 * 1024 + 1);
    let response = app
        .send(
            Method::POST,
            "/api/v1/files?name=big.bin",
            Some(&token),
            Body::from(big),
            None,
        )
        .await;
    assert_eq!(response.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test(migrations = "../../migrations")]
async fn replace_content_changes_downloaded_bytes(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let file = app.upload(&token, "name=a.txt", "old").await;
    let uri = format!("/api/v1/files/{}/content", id_of(&file));

    app.send(
        Method::PUT,
        &uri,
        Some(&token),
        Body::from("new content"),
        None,
    )
    .await;

    assert_eq!(app.get(&uri, &token).await.body, b"new content");
}

#[sqlx::test(migrations = "../../migrations")]
async fn ancestors_endpoint_returns_the_chain_and_hides_foreign_nodes(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    let parent = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "docs" }),
        )
        .await
        .json()["id"]
        .as_str()
        .expect("id")
        .to_owned();
    let child = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "inner", "parent_id": parent }),
        )
        .await
        .json()["id"]
        .as_str()
        .expect("id")
        .to_owned();

    let own = app
        .get(&format!("/api/v1/nodes/{child}/ancestors"), &alice)
        .await;
    let foreign = app
        .get(&format!("/api/v1/nodes/{child}/ancestors"), &bob)
        .await;

    assert_eq!(own.json()[0]["name"], "docs");
    assert_eq!(foreign.status, StatusCode::NOT_FOUND);
}
