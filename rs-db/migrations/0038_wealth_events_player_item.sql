-- no-transaction
-- How a player came by an item, for its trace.
CREATE INDEX CONCURRENTLY IF NOT EXISTS wealth_events_player_item_idx ON wealth_events (player_id, item, created_at);
