-- no-transaction
-- The oldest wealth events first, for the worlds to prune by. Built without
-- blocking the logs.
CREATE INDEX CONCURRENTLY IF NOT EXISTS wealth_events_created_idx ON wealth_events (created_at);
