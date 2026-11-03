-- The device that made each change, for "edited on Laptop" and conflict names. Writes through
-- the API set `homespace.device_id` in their transaction; writes without it (signed in without
-- a device, background jobs) record NULL. No foreign key, like owner_id: devices are deleted with
-- their user, whose nodes are cascade-deleted.
ALTER TABLE node_changes ADD COLUMN device_id UUID;

-- After a transaction that set it ends, the setting reads as '' on that connection, not NULL.
CREATE OR REPLACE FUNCTION record_node_change() RETURNS trigger AS $$
DECLARE
    changed nodes%ROWTYPE;
BEGIN
    IF TG_OP = 'DELETE' THEN
        changed := OLD;
    ELSE
        changed := NEW;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(changed.owner_id::text, 0));
    INSERT INTO node_changes (owner_id, node_id, device_id)
    VALUES (
        changed.owner_id,
        changed.id,
        NULLIF(current_setting('homespace.device_id', true), '')::uuid
    );
    RETURN NULL;
END
$$ LANGUAGE plpgsql;
