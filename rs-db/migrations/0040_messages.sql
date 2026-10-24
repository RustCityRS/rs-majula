-- The Message Centre: messages staff send players from the console, which
-- players read on the website. One way only: players can't reply.
CREATE TABLE staff_messages (
    id         BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    subject    TEXT        NOT NULL,
    body       TEXT        NOT NULL,
    -- The console account that sent it, for staff and the audit only:
    -- never shown to the player or put in their data export.
    staff_name TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- When the player first opened it.
    read_at    TIMESTAMPTZ
);
-- Lists are newest first by id: the console's every message by the primary
-- key, and a player's (theirs, or the console filtered to them) by this.
CREATE INDEX staff_messages_player_idx ON staff_messages (player_id, id);
