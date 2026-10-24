-- no-transaction
-- One world's chat, newest first.
CREATE INDEX CONCURRENTLY IF NOT EXISTS chat_logs_node_id_idx ON chat_logs (node_id, id);
