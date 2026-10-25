CREATE TABLE search_entries (
    node_id          UUID PRIMARY KEY REFERENCES nodes (id) ON DELETE CASCADE,
    owner_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name_tsv         TSVECTOR NOT NULL,
    content_tsv      TSVECTOR,
    content_provider TEXT,
    content_sha256   BYTEA,
    tsv              TSVECTOR GENERATED ALWAYS AS (
        setweight(name_tsv, 'A') || setweight(coalesce(content_tsv, ''::tsvector), 'B')
    ) STORED,
    indexed_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX search_entries_tsv_idx ON search_entries USING GIN (tsv);
CREATE INDEX search_entries_owner_idx ON search_entries (owner_id);
