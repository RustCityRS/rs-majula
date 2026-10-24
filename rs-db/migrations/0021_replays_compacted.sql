-- Whether a finished recording was compacted (or was in one chunk already),
-- so the worlds' compaction only looks at the ones that weren't.
ALTER TABLE replays ADD COLUMN compacted BOOLEAN NOT NULL DEFAULT false;
UPDATE replays r SET compacted = true
WHERE r.ended_at IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM replay_chunks c WHERE c.replay_id = r.id AND c.seq > 0);
CREATE INDEX replays_compact_idx ON replays (node_id, id) WHERE NOT compacted;
