-- Scripts the worlds aborted, for staff. A world gathers each error over
-- a minute or so: one row per error and whom it happened to (a player, a
-- kind of NPC, or neither), with how many times and where the first was.
CREATE TABLE script_errors (
    id        BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    node_id   SMALLINT    NOT NULL,
    first_at  TIMESTAMPTZ NOT NULL,
    last_at   TIMESTAMPTZ NOT NULL,
    count     INT         NOT NULL,
    -- The script, as [trigger,subject], and where in it.
    script    TEXT        NOT NULL,
    file      TEXT        NOT NULL,
    line      INT         NOT NULL,
    message   TEXT        NOT NULL,
    -- The scripts that called it, innermost first, a line each.
    trace     TEXT        NOT NULL,
    -- Whom it ran on. NULL player_id with no npc: the world (or a bot).
    player_id BIGINT      REFERENCES players (id) ON DELETE SET NULL,
    npc       TEXT,
    x         SMALLINT    NOT NULL,
    z         SMALLINT    NOT NULL,
    y         SMALLINT    NOT NULL,
    -- The player's recording, and how far into it the first one was.
    replay_id BIGINT,
    replay_ms INT
);
CREATE INDEX script_errors_last_idx ON script_errors (last_at DESC);
CREATE INDEX script_errors_key_idx ON script_errors (script, line, message, last_at DESC);
