# Android app (upload from the phone)

## Problem

Photos, downloads and documents pile up on phones, and nothing copies them to home-space. The
README lists mobile as planned, but no client exists.

## Scope

An upload-only app. The user picks folders on the phone, and the app uploads new and changed files
from them to a folder on the server. It does not download, and it does not mirror deletes or edits
made on the server back to the phone. It is not a full file browser.

## Proposal

### Stack

- Kotlin, Jetpack Compose (Material 3), a single-activity app in `clients/android` (Gradle).
- Ktor client or OkHttp + kotlinx.serialization for the API. The Rust `api-client` is not reused;
  the wire types in `shared/protocol` are copied into Kotlin data classes by hand. Generating them
  from an OpenAPI spec can come later.
- WorkManager for background uploads (constraints: network, optional "Wi-Fi only" and "while
  charging"), Room for local state, DataStore for settings.
- Tokens in EncryptedSharedPreferences / Android Keystore.

### Sign in

- Password login via `POST /auth/login`, refresh via `/auth/refresh` (rotating, so serialize the
  refreshes the way `api-client` does).
- SSO: open `/auth/oidc/start` in a Custom Tab with an app-link or custom-scheme redirect, then
  call `/auth/oidc/exchange` with the PKCE verifier. The server's allowed client redirects need an
  entry for the app's scheme.
- The server URL is typed in on the first screen; `GET /auth/providers` decides which buttons
  appear.

### Tracked folders

- The user adds folders through the Storage Access Framework (`ACTION_OPEN_DOCUMENT_TREE`, with
  persisted URI permissions). This avoids broad storage permissions. A shortcut for the camera
  folder (DCIM) uses MediaStore and `READ_MEDIA_IMAGES` / `READ_MEDIA_VIDEO`.
- Each tracked folder maps to a target folder on the server (picked with a small folder browser
  over `GET /nodes`). Subfolders are created on the server as needed (`POST /folders`).
- Room table `tracked_files(folder_id, relative_path, size, modified_at, sha256?, remote_node_id,
  state)` records what has been uploaded. A file is uploaded again when its size or mtime changes.
  Since files are never deleted on the server, the user can free space on the phone.

### Uploading

- Use the chunked upload API (`POST /uploads`, `PATCH /uploads/{id}` with `Upload-Offset`) so large
  videos survive network changes and app restarts. Keep the upload id in Room and resume from
  `HEAD /uploads/{id}`.
- Pass `modified_at` from the file so the server keeps the original date.
- Name clashes: an existing file at the same path with different content becomes a new version
  (the server's existing behaviour), and identical content is skipped.
- Detect new files with a periodic WorkManager job (e.g. every 15 min) plus a `ContentObserver` on
  MediaStore for quick camera uploads, and a manual "Sync now" button.
- A foreground-service notification shows progress for long runs.

### Screens

1. Server + sign in
2. Tracked folders: list with last sync, pending count, and an add button
3. Folder settings: server target, Wi-Fi only, charging only, include subfolders, file-type
   filter
4. Upload queue / history with errors and retry

## Open questions

- Should a file deleted on the phone stay on the server? (Yes for now: upload-only.)
- Minimum Android version: API 26+ (Android 8)?
- Hash files before uploading (costly on large videos) or trust size + mtime only?
- Should the storage-limits work (`storage-limits.md`) be done first, so the app can warn before
  filling the disk?
