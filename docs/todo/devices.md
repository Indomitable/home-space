# Registered devices

## Problem

The server does not know which device does what. Several clients of one user (two desktops, the
web client, later the Android app) look the same to it:

- An unfinished chunked upload belongs to a user, not to a device. Only the client that started it
  knows its id. A device that is signed out or lost leaves its uploads to the 24 h expiry.
- The change journal does not say which device made a change. A client re-reads its own uploads
  when it catches up, and the UI cannot say "edited on Laptop".
- `sessions` (migration `0007`) is "one signed-in device", but its id changes with every sign-in.
  It cannot name a device over time, and there is no device list for the user.

## Proposal

### Identity

- On first run each client generates a UUID and keeps it: in the desktop sync database's `meta`
  table, in localStorage for the web client, and in DataStore for Android.
- The client sends it on sign-in: `device_id`, `device_name` (hostname or browser) and `platform`
  on `POST /auth/login` and in the OIDC exchange.
- The device id only labels where something came from. Access still rests on the session. The
  server accepts a device id only together with a verified sign-in, and only for the user signing
  in.

### Schema

```sql
CREATE TABLE devices (
    id           UUID PRIMARY KEY,
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    platform     TEXT NOT NULL,          -- desktop-linux, desktop-macos, web, android, ...
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE sessions     ADD COLUMN device_id UUID REFERENCES devices (id) ON DELETE CASCADE;
ALTER TABLE uploads      ADD COLUMN device_id UUID REFERENCES devices (id) ON DELETE SET NULL;
ALTER TABLE node_changes ADD COLUMN device_id UUID;   -- no foreign key, like owner_id
```

- A device id already registered to another user is refused (`409`), so one user cannot claim
  another user's device.
- `last_seen_at` is updated on token refresh, at most once an hour.
- Access tokens carry the session, and the session carries the device. The `AuthUser` extractor
  exposes `device_id`.

### Tagging

- **Uploads:** `POST /uploads` and single-request uploads record the device. Signing a device out
  or deleting it cancels its unfinished uploads straight away.
- **Changes:** the API runs `SET LOCAL homespace.device_id = '<id>'` in each write transaction.
  `record_node_change()` reads it with `current_setting('homespace.device_id', true)` and stores
  it. Writes without it (CLI, background jobs) record `NULL`.
- **`GET /changes`** returns `device_id` with each entry. With journal compaction, only the latest
  writer of each node is known, which is what echo skipping and "edited on" need.

### API

| Method | Path | Purpose |
|---|---|---|
| GET | `/devices` | The user's devices: name, platform, last seen, current one marked, unfinished uploads. |
| PATCH | `/devices/{id}` | Rename. |
| DELETE | `/devices/{id}` | Sign out: delete its sessions and cancel its uploads. The row stays, so the journal still names it. |
| GET | `/uploads?device=me` | The unfinished uploads of this device, to clean up after its local database is lost. |

### Clients

- **Desktop:** a Devices section in settings. When catching up, sync skips entries made by this
  device whose content it already has, by matching `device_id` and the base's hash. Conflict copies
  are named after the other device ("report (Laptop).docx").
- **Web:** the same Devices list in the account page.
- **Android:** registers on first sign-in, so its uploads show as "from Phone".

## Order

1. `devices` table, registration on sign-in, `device_id` on sessions, `GET` and `DELETE /devices`.
2. Tag uploads, and cancel them when the device is signed out.
3. Tag changes through the session setting, and return `device_id` from `/changes`.
4. Client features: echo skipping, "edited on", conflict copy names, the device list in the UI.

## Not in scope

- Per-device permissions or limits.
- Pushing changes from one device to a chosen other device. Every device still syncs from the
  journal.
