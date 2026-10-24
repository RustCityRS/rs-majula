-- no-transaction
-- The mutes in force, for the sanctions list.
CREATE INDEX CONCURRENTLY IF NOT EXISTS players_muted_until_idx ON players (muted_until) WHERE muted_until IS NOT NULL;
