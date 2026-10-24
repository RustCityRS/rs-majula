-- Staff tools: login history for linking accounts, and the reasons
-- moderators give.

-- Each login that authenticated: which world, and the address it came from.
-- Accounts that log in from the same address are likely the same person.
-- Rows older than the retention window are pruned by the worlds.
CREATE TABLE login_logs (
    id         BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    node_id    SMALLINT    NOT NULL,
    ip         INET        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX login_logs_player_idx ON login_logs (player_id, created_at);
CREATE INDEX login_logs_ip_idx ON login_logs (ip, created_at);
CREATE INDEX login_logs_created_idx ON login_logs (created_at);

-- For a player's timeline: the abuse reports they sent.
CREATE INDEX snapshots_reporter_idx ON snapshots (reporter_id, created_at);

-- Why the moderator did it, when they said (from the console: the reason
-- typed, or the sanction template applied).
ALTER TABLE moderation_logs ADD COLUMN reason TEXT;
