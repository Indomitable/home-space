# Resumable chunked upload

Files larger than 32 MiB (`api_client::RESUMABLE_THRESHOLD`) are uploaded in chunks, so no
single request exceeds a proxy's body limit. Cloudflare, including Tunnel, allows 100 MB per
request on the free plan. Chunks also let an upload resume after a dropped connection.
Smaller files still go up as one streamed request (`POST /api/v1/files` or
`PUT /api/v1/files/{id}/content`).

## Protocol

| Request | Purpose |
|---|---|
| `POST /api/v1/uploads` `{name, parent_id?, size, modified_at?, mime?, node_id?}` | Starts an upload and returns `201 {id, offset, size, chunk_size}`. When `node_id` is set, the upload replaces that file's content, like `PUT /files/{id}/content`. |
| `PATCH /api/v1/uploads/{id}` with the `Upload-Offset: N` header and a raw body of at most `chunk_size` bytes | Appends the body only if `N` equals the bytes already received, then returns `{offset}`. The chunk that reaches `size` stores the file and returns `{offset, node}` with `201` (new file) or `200` (new version). |
| `GET` / `HEAD /api/v1/uploads/{id}` | Returns the current offset, in the body and in the `Upload-Offset` header. |
| `DELETE /api/v1/uploads/{id}` | Cancels the upload. |

Error responses:

| Status | Cause |
|---|---|
| `409` | Wrong offset; the `Upload-Offset` header carries the right one. |
| `413` | A `size` above `HOMESPACE_MAX_UPLOAD_BYTES`, a chunk larger than `chunk_size`, or a chunk that goes past `size`. |
| `404` | The upload is unknown or belongs to another user. |

## Server

- An upload is a row in the `uploads` table plus a file at `<storage_root>/uploads/<id>`.
- The length of that file is the offset, so chunks only touch the row's `updated_at`.
- Bytes received before a dropped connection are kept, and the client continues from them.
- The SHA-256 is computed as bytes arrive, so finishing a large upload does not reread it. After a server restart or a failed write, the hash is rebuilt from the file when the upload completes.
- Completing an upload goes through the same `BlobStore` rename path and `FileService::finish_store` as a single-request upload. Versioning, search indexing and change events therefore behave the same.
- Every hour, the server removes uploads that have received nothing for `HOMESPACE_UPLOAD_EXPIRY_HOURS` (24 by default). It also removes leftover staged files of interrupted single-request uploads.

Settings:

| Variable | Default | |
|---|---|---|
| `HOMESPACE_MAX_UPLOAD_BYTES` | 10737418240 (10 GiB) | Largest file, checked against `size` when an upload starts. |
| `HOMESPACE_UPLOAD_CHUNK_BYTES` | 33554432 (32 MiB) | Largest chunk. Keep it below the proxy's request limit; it does not limit the file size. |
| `HOMESPACE_UPLOAD_EXPIRY_HOURS` | 24 | Idle time after which an unfinished upload is deleted. |

## Clients

- **Web** (`clients/web/src/upload.rs`) slices the `File` and sends each chunk with the same XHR code, so progress keeps working. The upload id is saved in localStorage under the folder, name, size and `lastModified`, so dropping the same file again continues from the server's offset, even after a page reload.
- **Desktop**, for both manual uploads and sync, uses `Client::upload_resumable`. It reads each chunk from disk and, after an error, continues from the server's offset. An unfinished upload is cancelled when the client gives up; there is no resume across app restarts.
- Both retry a chunk up to 5 times with exponential backoff. They retry on network errors, proxy errors and `409`.

## Reverse proxy

nginx needs a body limit on `/api/v1/uploads` above the chunk size, plus the usual streaming
settings. See [`nginx/example.domain.com.conf`](nginx/example.domain.com.conf). Behind
Cloudflare, nothing needs to change as long as `HOMESPACE_UPLOAD_CHUNK_BYTES` stays below
100 MB.

## Known limits

- If the response to the last chunk is lost, the file has been stored but the upload no longer exists. The client then reports an error, and uploading again finds identical content and changes nothing.
- The desktop shows progress once per chunk, not continuously within a chunk.
