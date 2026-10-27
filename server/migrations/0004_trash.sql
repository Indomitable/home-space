-- Deleted files stay in their owner's recycle bin (a hidden folder) until restored or purged.
-- `trashed_at` marks them and is the "deleted at" time; only files are ever binned.
ALTER TABLE nodes ADD COLUMN trashed_at TIMESTAMPTZ;
ALTER TABLE nodes ADD CONSTRAINT nodes_trash_kind CHECK (trashed_at IS NULL OR kind = 'file');

-- Binned files may share names, so uniqueness only covers live nodes.
ALTER TABLE nodes DROP CONSTRAINT nodes_unique_name;
CREATE UNIQUE INDEX nodes_unique_name
    ON nodes (owner_id, parent_id, kind, name) NULLS NOT DISTINCT
    WHERE trashed_at IS NULL;

CREATE TABLE trash_entries (
    node_id       UUID PRIMARY KEY REFERENCES nodes (id) ON DELETE CASCADE,
    original_path TEXT NOT NULL
);

CREATE TABLE user_folders (
    user_id           UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    trash_id          UUID NOT NULL UNIQUE REFERENCES nodes (id),
    lost_and_found_id UUID NOT NULL UNIQUE REFERENCES nodes (id)
);

-- Give existing users their folders, adopting a root folder that is already named
-- "Lost & Found".
DO $$
DECLARE
    member RECORD;
    trash UUID;
    found UUID;
BEGIN
    FOR member IN SELECT id FROM users LOOP
        trash := gen_random_uuid();
        INSERT INTO nodes (id, owner_id, parent_id, kind, name, mime)
        VALUES (trash, member.id, NULL, 'folder', '.recycle-bin-' || trash, 'inode/directory');

        SELECT id INTO found FROM nodes
        WHERE owner_id = member.id AND parent_id IS NULL AND kind = 'folder'
          AND name = 'Lost & Found';
        IF found IS NULL THEN
            found := gen_random_uuid();
            INSERT INTO nodes (id, owner_id, parent_id, kind, name, mime)
            VALUES (found, member.id, NULL, 'folder', 'Lost & Found', 'inode/directory');
        END IF;

        INSERT INTO user_folders (user_id, trash_id, lost_and_found_id)
        VALUES (member.id, trash, found);
    END LOOP;
END
$$;
