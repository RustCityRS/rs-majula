-- no-transaction
-- The oldest chat first, for the worlds to prune by. Built without blocking
-- chat.
CREATE INDEX CONCURRENTLY IF NOT EXISTS chat_logs_created_idx ON chat_logs (created_at);
