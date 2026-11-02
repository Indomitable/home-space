# home-space

## Server

Rust workspace in `server/`: `db` (sqlx/Postgres), `auth` (argon2 + JWT access/refresh tokens, optional OpenID Connect sign-in),
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
node is created or renamed; deleting a node removes its entry. Files in the recycle bin are not
indexed (nor is the bin itself) and are indexed again when restored. On startup the server indexes
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

Deleting moves files to the recycle bin instead of removing them. Deleting a file bins it;
deleting a folder bins every file below it (flat, each remembering its `original_path`) and
removes the folders, so folders are never in the bin. Binned files keep their id, content and
versions, and may share names: uniqueness only applies to live nodes, so a deleted file never
blocks its name. They disappear from listings, search and every node endpoint (404) until they
are restored or purged.

Every user has two special folders, created with the user. The recycle bin is a hidden root
folder (`.recycle-bin-<id>`) that no node endpoint lists or accepts (404). `Lost & Found` is a
root folder that restored files land in, directly under their own names. It can be renamed
or moved but not deleted, nor can a folder containing it (409).

Restoring (`POST /trash/restore`) and moving several nodes (`POST /nodes/move`) take an
`on_conflict` mode for files whose name is taken at the destination, or that share a name within
the selection (folders merge and never clash):

- `abort` (default): change nothing and return the `conflicts` (the path relative to what is
  moved, or the original path when restoring; `existing` is null for a clash within the selection).
- `override`: apply the move rule above; of several same-named binned files, the most recently
  deleted ends up current and the older ones become versions.
- `skip`: never modify the destination, and return the clashing files in `skipped`. A restore
  leaves them in the bin (all of them, for same-named files in the selection); a move puts them
  in the bin with their current path, so the source folders they leave behind are removed.

Previous versions live in `file_versions`; their blobs are under `<storage>/versions/`.
Folders have the MIME type `inode/directory`.

### API (`/api/v1`)

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/login`, `/auth/refresh`, `/auth/logout` | JWT access + rotating refresh tokens |
| GET | `/auth/providers` | how users can sign in: `{password, oidc: {name}?}` |
| GET / GET / POST | `/auth/oidc/start`, `/auth/oidc/callback`, `/auth/oidc/exchange` | single sign-on, see [Single sign-on](#single-sign-on) |
| GET | `/me` | current user: `{id, username, lost_and_found_id}` |
| GET | `/nodes?parent_id=` | list folder (root when omitted) |
| GET / PATCH / DELETE | `/nodes/{id}` | metadata / rename or move / delete (to the recycle bin; 409 for Lost & Found) |
| POST | `/nodes/move` | `{ids, parent_id, on_conflict?}` → `{moved, skipped, conflicts, target}`; `on_conflict` is `abort` (default: report clashes, change nothing), `override` (merge) or `skip` (destination untouched, clashing files go to the recycle bin) |
| POST | `/folders` | `{parent_id?, name}` |
| POST | `/files?name=&parent_id=` | raw-body streaming upload (201 new, 200 unchanged or new version) |
| GET / PUT | `/files/{id}/content` | download / replace content |
| GET | `/nodes/{id}/versions` | previous versions of a file, newest first |
| DELETE | `/nodes/{id}/versions/{version}` | delete a previous version |
| GET | `/files/{id}/versions/{version}/content` | download a previous version |
| GET | `/nodes/{id}/ancestors` | path from the root down to the node's parent |
| GET | `/trash` | binned files, newest first: `{node, original_path, deleted_at}` |
| POST | `/trash/restore` | `{ids, on_conflict?}` → `{moved, skipped, conflicts, target}`; restores into Lost & Found |
| DELETE | `/trash` | empty the recycle bin (permanent) |
| DELETE | `/trash/{id}` | permanently delete one binned file and its versions (404 unless binned) |
| GET | `/search?q=&limit=` | full-text search; every word matches as a prefix, hits carry their `location` |
| POST | `/tickets` | `{scope: "events"}`, `{scope: "download", node_id}` or `{scope: "download_version", node_id, version}` → 60 s ticket |
| GET | `/ws` | live `node_created` / `node_updated` / `node_deleted` / `trash_changed` / `resync` events |
| GET | `/health` | liveness + DB ping |

All node and trash endpoints return 404 for ids owned by someone else. Authenticate with `Authorization: Bearer <access_token>`.
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

The desktop client (`clients/desktop`, iced) runs with `cargo run -p desktop`. It keeps the
session in `~/.local/share/home-space/session.json`, so rebuilt dev binaries sign in again without
a keychain prompt; build with `--features keyring` to keep it in the system keyring instead
(macOS Keychain, Secret Service). On macOS the Dock
icon only comes from an app bundle: `cargo install cargo-bundle`, then
`cd clients/desktop && cargo bundle --release` builds `Home Space.app` under `target/release/bundle/osx`.
The icons in `clients/desktop/packaging` are rendered from `home-space.svg`.

## Single sign-on

Besides passwords, users can sign in through any OpenID Connect provider: Auth0, Keycloak,
Authentik, Zitadel and so on. home-space is a confidential client of the provider (authorization
code flow with PKCE); once the provider has vouched for the user, home-space issues its own
tokens, so the rest of the API works as before. Setting `HOMESPACE_OIDC_ISSUER` turns it on:

| Variable | |
|---|---|
| `HOMESPACE_PUBLIC_URL` | Address users reach the server at, e.g. `https://files.example.com`. Required. |
| `HOMESPACE_OIDC_ISSUER` | Issuer URL of the provider. Required. |
| `HOMESPACE_OIDC_CLIENT_ID`, `HOMESPACE_OIDC_CLIENT_SECRET` | The client registered at the provider. |
| `HOMESPACE_OIDC_NAME` | Label on the sign-in button (default `Single sign-on`). |
| `HOMESPACE_OIDC_REQUIRED_GROUP` | Only members of this group may sign in. Optional. |
| `HOMESPACE_OIDC_GROUPS_CLAIM` | ID token claim listing the groups (default `groups`). |
| `HOMESPACE_OIDC_SCOPES` | Default `openid profile email`. |
| `HOMESPACE_PASSWORD_LOGIN` | `false` allows only the provider. Default `true`. |

The issuer and public URL must use https (plain http is accepted for `localhost` only). Register
`<HOMESPACE_PUBLIC_URL>/api/v1/auth/oidc/callback` as the redirect URI at the provider.

- **Keycloak:** create a confidential client in your realm with the standard flow on; the issuer
  is `https://<host>/realms/<realm>`. For groups, add a *Group Membership* mapper (full path off)
  to the client's dedicated scope.
- **Auth0:** create a *Regular Web Application*; the issuer is `https://<tenant>.<region>.auth0.com/`
  (with the trailing slash). Auth0 only puts custom claims in tokens under a namespace, so for
  groups add an Action that sets e.g. `https://files.example.com/groups` on the ID token and
  point `HOMESPACE_OIDC_GROUPS_CLAIM` at it.

A user is created on first sign-in, named after `preferred_username`, then the e-mail's local
part, then the subject (a number is added if the name is taken). Accounts are matched on the
provider's stable subject id, never on e-mail. They have no password, so they cannot use password
login, and the CLI can still give them one (`user passwd`).

Desktop clients open the system browser and receive the result on a loopback port; the web client
is redirected to `/login/callback`. In both, the server hands back a 60 second, single-use code
that only the client holding the PKCE verifier can exchange for tokens.

To try it locally with Keycloak (user `demo` / password `demo`, in group `family`):

```sh
podman-compose --profile oidc up -d keycloak     # http://127.0.0.1:8180, admin / admin
cd server && HOMESPACE_PUBLIC_URL=http://127.0.0.1:8080 \
  HOMESPACE_OIDC_ISSUER=http://127.0.0.1:8180/realms/homespace \
  HOMESPACE_OIDC_CLIENT_ID=homespace HOMESPACE_OIDC_CLIENT_SECRET=homespace-dev-secret \
  HOMESPACE_OIDC_NAME=Keycloak HOMESPACE_OIDC_REQUIRED_GROUP=family cargo run -p app -- serve
```

The realm (`docs/keycloak/homespace-realm.json`) allows the redirect URIs of the server on 8080 and
of `trunk serve` on 8081; set `HOMESPACE_PUBLIC_URL` to the address the browser uses. Keycloak runs
on the host network name `127.0.0.1` here, so run the server on the host rather than in Compose.

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

The signing secret can also come from a file (`HOMESPACE_JWT_SECRET_FILE`, e.g. a Docker/Podman
secret). To rotate it, set the new value and move the old one to `HOMESPACE_JWT_SECRET_PREVIOUS`;
sessions keep working, and the old value can be removed once access tokens (15 minutes by default)
signed with it have expired.

Access tokens are only accepted while the session they were issued for exists in the database. A
session starts after a verified password or provider login and ends on logout, on refresh token
reuse, on a password change or when it expires. So the signing secret alone cannot forge access:
a token it signs for a session that does not exist is rejected, and logging out takes effect at
once rather than when the access token expires.

### Running on a server

`compose.prod.yaml` keeps the database and the blobs in directories you choose and runs both
containers as one host user, so only that user (and root) can read them:

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin homespace
sudo install -d -o homespace -g homespace -m 700 /srv/homespace/db /srv/homespace/files

cat > .env <<EOF
HOMESPACE_DB_DIR=/srv/homespace/db
HOMESPACE_DATA_DIR=/srv/homespace/files
HOMESPACE_UID=$(id -u homespace)
HOMESPACE_GID=$(id -g homespace)
HOMESPACE_JWT_SECRET=$(head -c 32 /dev/urandom | base64)
EOF
chmod 600 .env

docker compose -f compose.yaml -f compose.prod.yaml up -d --build
docker compose -f compose.yaml -f compose.prod.yaml run --rm app user add me
```

The directories must exist and be owned by that user before the first start (mode `700` keeps
everyone else out). With rootless Podman the uid is remapped, so own the directories through
`podman unshare chown` instead, or drop the `user:` lines and rely on the rootless user itself.
Change the default `homespace` database password in `compose.yaml` before exposing anything.

Postgres is published on `127.0.0.1:5433` (`HOMESPACE_DB_PORT`) and the app on `127.0.0.1:8080`
(`HOMESPACE_PORT`), so it does not clash with the `scripts/db.sh` database on 5432.
