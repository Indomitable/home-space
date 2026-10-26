# home-space

## Server

Rust workspace in `server/`: `db` (sqlx/Postgres), `auth` (argon2 + JWT access/refresh tokens),
`files` (blobs on disk + ownership-scoped tree), `search` (full-text index), `notifications`
(per-user websocket events), `api` (axum REST + `/ws`), `app` (the `homespace` binary).

```sh
cd server
cp .env.example .env            # set HOMESPACE_JWT_SECRET (>= 32 chars)
./scripts/db.sh up              # Postgres in podman (down | destroy | psql | logs)
cargo run -p app -- user add me # prompts for a password; there is no public signup
cargo run -p app -- serve       # http://127.0.0.1:8080/api/v1
```

Tests need the database running and `DATABASE_URL` set (`.env` is picked up):
`cargo test --workspace`.

### Search

Names are indexed in Postgres (`search_entries`: a weighted `tsvector` with a GIN index) when a
node is created or renamed; deleting a node removes its entry. On startup the server indexes
anything missing, and `cargo run -p app -- search reindex [--all]` does the same on demand
(`--all` drops the index first).

File content is indexed through `ContentProvider`s (`server/crates/search/src/provider.rs`). None
are registered yet, so only names are searched. To add one, implement the trait (`id`, `version`,
`accepts`, `extract`) and pass it to `SearchIndex::start` in `server/crates/app/src/main.rs`.
Extraction runs in a background worker after upload or replace, and the startup/`reindex` pass
re-extracts files whose content or provider version changed. Content matches rank below name
matches.

### Names and versions

Names are unique per folder and kind (case-sensitive), so a file and a folder may share a name.
Creating a folder or renaming onto a taken name is a 409. Everything else resolves collisions:

- Uploading (or replacing) a file whose name exists: identical content (size + SHA-256) is ignored
  (200); different content archives the old content as a version and bumps `version`.
- Moving a file onto a same-named file keeps the destination node. Identical content drops the source;
  different content archives the destination's content and the moved content becomes current. The
  source's older versions are appended to the history, and the dates become the earliest
  `created_at` and the latest `updated_at` of both.
- Moving a folder onto a same-named folder merges it recursively into the destination, applying
  the file rule to each conflict; the source folder is removed.

Previous versions live in `file_versions`; their blobs are under `<storage>/versions/`.
Folders have the MIME type `inode/directory`.

### API (`/api/v1`)

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/login`, `/auth/refresh`, `/auth/logout` | JWT access + rotating refresh tokens |
| GET | `/me` | current user |
| GET | `/nodes?parent_id=` | list folder (root when omitted) |
| GET / PATCH / DELETE | `/nodes/{id}` | metadata / rename or move / delete |
| POST | `/folders` | `{parent_id?, name}` |
| POST | `/files?name=&parent_id=` | raw-body streaming upload (201 new, 200 unchanged or new version) |
| GET / PUT | `/files/{id}/content` | download / replace content |
| GET | `/nodes/{id}/versions` | previous versions of a file, newest first |
| DELETE | `/nodes/{id}/versions/{version}` | delete a previous version |
| GET | `/files/{id}/versions/{version}/content` | download a previous version |
| GET | `/nodes/{id}/ancestors` | path from the root down to the node's parent |
| GET | `/search?q=&limit=` | full-text search; every word matches as a prefix, hits carry their `location` |
| POST | `/tickets` | `{scope: "events"}`, `{scope: "download", node_id}` or `{scope: "download_version", node_id, version}` → 60 s ticket |
| GET | `/ws` | live `node_created` / `node_updated` / `node_deleted` / `resync` events |
| GET | `/health` | liveness + DB ping |

All node endpoints return 404 for nodes owned by someone else. Authenticate with `Authorization: Bearer <access_token>`.
Browsers cannot send that header on a websocket or a plain download link, so `/ws?ticket=` and
`/files/{id}/content?ticket=` accept a short-lived, single-scope ticket instead.

## Shared protocol

`shared/protocol` holds the wire types (DTOs, events, error codes, route paths) used by the
server and every client. It has no server dependencies and compiles to WebAssembly.

## Clients

Rust workspace in `clients/`: `api-client` (typed HTTP client with serialized token refresh,
shared by all clients) and `web` (Leptos CSR app). Desktop (iced), CLI and mobile are planned.

Needs `rustup target add wasm32-unknown-unknown` and `cargo install trunk --locked`.

```sh
# development: server on :8080, web client with live reload on :8081 (proxies /api)
cd clients/web && trunk serve

# production: one origin serves both
cd clients/web && trunk build --release
cd ../../server && HOMESPACE_WEB_ROOT=../clients/web/dist cargo run -p app -- serve
```

The desktop client (`clients/desktop`, iced) runs with `cargo run -p desktop`. On macOS the Dock
icon only comes from an app bundle: `cargo install cargo-bundle`, then
`cd clients/desktop && cargo bundle --release` builds `Home Space.app` under `target/release/bundle/osx`.
The icons in `clients/desktop/packaging` are rendered from `home-space.svg`.

## Compose

`compose.yaml` runs Postgres and the app (API plus the web client, built by the `Dockerfile`) and
can run the whole test suite. Works with `podman compose` / `podman-compose`.

```sh
export HOMESPACE_JWT_SECRET=$(head -c 32 /dev/urandom | base64)   # 32+ characters
podman-compose up -d --build                  # http://127.0.0.1:8080
podman-compose run --rm app user add me      # prompts for a password (add -it if needed)
podman-compose --profile test run --rm test   # server + client tests against the compose Postgres
podman-compose down                           # add -v to also delete the database and files
```

Postgres is published on `127.0.0.1:5433` (`HOMESPACE_DB_PORT`) and the app on `127.0.0.1:8080`
(`HOMESPACE_PORT`), so it does not clash with the `scripts/db.sh` database on 5432.
