-- The journal is compacted instead of pruned by age: only the latest entry of each node is
-- kept, which is all `GET /changes` reads, so a client can catch up from any cursor however long
-- it was away. This index finds the older entries of a node.
CREATE INDEX node_changes_owner_node_seq ON node_changes (owner_id, node_id, seq);
