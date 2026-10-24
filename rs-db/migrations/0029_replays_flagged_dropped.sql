-- no-transaction
-- Led by score, it served none of the flagged-session queries
-- (replays_flagged_started_idx does).
DROP INDEX CONCURRENTLY IF EXISTS replays_flagged_idx;
