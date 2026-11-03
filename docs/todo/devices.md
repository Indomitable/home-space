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

## Done

Steps 1 and 2 below (migration `0010_devices.sql`):

- **Identity.**
  - Each client makes up a UUID once and keeps it: `~/.local/share/home-space/device-id` on the
    desktop, `homespace.device_id` in localStorage on the web.
  - The client sends `device: {id, name, platform}` on `POST /auth/login` and
    `POST /auth/oidc/exchange`. The name is the host name on the desktop and the browser's name
    on the web. Older clients leave the field out and sign in as before, without a device.
- **Schema.**
  - `devices` is keyed by `(user_id, id)`, so several users of one install each get their own
    row and nobody can claim another user's device. (The first design refused an id already
    registered to another user with `409`; that would have broken switching users on one desktop.)
  - `sessions.device_id` references it. `uploads.device_id` has no foreign key: devices are only
    deleted with their user.
- **Sign-in.**
  - The device is registered, or marked as seen. A known device keeps the name the user gave it.
  - `last_seen_at` is updated on token refresh, at most once an hour.
  - `AuthUser` carries `device_id`, read from the session row the token check already loads.
- **Uploads.** `POST /uploads` records the device.
- **API.**
  - `GET /devices` lists the user's devices with `signed_in`, `current` and their number of
    unfinished `uploads`.
  - `PATCH /devices/{id}` renames one.
  - `DELETE /devices/{id}` ends its sessions and cancels its unfinished uploads. The row stays, so
    the device is still named in history.
  - `GET /uploads?this_device=true` lists the uploads of the calling device, to clean up after
    its local database is lost.
- **Signing out.** Signing out on the device itself (`/auth/logout`) does not cancel its
  uploads. The desktop keeps its saved upload sessions and continues them after signing in again.
- **API client.** `Client::with_device`, `devices`, `rename_device`, `sign_out_device`, `uploads`.

## Left

### Tagging changes (step 3)

```sql
ALTER TABLE node_changes ADD COLUMN device_id UUID;   -- no foreign key, like owner_id
```

- The API runs `SET LOCAL homespace.device_id = '<id>'` in each write transaction.
  `record_node_change()` reads it with `current_setting('homespace.device_id', true)` and stores
  it. Writes without it (CLI, background jobs) record `NULL`.
  - Every write in `db::nodes` has to take the device, and so do `FileService` methods.
- **`GET /changes`** returns `device_id` with each entry. With journal compaction, only the latest
  writer of each node is known, which is what "edited on" needs.
- Echo skipping does not need it, and is done: sync skips a live event when it already holds
  the node as the event shows it. Skipping by device would be wrong, since the desktop's file
  browser also writes as this device, and sync has not applied those writes yet.

### Client features (step 4)

- **Desktop.**
  - A Devices section in settings: the list, rename, sign out.
  - Conflict copies are named after the other device ("report (Laptop).docx").
  - On macOS the host name falls back to "Desktop", since neither `/etc/hostname` nor `HOSTNAME`
    is there. Read the computer name instead.
- **Web.** The same Devices list, on an account page, which the web client does not have yet.
- **Android.** Registers on first sign-in, so its uploads show as "from Phone".

## Not in scope

- Per-device permissions or limits.
- Pushing changes from one device to a chosen other device. Every device still syncs from the
  journal.
