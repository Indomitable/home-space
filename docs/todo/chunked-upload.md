# Resumable chunked upload

## Why

Today a file is uploaded as a single streamed `XmlHttpRequest` POST
(`clients/web/src/upload.rs` → `BlobStore::stage` in `server/crates/files/src/store.rs`).
The server streams the body into a temp file, hashes it with SHA-256, enforces
`max_upload_bytes`, then renames it into place. This is enough while the reverse proxy
accepts large bodies.

Chunking becomes necessary when:

1. A proxy caps the request size, e.g. Cloudflare (including Tunnel) allows 100 MB per
   request on the free plan.
2. Very large files are uploaded over unreliable links (mobile, Wi-Fi), where resuming
   saves real time.
3. A proxy or the server times out long requests.

## Current setup: nginx for the single-request upload

Raise the limit on the upload route only, and keep the default 1 MB elsewhere:

```nginx
location /api/v1/files {
    proxy_pass http://127.0.0.1:8080;
    client_max_body_size    10g;    # match HOMESPACE_MAX_UPLOAD_BYTES (0 = unlimited)
    proxy_request_buffering off;    # stream to the app instead of spooling to disk first
    proxy_http_version      1.1;
    client_body_timeout     300s;   # resets on every read, so slow links don't time out
    proxy_send_timeout      300s;
    proxy_read_timeout      300s;   # covers the final hash + rename + DB insert
}
```

## Design (tus-like)

- `POST /uploads {name, parent_id, size}` returns `upload_id`. The server creates a temp
  file in `<root>/tmp`.
- `PATCH /uploads/{id}` with header `Upload-Offset: N` and a raw body of about 8–32 MB,
  sent with the existing XHR code so progress keeps working. The server appends only if
  `N` equals the current size, then returns the new offset.
- `HEAD /uploads/{id}` returns the current offset, so the client can resume after an error
  or a page reload. The client keys the upload by name + size + lastModified.
- When `offset == size`, the server hashes the file and commits it through the existing
  `BlobStore` rename path and `FileService` to create the node.
- A cleanup job removes stale temp files.

## What not to repeat from the old ASP.NET version

- Multipart framing: every chunk was parsed as a form.
- No way to resume: the upload id lived only in memory and the server could not report its
  offset.
- `fileHash` was always `"-"`, so there was no integrity check.
- Chunks were sent one at a time with no per-chunk retry.

## Verification

- `cargo test --workspace`: an upload split into chunks gives the same SHA-256 as a single
  POST.
- A wrong `Upload-Offset` returns 409.
- Resuming after an interrupted PATCH continues from the server's offset.
- Manual: upload a file over 1 GB, cut the network halfway, and check that it resumes.
