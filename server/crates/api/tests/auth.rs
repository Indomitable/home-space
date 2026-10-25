mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

use common::{PASSWORD, setup};

#[sqlx::test(migrations = "../../migrations")]
async fn requests_without_a_token_are_unauthorized(pool: PgPool) {
    let app = setup(pool).await;
    let response = app
        .send(Method::GET, "/api/v1/nodes", None, Default::default(), None)
        .await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn garbage_token_is_unauthorized(pool: PgPool) {
    let app = setup(pool).await;
    let response = app.get("/api/v1/nodes", "garbage").await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn wrong_password_is_unauthorized(pool: PgPool) {
    let app = setup(pool).await;
    app.user_token("alice").await;
    let response = app
        .json(
            Method::POST,
            "/api/v1/auth/login",
            None,
            json!({ "username": "alice", "password": "wrong-password" }),
        )
        .await;
    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn me_returns_the_logged_in_user(pool: PgPool) {
    let app = setup(pool).await;
    let token = app.user_token("alice").await;
    let response = app.get("/api/v1/me", &token).await;
    assert_eq!(response.json()["username"], "alice");
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_issues_new_tokens_and_old_refresh_token_stops_working(pool: PgPool) {
    let app = setup(pool).await;
    app.user_token("alice").await;
    let login = app
        .json(
            Method::POST,
            "/api/v1/auth/login",
            None,
            json!({ "username": "alice", "password": PASSWORD }),
        )
        .await;
    let old_refresh = login.json()["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_owned();

    let refreshed = app
        .json(
            Method::POST,
            "/api/v1/auth/refresh",
            None,
            json!({ "refresh_token": old_refresh }),
        )
        .await;
    let reused = app
        .json(
            Method::POST,
            "/api/v1/auth/refresh",
            None,
            json!({ "refresh_token": old_refresh }),
        )
        .await;

    assert_eq!(refreshed.status, StatusCode::OK);
    assert_eq!(reused.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_revokes_the_refresh_token(pool: PgPool) {
    let app = setup(pool).await;
    app.user_token("alice").await;
    let login = app
        .json(
            Method::POST,
            "/api/v1/auth/login",
            None,
            json!({ "username": "alice", "password": PASSWORD }),
        )
        .await;
    let refresh = login.json()["refresh_token"]
        .as_str()
        .expect("refresh")
        .to_owned();

    let logout = app
        .json(
            Method::POST,
            "/api/v1/auth/logout",
            None,
            json!({ "refresh_token": refresh }),
        )
        .await;
    let after = app
        .json(
            Method::POST,
            "/api/v1/auth/refresh",
            None,
            json!({ "refresh_token": refresh }),
        )
        .await;

    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
}
