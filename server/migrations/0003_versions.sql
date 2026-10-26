UPDATE nodes SET mime = 'inode/directory' WHERE kind = 'folder';
UPDATE nodes SET mime = 'application/octet-stream' WHERE mime IS NULL;
ALTER TABLE nodes ALTER COLUMN mime SET NOT NULL;
ALTER TABLE nodes ADD COLUMN version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1);

ALTER TABLE nodes DROP CONSTRAINT nodes_unique_name;
ALTER TABLE nodes ADD CONSTRAINT nodes_unique_name
    UNIQUE NULLS NOT DISTINCT (owner_id, parent_id, kind, name);

CREATE TABLE file_versions (
    id          UUID PRIMARY KEY,
    node_id     UUID NOT NULL REFERENCES nodes (id) ON DELETE CASCADE,
    owner_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    version     INTEGER NOT NULL CHECK (version >= 1),
    size        BIGINT NOT NULL CHECK (size >= 0),
    sha256      BYTEA NOT NULL,
    mime        TEXT NOT NULL,
    modified_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT file_versions_unique UNIQUE (node_id, version)
);
