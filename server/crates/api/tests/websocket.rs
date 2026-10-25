mod common;

use futures_util::StreamExt;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use common::{SHORT_WAIT, setup};

async fn connect(
    addr: std::net::SocketAddr,
    token: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let mut request = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .expect("request");
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {token}").parse().expect("header"),
    );
    connect_async(request).await.expect("connect").0
}

#[sqlx::test(migrations = "../../migrations")]
async fn owner_receives_node_created_and_others_do_not(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(axum::serve(listener, app.router.clone()).into_future());

    let mut alice_ws = connect(addr, &alice).await;
    let mut bob_ws = connect(addr, &bob).await;
    app.upload(&alice, "name=a.txt", "hello").await;

    let message = tokio::time::timeout(SHORT_WAIT * 10, alice_ws.next())
        .await
        .expect("alice should receive an event")
        .expect("stream open")
        .expect("message");
    let Message::Text(text) = message else {
        panic!("expected text message, got {message:?}");
    };
    let event: serde_json::Value = serde_json::from_str(text.as_str()).expect("json");
    assert_eq!(event["type"], "node_created");
    assert_eq!(event["node"]["name"], "a.txt");
    assert!(
        tokio::time::timeout(SHORT_WAIT, bob_ws.next())
            .await
            .is_err(),
        "bob must not receive alice's events"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn websocket_requires_authentication(pool: PgPool) {
    let app = setup(pool).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(axum::serve(listener, app.router.clone()).into_future());

    let request = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .expect("request");

    assert!(connect_async(request).await.is_err());
}
