-- Player support: earlier saves to roll a player back to, and password
-- resets staff issue.

-- Earlier copies of players' saves. Taken at logout and through long
-- sessions (`logout`, `session`; at most one every so often), and just
-- before staff change a save (`staff`, with who and why in `note`) so the
-- change can be undone. Rows past the retention window are pruned by the
-- worlds, keeping each player's newest few.
CREATE TABLE save_history (
    id            BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id     BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    kind          TEXT        NOT NULL CHECK (kind IN ('logout', 'session', 'staff')),
    -- The world that wrote it.
    node_id       SMALLINT    NOT NULL,
    note          TEXT,
    saved_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- As in player_saves.
    x             SMALLINT    NOT NULL,
    z             SMALLINT    NOT NULL,
    y             SMALLINT    NOT NULL,
    body          SMALLINT[]  NOT NULL,
    colors        SMALLINT[]  NOT NULL,
    gender        SMALLINT    NOT NULL,
    runenergy     INT         NOT NULL,
    playtime      INT         NOT NULL,
    stats         INT[]       NOT NULL,
    levels        SMALLINT[]  NOT NULL,
    afk_zones     INT[]       NOT NULL,
    last_afk_zone SMALLINT    NOT NULL,
    public_chat   SMALLINT    NOT NULL,
    private_chat  SMALLINT    NOT NULL,
    trade_chat    SMALLINT    NOT NULL,
    last_date     BIGINT      NOT NULL,
    varp_ids      INT[]       NOT NULL,
    varp_values   INT[]       NOT NULL,
    inv_types     INT[]       NOT NULL,
    inv_slots     SMALLINT[]  NOT NULL,
    inv_objs      INT[]       NOT NULL,
    inv_counts    INT[]       NOT NULL
);
CREATE INDEX save_history_player_idx ON save_history (player_id, saved_at);
CREATE INDEX save_history_saved_idx ON save_history (saved_at);

-- A password reset staff issued: the hash of the one-time code the player
-- redeems on the website, and when it runs out.
ALTER TABLE players
    ADD COLUMN reset_code_hash  TEXT,
    ADD COLUMN reset_code_until TIMESTAMPTZ;
