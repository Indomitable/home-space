-- A per-user journal of node changes, so a client can catch up on what it missed with
-- `GET /changes?since=<seq>`. Only the node id is recorded; its current state is read from
-- `nodes` when the journal is queried, and a node that is gone or binned reads as deleted.
CREATE TABLE node_changes (
    seq        BIGSERIAL PRIMARY KEY,
    -- No foreign key: the rows written while a user's nodes are cascade-deleted with the
    -- user would violate it.
    owner_id   UUID NOT NULL,
    node_id    UUID NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX node_changes_owner_seq ON node_changes (owner_id, seq);
CREATE INDEX node_changes_changed_at ON node_changes (changed_at);

-- The highest seq pruned from each owner's journal; a client behind it has to rescan.
CREATE TABLE node_changes_pruned (
    owner_id UUID PRIMARY KEY,
    through  BIGINT NOT NULL
);

-- Runs at commit (the triggers are deferred) under a per-owner lock, so an owner's seqs are
-- handed out in commit order and a reader never sees a later seq before an earlier one.
CREATE FUNCTION record_node_change() RETURNS trigger AS $$
DECLARE
    changed nodes%ROWTYPE;
BEGIN
    IF TG_OP = 'DELETE' THEN
        changed := OLD;
    ELSE
        changed := NEW;
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(changed.owner_id::text, 0));
    INSERT INTO node_changes (owner_id, node_id) VALUES (changed.owner_id, changed.id);
    RETURN NULL;
END
$$ LANGUAGE plpgsql;

CREATE CONSTRAINT TRIGGER nodes_record_created_or_deleted
    AFTER INSERT OR DELETE ON nodes
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION record_node_change();

CREATE CONSTRAINT TRIGGER nodes_record_updated
    AFTER UPDATE ON nodes
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    WHEN (OLD.* IS DISTINCT FROM NEW.*)
    EXECUTE FUNCTION record_node_change();
