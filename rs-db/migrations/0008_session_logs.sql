-- Gameplay logs written by content scripts, for moderators to review.

-- `session_log`: notable events, as text. `event_type` is the script's
-- ^log_moderator (0) or ^log_adventure (1).
CREATE TABLE session_logs (
    id         BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    -- Where the player was, and on which world.
    node_id    SMALLINT    NOT NULL,
    x          SMALLINT    NOT NULL,
    z          SMALLINT    NOT NULL,
    y          SMALLINT    NOT NULL,
    event_type SMALLINT    NOT NULL,
    event      TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX session_logs_player_idx ON session_logs (player_id, created_at);

-- `wealth_event`: wealth entering or leaving a player's hands. `event_type`
-- is the script's ^wealth_* constant (pickup, shop buy/sell, alchemy, party
-- room).
CREATE TABLE wealth_events (
    id         BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    node_id    SMALLINT    NOT NULL,
    x          SMALLINT    NOT NULL,
    z          SMALLINT    NOT NULL,
    y          SMALLINT    NOT NULL,
    event_type SMALLINT    NOT NULL,
    -- The obj's debug name.
    item       TEXT        NOT NULL,
    count      INT         NOT NULL,
    -- In coins, as the script valued it.
    value      INT         NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX wealth_events_player_idx ON wealth_events (player_id, created_at);
