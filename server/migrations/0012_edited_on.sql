-- The device that wrote a file's content (or created a folder), for "Edited on Laptop". NULL
-- when the write did not come from a registered device. Renames and moves keep it. No foreign
-- key, like node_changes.device_id: devices are deleted with their user.
ALTER TABLE nodes ADD COLUMN edited_on UUID;
ALTER TABLE file_versions ADD COLUMN edited_on UUID;

-- Reads the device the API set for the transaction (see 0011); '' once such a transaction ended.
CREATE FUNCTION record_node_editor() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'INSERT' OR NEW.sha256 IS DISTINCT FROM OLD.sha256 THEN
        NEW.edited_on := NULLIF(current_setting('homespace.device_id', true), '')::uuid;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER nodes_record_editor
    BEFORE INSERT OR UPDATE OF sha256 ON nodes
    FOR EACH ROW EXECUTE FUNCTION record_node_editor();
