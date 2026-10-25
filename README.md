# home-space

## Server

Rust workspace in `server/`: `db` (sqlx/Postgres), `auth` (argon2 + JWT access/refresh tokens),
`files` (blobs on disk + ownership-scoped tree), `notifications` (per-user websocket events),
`api` (axum REST + `/ws`), `app` (the `homespace` binary).

```sh
cd server
cp .env.example .env            # set HOMESPACE_JWT_SECRET (>= 32 chars)
./scripts/db.sh up              # Postgres in podman (down | destroy | psql | logs)
cargo run -p app -- user add me # prompts for a password; there is no public signup
cargo run -p app -- serve       # http://127.0.0.1:8080/api/v1
```

Tests need the database running and `DATABASE_URL` set (`.env` is picked up):
`cargo test --workspace`.

### API (`/api/v1`)

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/login`, `/auth/refresh`, `/auth/logout` | JWT access + rotating refresh tokens |
| GET | `/me` | current user |
| GET | `/nodes?parent_id=` | list folder (root when omitted) |
| GET / PATCH / DELETE | `/nodes/{id}` | metadata / rename or move / delete |
| POST | `/folders` | `{parent_id?, name}` |
| POST | `/files?name=&parent_id=` | raw-body streaming upload |
| GET / PUT | `/files/{id}/content` | download / replace content |
| GET | `/ws` | live `node_created` / `node_updated` / `node_deleted` events |
| GET | `/health` | liveness + DB ping |

All node endpoints return 404 for nodes owned by someone else. Authenticate with `Authorization: Bearer <access_token>`.
