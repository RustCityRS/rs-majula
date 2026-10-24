-- no-transaction
-- Flagged sessions, the latest first, as the replay pages list them. Built
-- without blocking recording.
CREATE INDEX CONCURRENTLY IF NOT EXISTS replays_flagged_started_idx ON replays (started_at DESC) WHERE score > 0;
