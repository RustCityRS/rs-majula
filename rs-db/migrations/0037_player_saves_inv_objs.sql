-- no-transaction
-- The saves holding an item, for its holders.
CREATE INDEX CONCURRENTLY IF NOT EXISTS player_saves_inv_objs_idx ON player_saves USING gin (inv_objs);
