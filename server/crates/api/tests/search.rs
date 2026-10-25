mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

use common::setup;

#[sqlx::test(migrations = "../../migrations")]
async fn search_requires_authentication(pool: PgPool) {
    let app = setup(pool).await;
    let response = app
        .send(
            Method::GET,
            "/api/v1/search?q=a",
            None,
            axum::body::Body::empty(),
            None,
        )
        .await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn uploaded_files_are_found_by_name_with_their_location(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let folder = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "trips" }),
        )
        .await;
    let folder_id = folder.json()["id"].as_str().expect("id").to_owned();
    app.upload(
        &alice,
        &format!("name=IMG_2024-holidayPhoto.jpg&parent_id={folder_id}"),
        "x",
    )
    .await;

    let response = app.get("/api/v1/search?q=holi", &alice).await;

    assert_eq!(response.status, StatusCode::OK);
    let hits = response.json();
    assert_eq!(hits[0]["node"]["name"], "IMG_2024-holidayPhoto.jpg");
    assert_eq!(hits[0]["location"], json!(["trips"]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn renamed_nodes_are_found_under_the_new_name_only(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let file = app.upload(&alice, "name=draft.txt", "x").await;
    let id = file.json()["id"].as_str().expect("id").to_owned();
    app.json(
        Method::PATCH,
        &format!("/api/v1/nodes/{id}"),
        Some(&alice),
        json!({ "name": "final.txt" }),
    )
    .await;

    assert!(
        app.get("/api/v1/search?q=draft", &alice)
            .await
            .json()
            .as_array()
            .expect("array")
            .is_empty()
    );
    assert_eq!(
        app.get("/api/v1/search?q=final", &alice).await.json()[0]["node"]["id"],
        id
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleted_nodes_disappear_from_results(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let file = app.upload(&alice, "name=gone.txt", "x").await;
    let id = file.json()["id"].as_str().expect("id").to_owned();
    app.send(
        Method::DELETE,
        &format!("/api/v1/nodes/{id}"),
        Some(&alice),
        axum::body::Body::empty(),
        None,
    )
    .await;

    let hits = app.get("/api/v1/search?q=gone", &alice).await.json();

    assert!(hits.as_array().expect("array").is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn users_never_see_each_others_results(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    app.upload(&alice, "name=secret.txt", "x").await;

    let hits = app.get("/api/v1/search?q=secret", &bob).await.json();

    assert!(hits.as_array().expect("array").is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn blank_query_returns_an_empty_list(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    app.upload(&alice, "name=a.txt", "x").await;

    let response = app.get("/api/v1/search?q=%20-", &alice).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json(), json!([]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn limit_caps_the_number_of_results(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    for name in ["a1.txt", "a2.txt", "a3.txt"] {
        app.upload(&alice, &format!("name={name}"), "x").await;
    }

    let hits = app.get("/api/v1/search?q=a&limit=2", &alice).await.json();

    assert_eq!(hits.as_array().expect("array").len(), 2);
}
