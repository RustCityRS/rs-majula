-- no-transaction
-- The accounts registered lately.
CREATE INDEX CONCURRENTLY IF NOT EXISTS players_created_at_idx ON players (created_at);
