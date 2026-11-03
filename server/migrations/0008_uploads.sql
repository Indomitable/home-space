-- A resumable upload in progress. Its bytes live in `<storage_root>/uploads/<id>`; the length of
-- that file is the upload's offset, so chunks never need a write here. The row is deleted once
-- the upload completes, is cancelled or expires.
CREATE TABLE uploads (
    id          UUID PRIMARY KEY,
    owner_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Folder of a new file (NULL is the root); ignored when `node_id` is set.
    parent_id   UUID,
    -- The existing file whose content the upload replaces.
    node_id     UUID,
    name        TEXT NOT NULL CHECK (
        length(name) BETWEEN 1 AND 255
        AND position('/' IN name) = 0
        AND position(E'\\' IN name) = 0
        AND name NOT IN ('.', '..')
    ),
    size        BIGINT NOT NULL CHECK (size >= 0),
    mime        TEXT,
    modified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX uploads_owner_idx ON uploads (owner_id);
CREATE INDEX uploads_updated_idx ON uploads (updated_at);
