-- no-transaction
-- One sender's chat, newest first.
CREATE INDEX CONCURRENTLY IF NOT EXISTS chat_logs_player_id_idx ON chat_logs (player_id, id);
