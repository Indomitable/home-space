mod common;

use axum::{
    body::Body,
    http::{Method, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;

use common::{TestApp, setup};
use notifications::Event;

fn id_of(response: &common::Response) -> String {
    response.json()["id"].as_str().expect("id").to_owned()
}

async fn delete(app: &TestApp, token: &str, uri: &str) -> common::Response {
    app.send(Method::DELETE, uri, Some(token), Body::empty(), None)
        .await
}

async fn folder(app: &TestApp, token: &str, name: &str, parent: Option<&str>) -> String {
    let response = app
        .json(
            Method::POST,
            "/api/v1/folders",
            Some(token),
            json!({ "name": name, "parent_id": parent }),
        )
        .await;
    id_of(&response)
}

async fn upload(
    app: &TestApp,
    token: &str,
    parent: &str,
    name: &str,
    content: &'static str,
) -> String {
    let response = app
        .upload(token, &format!("name={name}&parent_id={parent}"), content)
        .await;
    id_of(&response)
}

async fn restore(app: &TestApp, token: &str, ids: &[&str], on_conflict: &str) -> common::Response {
    app.json(
        Method::POST,
        "/api/v1/trash/restore",
        Some(token),
        json!({ "ids": ids, "on_conflict": on_conflict }),
    )
    .await
}

async fn root_names(app: &TestApp, token: &str) -> Vec<String> {
    let listing = app.get("/api/v1/nodes", token).await.json();
    listing
        .as_array()
        .expect("array")
        .iter()
        .map(|node| node["name"].as_str().expect("name").to_owned())
        .collect()
}

async fn user_id(app: &TestApp, token: &str) -> db::UserId {
    let me = app.get("/api/v1/me", token).await.json();
    db::UserId(me["id"].as_str().expect("id").parse().expect("uuid"))
}

async fn lost_and_found(app: &TestApp, token: &str) -> String {
    let listing = app.get("/api/v1/nodes", token).await.json();
    listing
        .as_array()
        .expect("array")
        .iter()
        .find(|node| node["name"] == "Lost & Found")
        .and_then(|node| node["id"].as_str())
        .expect("lost and found")
        .to_owned()
}

#[sqlx::test(migrations = "../../migrations")]
async fn trash_requires_authentication(pool: PgPool) {
    let app = setup(pool).await;
    let list = app
        .send(Method::GET, "/api/v1/trash", None, Body::empty(), None)
        .await;
    let empty = app
        .send(Method::DELETE, "/api/v1/trash", None, Body::empty(), None)
        .await;
    assert_eq!(list.status, StatusCode::UNAUTHORIZED);
    assert_eq!(empty.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_a_folder_lists_its_files_in_the_bin(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let photos = folder(&app, &alice, "Photos", None).await;
    let year = folder(&app, &alice, "2024", Some(&photos)).await;
    let jpg = upload(&app, &alice, &year, "a.jpg", "image").await;
    let txt = upload(&app, &alice, &photos, "b.txt", "text").await;

    let response = delete(&app, &alice, &format!("/api/v1/nodes/{photos}")).await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);

    let listing = app.get("/api/v1/trash", &alice).await;
    assert_eq!(listing.status, StatusCode::OK);
    let items = listing.json();
    let mut paths = items
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item["original_path"].as_str().expect("path").to_owned())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths, ["/Photos/2024/a.jpg", "/Photos/b.txt"]);
    let ids = items
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item["node"]["id"].as_str().expect("id").to_owned())
        .collect::<Vec<_>>();
    assert!(ids.contains(&jpg) && ids.contains(&txt));
    let deleted_at = items[0]["deleted_at"].as_str().expect("deleted_at");
    assert!(deleted_at.contains('T'), "rfc3339: {deleted_at}");
    for id in [photos.as_str(), year.as_str()] {
        let gone = app.get(&format!("/api/v1/nodes/{id}"), &alice).await;
        assert_eq!(gone.status, StatusCode::NOT_FOUND);
    }
    assert_eq!(root_names(&app, &alice).await, ["Lost & Found"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn binned_files_are_gone_from_every_node_endpoint(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let file = upload(&app, &alice, &dir, "a.txt", "one").await;
    app.upload(&alice, &format!("name=a.txt&parent_id={dir}"), "two")
        .await;
    delete(&app, &alice, &format!("/api/v1/nodes/{file}")).await;

    let gets = [
        format!("/api/v1/nodes/{file}"),
        format!("/api/v1/files/{file}/content"),
        format!("/api/v1/nodes/{file}/versions"),
        format!("/api/v1/files/{file}/versions/1/content"),
        format!("/api/v1/nodes/{file}/ancestors"),
    ];
    for uri in gets {
        let response = app.get(&uri, &alice).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND, "{uri}");
    }
    let listing = app
        .get(&format!("/api/v1/nodes?parent_id={dir}"), &alice)
        .await;
    assert_eq!(listing.json(), json!([]));
    let again = delete(&app, &alice, &format!("/api/v1/nodes/{file}")).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    let version = delete(&app, &alice, &format!("/api/v1/nodes/{file}/versions/1")).await;
    assert_eq!(version.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn binned_files_are_not_searchable_until_restored(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let file = upload(&app, &alice, &dir, "holiday.txt", "x").await;
    let found = app.get("/api/v1/search?q=holiday", &alice).await;
    assert_eq!(found.json().as_array().map(Vec::len), Some(1));

    delete(&app, &alice, &format!("/api/v1/nodes/{dir}")).await;

    let hidden = app.get("/api/v1/search?q=holiday", &alice).await;
    assert_eq!(hidden.json(), json!([]));
    // Catch-up indexes only the live folders: Lost & Found, never the recycle bin.
    let report = app.state.search.catch_up().await.expect("catch up");
    assert_eq!(report.names_indexed, 1);
    let hidden = app.get("/api/v1/search?q=holiday", &alice).await;
    assert_eq!(hidden.json(), json!([]));
    let bin = app.get("/api/v1/search?q=recycle", &alice).await;
    assert_eq!(bin.json(), json!([]));

    let restored = restore(&app, &alice, &[&file], "abort").await;
    assert_eq!(restored.status, StatusCode::OK);

    let hits = app.get("/api/v1/search?q=holiday", &alice).await.json();
    assert_eq!(hits.as_array().map(Vec::len), Some(1));
    assert_eq!(hits[0]["node"]["id"], file);
    assert_eq!(hits[0]["location"], json!(["Lost & Found"]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn restoring_returns_the_moved_files_and_the_target(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let file = upload(&app, &alice, &dir, "a.txt", "one").await;
    app.upload(&alice, &format!("name=a.txt&parent_id={dir}"), "two")
        .await;
    delete(&app, &alice, &format!("/api/v1/nodes/{dir}")).await;
    let found = lost_and_found(&app, &alice).await;

    let response = restore(&app, &alice, &[&file], "abort").await;

    assert_eq!(response.status, StatusCode::OK);
    let body = response.json();
    assert_eq!(body["moved"][0]["id"], file);
    assert_eq!(body["moved"][0]["parent_id"], found);
    assert_eq!(body["moved"][0]["version"], 2);
    assert_eq!(body["target"]["id"], found);
    assert_eq!(body["skipped"], json!([]));
    assert_eq!(body["conflicts"], json!([]));
    assert_eq!(app.get("/api/v1/trash", &alice).await.json(), json!([]));
    let versions = app
        .get(&format!("/api/v1/nodes/{file}/versions"), &alice)
        .await;
    assert_eq!(versions.json().as_array().map(Vec::len), Some(1));
    let content = app
        .get(&format!("/api/v1/files/{file}/content"), &alice)
        .await;
    assert_eq!(content.body, b"two");
}

#[sqlx::test(migrations = "../../migrations")]
async fn restore_conflicts_follow_the_chosen_mode(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let found = lost_and_found(&app, &alice).await;
    let live = upload(&app, &alice, &found, "a.txt", "live").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let binned = upload(&app, &alice, &dir, "a.txt", "binned").await;
    delete(&app, &alice, &format!("/api/v1/nodes/{dir}")).await;

    let aborted = restore(&app, &alice, &[&binned], "abort").await;
    assert_eq!(aborted.status, StatusCode::OK);
    let body = aborted.json();
    assert_eq!(body["moved"], json!([]));
    assert_eq!(body["conflicts"][0]["path"], "/dir/a.txt");
    assert_eq!(body["conflicts"][0]["source"]["id"], binned);
    assert_eq!(body["conflicts"][0]["existing"]["id"], live);
    assert_eq!(
        app.get("/api/v1/trash", &alice)
            .await
            .json()
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let skipped = restore(&app, &alice, &[&binned], "skip").await.json();
    assert_eq!(skipped["skipped"], json!([binned]));
    assert_eq!(skipped["moved"], json!([]));
    assert_eq!(
        app.get("/api/v1/trash", &alice)
            .await
            .json()
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let unchanged = app
        .get(&format!("/api/v1/files/{live}/content"), &alice)
        .await;
    assert_eq!(unchanged.body, b"live");

    let overridden = restore(&app, &alice, &[&binned], "override").await.json();
    assert_eq!(overridden["moved"][0]["id"], live);
    assert_eq!(overridden["moved"][0]["version"], 2);
    assert_eq!(app.get("/api/v1/trash", &alice).await.json(), json!([]));
    let merged = app
        .get(&format!("/api/v1/files/{live}/content"), &alice)
        .await;
    assert_eq!(merged.body, b"binned");
}

#[sqlx::test(migrations = "../../migrations")]
async fn restore_defaults_to_aborting_on_conflicts(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let found = lost_and_found(&app, &alice).await;
    upload(&app, &alice, &found, "a.txt", "live").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let binned = upload(&app, &alice, &dir, "a.txt", "binned").await;
    delete(&app, &alice, &format!("/api/v1/nodes/{binned}")).await;

    let response = app
        .json(
            Method::POST,
            "/api/v1/trash/restore",
            Some(&alice),
            json!({ "ids": [binned] }),
        )
        .await;

    assert_eq!(
        response.json()["conflicts"].as_array().map(Vec::len),
        Some(1)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn lost_and_found_cannot_be_deleted_but_can_be_renamed(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let found = lost_and_found(&app, &alice).await;

    let response = delete(&app, &alice, &format!("/api/v1/nodes/{found}")).await;
    assert_eq!(response.status, StatusCode::CONFLICT);
    assert_eq!(response.json()["error"], "conflict");
    assert_eq!(response.json()["message"], "Lost & Found cannot be deleted");

    let outer = folder(&app, &alice, "outer", None).await;
    let moved = app
        .json(
            Method::PATCH,
            &format!("/api/v1/nodes/{found}"),
            Some(&alice),
            json!({ "parent_id": outer }),
        )
        .await;
    assert_eq!(moved.status, StatusCode::OK);
    let ancestor = delete(&app, &alice, &format!("/api/v1/nodes/{outer}")).await;
    assert_eq!(ancestor.status, StatusCode::CONFLICT);

    let renamed = app
        .json(
            Method::PATCH,
            &format!("/api/v1/nodes/{found}"),
            Some(&alice),
            json!({ "name": "Found" }),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK);
    assert_eq!(renamed.json()["name"], "Found");
    let still = delete(&app, &alice, &format!("/api/v1/nodes/{found}")).await;
    assert_eq!(still.status, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_recycle_bin_folder_is_invisible_to_the_node_api(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let trash = app
        .state
        .db
        .special_folders(user_id(&app, &alice).await)
        .await
        .expect("folders")
        .trash_id;
    let other = folder(&app, &alice, "other", None).await;
    let file = upload(&app, &alice, &other, "a.txt", "x").await;

    assert_eq!(root_names(&app, &alice).await, ["Lost & Found", "other"]);
    let responses = [
        app.get(&format!("/api/v1/nodes/{trash}"), &alice).await,
        app.get(&format!("/api/v1/nodes?parent_id={trash}"), &alice)
            .await,
        app.get(&format!("/api/v1/nodes/{trash}/ancestors"), &alice)
            .await,
        delete(&app, &alice, &format!("/api/v1/nodes/{trash}")).await,
        delete(&app, &alice, &format!("/api/v1/trash/{trash}")).await,
        app.json(
            Method::PATCH,
            &format!("/api/v1/nodes/{trash}"),
            Some(&alice),
            json!({ "name": "mine" }),
        )
        .await,
        app.json(
            Method::PATCH,
            &format!("/api/v1/nodes/{file}"),
            Some(&alice),
            json!({ "parent_id": trash }),
        )
        .await,
        app.upload(&alice, &format!("name=x.txt&parent_id={trash}"), "x")
            .await,
        app.json(
            Method::POST,
            "/api/v1/folders",
            Some(&alice),
            json!({ "name": "x", "parent_id": trash }),
        )
        .await,
        restore(&app, &alice, &[&trash.to_string()], "abort").await,
    ];
    let statuses = responses.iter().map(|r| r.status).collect::<Vec<_>>();
    assert_eq!(statuses, vec![StatusCode::NOT_FOUND; responses.len()]);
    assert_eq!(app.get("/api/v1/trash", &alice).await.json(), json!([]));
}

#[sqlx::test(migrations = "../../migrations")]
async fn purging_deletes_one_file_and_emptying_deletes_the_rest(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let one = upload(&app, &alice, &dir, "one.txt", "1").await;
    let two = upload(&app, &alice, &dir, "two.txt", "2").await;
    let three = upload(&app, &alice, &dir, "three.txt", "3").await;
    delete(&app, &alice, &format!("/api/v1/nodes/{dir}")).await;

    let purged = delete(&app, &alice, &format!("/api/v1/trash/{one}")).await;
    assert_eq!(purged.status, StatusCode::NO_CONTENT);
    let again = delete(&app, &alice, &format!("/api/v1/trash/{one}")).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
    let listing = app.get("/api/v1/trash", &alice).await.json();
    let mut left = listing
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item["node"]["id"].as_str().expect("id").to_owned())
        .collect::<Vec<_>>();
    left.sort();
    let mut expected = vec![two.clone(), three.clone()];
    expected.sort();
    assert_eq!(left, expected);

    let emptied = delete(&app, &alice, "/api/v1/trash").await;
    assert_eq!(emptied.status, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/v1/trash", &alice).await.json(), json!([]));
    let gone = restore(&app, &alice, &[&two], "abort").await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn purging_a_live_file_is_not_found(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let dir = folder(&app, &alice, "dir", None).await;
    let file = upload(&app, &alice, &dir, "a.txt", "x").await;
    for id in [&file, &dir] {
        let response = delete(&app, &alice, &format!("/api/v1/trash/{id}")).await;
        assert_eq!(response.status, StatusCode::NOT_FOUND);
    }
    let still = app.get(&format!("/api/v1/nodes/{file}"), &alice).await;
    assert_eq!(still.status, StatusCode::OK);
}

#[sqlx::test(migrations = "../../migrations")]
async fn other_users_cannot_see_or_touch_binned_files(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let bob = app.user_token("bob").await;
    let file = app.upload(&alice, "name=secret.txt", "secret").await;
    let id = id_of(&file);
    delete(&app, &alice, &format!("/api/v1/nodes/{id}")).await;

    assert_eq!(app.get("/api/v1/trash", &bob).await.json(), json!([]));
    let attempts = [
        restore(&app, &bob, &[&id], "override").await,
        delete(&app, &bob, &format!("/api/v1/trash/{id}")).await,
    ];
    let statuses = attempts.iter().map(|r| r.status).collect::<Vec<_>>();
    assert_eq!(statuses, vec![StatusCode::NOT_FOUND; attempts.len()]);
    let emptied = delete(&app, &bob, "/api/v1/trash").await;
    assert_eq!(emptied.status, StatusCode::NO_CONTENT);

    let listing = app.get("/api/v1/trash", &alice).await.json();
    assert_eq!(listing.as_array().map(Vec::len), Some(1));
    let alice_id = user_id(&app, &alice).await;
    assert!(app.state.db.special_folders(alice_id).await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn bin_changes_are_announced_to_the_owner(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    let mut events = app.state.hub.subscribe(user_id(&app, &alice).await);
    let file = id_of(&app.upload(&alice, "name=a.txt", "x").await);
    let mut next = || {
        let mut seen = Vec::new();
        while let Ok(event) = events.try_recv() {
            seen.push(event);
        }
        seen
    };
    next();

    delete(&app, &alice, &format!("/api/v1/nodes/{file}")).await;
    let deleted = next();
    assert!(matches!(deleted.first(), Some(Event::NodeDeleted { .. })));
    assert_eq!(deleted.last(), Some(&Event::TrashChanged));

    restore(&app, &alice, &[&file], "abort").await;
    let restored = next();
    assert!(
        restored
            .iter()
            .any(|e| matches!(e, Event::NodeUpdated { .. }))
    );
    assert_eq!(restored.last(), Some(&Event::TrashChanged));

    delete(&app, &alice, &format!("/api/v1/nodes/{file}")).await;
    next();
    delete(&app, &alice, "/api/v1/trash").await;
    assert_eq!(next(), [Event::TrashChanged]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_names_can_be_deleted_and_reused(pool: PgPool) {
    let app = setup(pool).await;
    let alice = app.user_token("alice").await;
    for content in ["one", "two"] {
        let file = app.upload(&alice, "name=a.txt", content).await;
        assert_eq!(file.status, StatusCode::CREATED);
        let response = delete(&app, &alice, &format!("/api/v1/nodes/{}", id_of(&file))).await;
        assert_eq!(response.status, StatusCode::NO_CONTENT);
    }
    let trash: Value = app.get("/api/v1/trash", &alice).await.json();
    assert_eq!(trash.as_array().map(Vec::len), Some(2));
    assert_eq!(trash[0]["original_path"], "/a.txt");
    let live = app.upload(&alice, "name=a.txt", "three").await;
    assert_eq!(live.status, StatusCode::CREATED);
}
