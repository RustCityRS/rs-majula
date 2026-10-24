-- no-transaction
-- The bans in force, for the sanctions list.
CREATE INDEX CONCURRENTLY IF NOT EXISTS players_banned_until_idx ON players (banned_until) WHERE banned_until IS NOT NULL;
