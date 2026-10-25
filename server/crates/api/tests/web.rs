mod common;

use axum::{body::Body, http::Method, http::StatusCode};
use sqlx::PgPool;
use tower::ServiceExt;

use common::setup;

const INDEX: &str = "<html>home-space</html>";

fn web_router(app: &common::TestApp, dir: &tempfile::TempDir) -> axum::Router {
    std::fs::write(dir.path().join("index.html"), INDEX).expect("index");
    api::router_with_web(app.state.clone(), dir.path())
}

async fn status_and_body(router: axum::Router, uri: &str) -> (StatusCode, String) {
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("body")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[sqlx::test(migrations = "../../migrations")]
async fn unknown_client_routes_serve_the_index(pool: PgPool) {
    let app = setup(pool).await;
    let dir = tempfile::tempdir().expect("tempdir");

    let (status, body) = status_and_body(web_router(&app, &dir), "/folder/some-id").await;

    assert_eq!((status, body.as_str()), (StatusCode::OK, INDEX));
}

#[sqlx::test(migrations = "../../migrations")]
async fn unknown_api_routes_stay_json_not_found(pool: PgPool) {
    let app = setup(pool).await;
    let dir = tempfile::tempdir().expect("tempdir");

    let (status, body) = status_and_body(web_router(&app, &dir), "/api/v1/nope").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("not_found"));
}
