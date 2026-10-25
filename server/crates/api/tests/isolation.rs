mod common;

use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;

use common::setup;

#[sqlx::test(migrations = "../../migrations")]
async fn user_b_cannot_touch_user_a_nodes(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;

    let folder = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "private" }),
        )
        .await;
    let folder_id = folder.json()["id"].as_str().expect("id").to_owned();
    let file = app
        .upload(
            &alice,
            &format!("name=secret.txt&parent_id={folder_id}"),
            "secret",
        )
        .await;
    let file_id = file.json()["id"].as_str().expect("id").to_owned();

    let attempts = [
        app.get(&format!("/api/v1/nodes/{file_id}"), &bob).await,
        app.get(&format!("/api/v1/files/{file_id}/content"), &bob)
            .await,
        app.get(&format!("/api/v1/nodes?parent_id={folder_id}"), &bob)
            .await,
        app.upload(&bob, &format!("name=x.txt&parent_id={folder_id}"), "x")
            .await,
        app.json(
            Method::POST,
            "/api/v1/folders",
            Some(&bob),
            json!({ "name": "x", "parent_id": folder_id }),
        )
        .await,
        app.send(
            Method::PUT,
            &format!("/api/v1/files/{file_id}/content"),
            Some(&bob),
            Body::from("overwrite"),
            None,
        )
        .await,
        app.json(
            Method::PATCH,
            &format!("/api/v1/nodes/{file_id}"),
            Some(&bob),
            json!({ "name": "stolen.txt" }),
        )
        .await,
        app.send(
            Method::DELETE,
            &format!("/api/v1/nodes/{file_id}"),
            Some(&bob),
            Body::empty(),
            None,
        )
        .await,
    ];

    let statuses: Vec<StatusCode> = attempts.iter().map(|response| response.status).collect();
    assert_eq!(statuses, vec![StatusCode::NOT_FOUND; attempts.len()]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_b_cannot_move_own_node_into_user_a_folder(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    let folder = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "private" }),
        )
        .await;
    let file = app.upload(&bob, "name=mine.txt", "mine").await;

    let response = app
        .json(
            Method::PATCH,
            &format!("/api/v1/nodes/{}", file.json()["id"].as_str().expect("id")),
            Some(&bob),
            json!({ "parent_id": folder.json()["id"] }),
        )
        .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_a_data_is_intact_after_user_b_attempts(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    let file = app.upload(&alice, "name=secret.txt", "secret").await;
    let uri = format!(
        "/api/v1/files/{}/content",
        file.json()["id"].as_str().expect("id")
    );

    app.send(Method::PUT, &uri, Some(&bob), Body::from("overwrite"), None)
        .await;

    assert_eq!(app.get(&uri, &alice).await.body, b"secret");
}
