-- Session recordings, for moderators to replay a player's game in the staff
-- console: everything the world sent them and everything their client sent
-- back, from login to logout. Kept for a set number of days (the worlds'
-- `--replay-days`), then pruned. Large: keep `replay_chunks` out of routine
-- dumps (`pg_dump --exclude-table-data=replay_chunks`).
CREATE TABLE replays (
    -- Chosen by the world that records it.
    id          BIGINT      PRIMARY KEY,
    player_id   BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    node_id     SMALLINT    NOT NULL,
    -- The protocol revision it was recorded at: it only replays in that
    -- revision's client.
    revision    TEXT        NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- NULL while the session goes on, or if the world stopped without
    -- ending it.
    ended_at    TIMESTAMPTZ,
    -- When the last chunk came in.
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    duration_ms INT         NOT NULL DEFAULT 0,
    -- Compressed size of the chunks.
    bytes       BIGINT      NOT NULL DEFAULT 0,
    -- False once a chunk was lost: the replay stops there.
    complete    BOOLEAN     NOT NULL DEFAULT true,
    -- The anti-cheat checks: how suspicious the session looks (0 = nothing
    -- found), the checks it failed, and the measurements behind them (JSON).
    score       SMALLINT    NOT NULL DEFAULT 0,
    flags       TEXT[]      NOT NULL DEFAULT '{}',
    stats       TEXT        NOT NULL DEFAULT '{}'
);
CREATE INDEX replays_player_idx ON replays (player_id, started_at DESC);
CREATE INDEX replays_started_idx ON replays (started_at DESC);
CREATE INDEX replays_flagged_idx ON replays (score DESC, started_at DESC) WHERE score > 0;

-- A recording in order, a deflated chunk at a time.
CREATE TABLE replay_chunks (
    replay_id BIGINT NOT NULL REFERENCES replays (id) ON DELETE CASCADE,
    seq       INT    NOT NULL,
    -- Milliseconds into the session the chunk covers.
    start_ms  INT    NOT NULL,
    end_ms    INT    NOT NULL,
    data      BYTEA  NOT NULL,
    PRIMARY KEY (replay_id, seq)
);
