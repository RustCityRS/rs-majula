-- Public and private chat as the player typed it (before the word filter),
-- for moderators to check snapshots against.
CREATE TABLE chat_logs (
    id           BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id    BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    -- Who a private message was sent to, registered or not. NULL for public
    -- chat.
    to_name_hash BIGINT,
    -- Where the player was, and on which world.
    node_id      SMALLINT    NOT NULL,
    x            SMALLINT    NOT NULL,
    z            SMALLINT    NOT NULL,
    y            SMALLINT    NOT NULL,
    message      TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX chat_logs_player_idx ON chat_logs (player_id, created_at);
