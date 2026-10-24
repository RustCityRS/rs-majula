-- no-transaction
-- Each player's points by id, for the prune's keep-th newest.
CREATE INDEX CONCURRENTLY IF NOT EXISTS save_history_player_id_idx ON save_history (player_id, id);
