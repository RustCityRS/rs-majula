-- no-transaction
-- The oldest session logs first, for the worlds to prune by. Built without
-- blocking the logs.
CREATE INDEX CONCURRENTLY IF NOT EXISTS session_logs_created_idx ON session_logs (created_at);
