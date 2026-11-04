# Object storage backends: S3 and Azure Blob

## Problem

All file content lives on the local disk under `HOMESPACE_STORAGE_ROOT`
(`server/crates/files/src/store.rs`). The server can only hold as much as that disk, the disk has
to be backed up separately from the database, and moving the server means moving the data.
Object storage (Amazon S3 and S3-compatible services such as MinIO, Cloudflare R2, Backblaze B2,
Hetzner Object Storage, or Azure Blob Storage) is cheap, grows without limit, is replicated by
the provider, and lets the server run on a small machine.

The operator should be able to choose where content is stored. The local file system stays the
default, so nothing changes for existing installs.

## What is stored today

`BlobStore` is the only code that touches content, and its API is small:

| Use | Methods | Local implementation |
|---|---|---|
| Single-request upload | `stage`, `commit`, `commit_version`, `discard` | write `tmp/<id>`, hash while writing, `rename` into place |
| Resumable upload | `create_partial`, `append`, `partial_len`, `hash_partial`, `adopt_partial`, `remove_partial` | `uploads/<id>` grows with every chunk; offset = file length |
| Download, search | `open_blob`, `open_version` | return a `tokio::fs::File` |
| Versions and merges | `archive`, `promote` | `rename` between `blobs/<owner>/<node>` and `versions/<owner>/<version>` |
| Delete | `remove`, `remove_version` | `remove_file` |
| Clean-up | `sweep` | list `tmp/` and `uploads/`, remove old files |

Three things in it do not carry over to object storage as they are:

1. **Rename.** `archive` and `promote` (the `BlobOp`s that `db::nodes` returns for overwrites
   and merges) are a free `rename` on disk. S3 and Azure have no rename: it is a server-side copy
   plus a delete, which takes time on large files, and a single S3 `CopyObject` stops at 5 GB.
2. **Append.** A resumable upload appends to a file and reads its length back as the offset.
   Objects are immutable; S3 multipart parts must be at least 5 MiB (except the last), and a part
   is all or nothing.
3. **`tokio::fs::File` in the API.** `download`, `download_version` and the search indexer get a
   file handle. They need a byte stream (with `Range`) instead.

## Proposal

### Immutable blobs addressed by id

Give every stored content its own id and never move it. The database points at blobs; changing
which node or version owns some content is a database update, not a storage operation.

```sql
ALTER TABLE nodes         ADD COLUMN blob_id UUID;   -- NULL for folders
ALTER TABLE file_versions ADD COLUMN blob_id UUID NOT NULL;

-- Content written but not yet referenced (an upload in flight), and content no longer referenced
-- (deleted, or replaced in a merge). A worker finishes both.
CREATE TABLE blob_journal (
    blob_id    UUID PRIMARY KEY,
    owner_id   UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    state      TEXT NOT NULL CHECK (state IN ('pending', 'delete')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- Object key: `blobs/<owner_id>/<blob_id>` under an optional prefix. The local layout stays the
  same for nodes, because the migration sets `blob_id = id` for existing files: their path does
  not change. Existing version files move once from `versions/<owner>/<version>` to
  `blobs/<owner>/<version>` (with `blob_id = id`), a `rename` on the same disk.
- `BlobOp::Archive` becomes "the new version row takes the node's `blob_id`, the node gets the
  new one". `BlobOp::Promote` becomes "`to.blob_id = from.blob_id`". `BlobOp::Remove` becomes a
  `blob_journal` row with `state = 'delete'`, written **in the same transaction** that drops the
  reference. `apply_blob_ops` goes away.
- Writing: insert a `pending` journal row, write the object, then the transaction that creates
  the node or version deletes the journal row. A crash in between leaves a `pending` row that the
  worker removes (with the object) after `HOMESPACE_UPLOAD_EXPIRY_HOURS`. No bucket listing is
  needed to find orphans.
- Deleting: the worker takes `delete` rows, checks that no `nodes`, `file_versions` or
  `transfer_items` row still references the blob, deletes the object, then the row. A failed
  delete is retried on the next run instead of being logged and forgotten as today.

This is worth doing on its own: today the `BlobOp`s run after the commit and a failure leaves the
database and the disk out of step (`"failed to apply blob operation"`). It also makes
deduplication by SHA-256 possible later (several rows, one blob), which the delete check already
handles.

### Storage trait

`BlobStore` becomes a thin wrapper over a backend, chosen at start-up:

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn put(&self, key: &Key, body: ByteStream, limits: Limits) -> Result<Written>;   // size, sha256
    async fn get(&self, key: &Key, range: Option<Range<u64>>) -> Result<Blob>;  // size + stream
    async fn delete(&self, key: &Key) -> Result<()>;                            // missing = Ok
    async fn head(&self, key: &Key) -> Result<Option<u64>>;
    // Resumable uploads, see below
    async fn start_parts(&self, key: &Key) -> Result<PartsId>;
    async fn put_part(&self, key: &Key, parts: &PartsId, index: u32, chunk: Bytes) -> Result<PartTag>;
    async fn finish_parts(&self, key: &Key, parts: &PartsId, tags: &[PartTag]) -> Result<()>;
    async fn abort_parts(&self, key: &Key, parts: &PartsId) -> Result<()>;
}
```

- Build it on the [`object_store`](https://crates.io/crates/object_store) crate (Apache Arrow).
  One dependency covers local files, S3 and S3-compatible services, Azure Blob, Google Cloud
  Storage and an in-memory store for tests. Its `MultipartStore` trait exposes upload ids and
  part ids, so a multipart upload can be resumed after a restart. The alternative, the official
  `aws-sdk-s3` and `azure_storage_blob` crates, means two large SDKs and two code paths.
- Keep our own `LocalBackend` (today's code) for the local file system rather than
  `object_store::local`, so local installs keep `rename`-based atomic writes, the existing
  resumable-upload files and `fsync` behaviour.
- `download` and `download_version` return a `Blob { size, stream }`. The API wraps the stream in
  `Body::from_stream`. Implement `Range` here once, for downloads, shares
  ([sharing.md](sharing.md)) and transfers ([send-to-device.md](send-to-device.md)).
- The search indexer reads through `tokio_util::io::SyncIoBridge` over the stream inside
  `spawn_blocking`, so `ContentProvider::extract(&mut dyn Read)` does not change. It already
  stops at `MAX_CONTENT_BYTES`; with a range request it fetches only that much.

### Uploads

- **Single request** (`POST /nodes`, `PUT /nodes/{id}/content`): stream into the backend while
  hashing and counting, as `stage` does now. Local: `tmp/` then `rename`. S3/Azure: a multipart
  upload (object_store's `WriteMultipart`) straight to `blobs/<owner>/<new blob_id>`; the object
  appears only when complete. Over the size limit or on error, abort it.
- **Resumable** (`/uploads`, [chunked-upload.md](../chunked-upload.md)): each chunk becomes one
  part.
  - New columns on `uploads`: `blob_id`, `parts_id` (the backend's multipart id), and a
    `upload_parts (upload_id, index, offset, size, tag)` table.
  - A part is all or nothing, so a chunk is buffered (memory up to the chunk size, or a spool file
    in `HOMESPACE_STAGING_DIR`) and sent with `put_part` only when it has fully arrived. An
    interrupted chunk is dropped and the offset stays at the last whole part. The protocol already
    tells the client the offset, so clients do not change.
  - S3 parts must be at least 5 MiB except the last. `HOMESPACE_UPLOAD_CHUNK_BYTES` must be at least
    5 MiB with an S3 backend (checked at start-up), and a non-final chunk smaller than that is
    refused with 409 and the current offset.
  - The SHA-256 can no longer be rebuilt by reading the partial file, because S3 parts cannot be
    read before completion. Store the hasher state after each part (`sha2` 0.11 implements
    `digest::SerializableState`, about 100 bytes) in `uploads.hash_state`. This also removes the
    re-read after a restart on the local backend.
  - Expiry and cancel call `abort_parts`. Also set a bucket lifecycle rule ("abort incomplete
    multipart uploads after 2 days") as a safety net; the README documents it.

### Configuration

| Setting | Meaning |
|---|---|
| `HOMESPACE_STORAGE` | `local` (default), `s3`, `azure` |
| `HOMESPACE_STORAGE_ROOT` | Local backend: as today |
| `HOMESPACE_STORAGE_URL` | `s3://bucket/prefix` or `az://container/prefix` |
| `HOMESPACE_S3_ENDPOINT`, `HOMESPACE_S3_REGION`, `HOMESPACE_S3_ACCESS_KEY_ID`, `HOMESPACE_S3_SECRET_ACCESS_KEY` (+ `_FILE`) | S3; the endpoint is for MinIO, R2, Hetzner and others; path-style addressing switch |
| `HOMESPACE_AZURE_ACCOUNT`, `HOMESPACE_AZURE_ACCESS_KEY` or `HOMESPACE_AZURE_SAS` (+ `_FILE`), or managed identity | Azure |
| `HOMESPACE_STAGING_DIR` | Spool for chunk buffers; defaults to a temp dir |

- Secrets take a `_FILE` variant like `HOMESPACE_JWT_SECRET_FILE`, for Docker/Podman secrets.
  The standard `AWS_*` and `AZURE_*` variables are also honoured, so instance roles work.
- At start-up the server writes, reads and deletes a small probe object, and refuses to start
  with a clear message when the bucket is unreachable or the credentials lack a permission.
- `GET /capabilities` gains nothing the clients need; the backend is an operator choice. The
  admin panel ([admin-panel.md](admin-panel.md)) can show it.

### Interaction with other features

- **Recycle bin**: trashing only changes `parent_id`; no content moves, on any backend. Empty
  trash and purge write `delete` journal rows in their transaction; the worker deletes the
  objects. Restore with Override reuses the merge code, which now updates `blob_id`s.
- **Versions**: overwrite and merge become pointer updates, so a 20 GB file is versioned as
  quickly on S3 as on disk.
- **Send to device** ([send-to-device.md](send-to-device.md)): the plan uses a hard link to
  `blobs/<owner>/<node>` as a free snapshot. With immutable blobs, `transfer_items.blob_id`
  simply references the node's current blob on every backend; the delete check above keeps it
  alive until the transfer finishes. Local items upload into a new blob like any upload. The
  `transfers/<id>/` directory is no longer needed. If this item ships first, change that plan's
  storage section before it is built.
- **Sharing** ([sharing.md](sharing.md)): share downloads go through the same `Blob` stream
  with `Range`. No change to the plan.
- **Storage limits** ([storage-limits.md](storage-limits.md)): `statvfs` free space only means
  something for the local backend. The backend reports `capacity() -> Option<u64>`; S3/Azure
  return `None`, so the free-space reserve is skipped there and the UI shows usage only (sum of
  `size` from the database, which already counts versions and the bin). Per-user quotas work
  the same on every backend.
- **Search**: content extraction reads through the backend (see the trait). Re-indexing
  everything on S3 downloads every indexed file once; `search reindex --all` warns about that.
- **Sync, changes, devices, auth**: not affected; they only touch the database.

### Moving existing data

- `homespace storage migrate --to <url>`: copies every blob referenced by `nodes` and
  `file_versions` from the configured backend to the target, verifies size (and SHA-256 for
  nodes), and prints a summary. It is resumable (skips objects whose size already matches) and
  works while the server runs; a final run with the server stopped picks up the last changes.
  The operator then switches `HOMESPACE_STORAGE` and restarts. The old data is left in place
  until the operator deletes it.
- `homespace storage verify`: lists blobs referenced in the database that are missing in the
  backend, and objects in the backend that nothing references.

### Download path

The server streams content from the backend to the client. That keeps authorisation, `Range`,
`Content-Disposition` and the device tagging in one place, and works behind any proxy. Later, an
optional `HOMESPACE_STORAGE_REDIRECT=true` can answer large downloads with a 302 to a short-lived
pre-signed URL (S3 presigned GET, Azure SAS), which saves the server's bandwidth. Uploads stay
proxied, because the server must hash and count the bytes.

### Testing

- Unit tests of `FileService`, uploads and search run against `object_store::memory::InMemory`
  and against `LocalBackend`, through one shared test suite for the trait.
- Integration tests against MinIO and Azurite (the Azure emulator) in a `compose.yaml` profile
  (`podman compose --profile storage run --rm test`), skipped when the services are not up.
- Fault tests: a crash between put and commit leaves a `pending` row that the worker cleans up;
  a failed delete is retried; a resumed upload after a restart produces the right SHA-256.

### Steps

1. Immutable blobs on the local backend: migration (`blob_id`, `blob_journal`, moving version
   files), `BlobOp` replaced by pointer updates and journal rows, the journal worker next to
   `expire_uploads`. No new backend yet, all existing tests green.
2. The `Backend` trait, `LocalBackend` from today's code, `Blob` stream in `download`,
   `download_version` and search, `Range` support on the download routes.
3. Resumable uploads on parts: `upload_parts`, persisted hash state, chunk buffering.
4. `object_store` S3 and Azure backends, configuration, start-up probe, MinIO/Azurite tests.
5. `storage migrate` and `storage verify` commands.
6. Docs: a `docs/object-storage.md` with set-up for AWS, MinIO, Hetzner and Azure (bucket
   policy, lifecycle rule, backups), settings in the README, `compose.yaml` and
   `server/.env.example`.
7. Later: pre-signed download redirects, deduplication by SHA-256.

## Open questions

- **Who chooses the storage**: one backend for the whole server, set by the operator (this
  plan), or per user ("bring your own bucket") set in the app? Per user needs encrypted
  credentials in the database, a settings UI, per-user migration, and moving files between users
  (send-to-device, future sharing copies) across backends.
- **Encryption**: rely on the provider's server-side encryption (SSE-S3/SSE-KMS, Azure default
  encryption), or add server-side encryption with our own key so the provider never sees
  plaintext? Our own key breaks pre-signed downloads and makes losing the key fatal.
- **Pre-signed downloads**: worth it at all for a home server? They bypass the server's
  bandwidth but expose the bucket host to clients and need CORS on the bucket for the web client.
- **Google Cloud Storage**: `object_store` supports it for free; offer it or only S3 and Azure?
- **Local cache**: keep a size-limited local cache of recently read blobs (thumbnails, search
  re-index) in front of a remote backend, or not until needed?
