use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

use api_client::Client;

const NODE_ID: &str = "00000000-0000-0000-0000-000000000002";

#[tokio::test]
async fn search_sends_the_query_and_decodes_hits() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "access_token": "old", "refresh_token": "refresh-1", "expires_in": 900 }),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({ "id": "00000000-0000-0000-0000-000000000001", "username": "alice" }),
        ))
        .mount(&server)
        .await;
    let client = Client::new(server.uri());
    client.login("alice", "pw").await.expect("login");
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(header("authorization", "Bearer old"))
        .and(query_param("q", "holi day"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "node": {
                "id": NODE_ID,
                "parent_id": null,
                "name": "holiday.jpg",
                "kind": "file",
                "size": 5,
                "sha256": null,
                "mime": "image/jpeg",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z",
            },
            "location": ["trips"],
        }])))
        .expect(1)
        .mount(&server)
        .await;

    let hits = client.search("holi day").await.expect("search");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node.name, "holiday.jpg");
    assert_eq!(hits[0].location, ["trips"]);
}
