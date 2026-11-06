-- Predefined properties of a node, at most one row each; most nodes have none. A new property is
-- a new column.
CREATE TABLE node_properties (
    node_id       UUID PRIMARY KEY REFERENCES nodes (id) ON DELETE CASCADE,
    -- The name of the device that last auto-uploaded the file. The name rather than the id,
    -- because it must outlive the device; a later rename of the device does not change it.
    source_device TEXT
);

-- How a finished upload is stored; see `CreateUpload`.
ALTER TABLE uploads ADD COLUMN keep_both BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE uploads ADD COLUMN auto_upload BOOLEAN NOT NULL DEFAULT false;
