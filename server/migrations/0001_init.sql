CREATE TABLE users (
    id            UUID PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE CHECK (length(username) BETWEEN 1 AND 64),
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE refresh_tokens (
    id         UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id  UUID NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX refresh_tokens_family_idx ON refresh_tokens (family_id);

CREATE TABLE nodes (
    id         UUID PRIMARY KEY,
    owner_id   UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    parent_id  UUID REFERENCES nodes (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL CHECK (kind IN ('folder', 'file')),
    name       TEXT NOT NULL CHECK (
        length(name) BETWEEN 1 AND 255
        AND position('/' IN name) = 0
        AND position(E'\\' IN name) = 0
        AND name NOT IN ('.', '..')
    ),
    size       BIGINT NOT NULL DEFAULT 0 CHECK (size >= 0),
    sha256     BYTEA,
    mime       TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT nodes_unique_name UNIQUE NULLS NOT DISTINCT (owner_id, parent_id, name)
);

CREATE INDEX nodes_owner_parent_idx ON nodes (owner_id, parent_id);
