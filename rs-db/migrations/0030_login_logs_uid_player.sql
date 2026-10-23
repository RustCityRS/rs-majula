-- no-transaction
-- The accounts each browser was used by, to tell a crowded one quickly.
CREATE INDEX CONCURRENTLY IF NOT EXISTS login_logs_uid_player_idx ON login_logs (uid, player_id) WHERE uid IS NOT NULL;
