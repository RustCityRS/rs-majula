-- A player's previous display name and when they took the current one. The
-- previous name stays held (unclaimable by others, and still finds them) for
-- 35 days after the change.
ALTER TABLE players
    ADD COLUMN previous_display_name   TEXT,
    ADD COLUMN display_name_changed_at TIMESTAMPTZ;

CREATE INDEX players_display_name_lower ON players (lower(display_name));
CREATE INDEX players_previous_display_name_lower ON players (lower(previous_display_name))
    WHERE previous_display_name IS NOT NULL;

-- The chat filters are the account's, not the save's: the lobby sets them
-- before an account has a save.
ALTER TABLE account_details
    ADD COLUMN public_chat  SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN private_chat SMALLINT NOT NULL DEFAULT 0,
    ADD COLUMN trade_chat   SMALLINT NOT NULL DEFAULT 0;

UPDATE account_details d
SET public_chat = s.public_chat, private_chat = s.private_chat, trade_chat = s.trade_chat
FROM player_saves s
WHERE s.player_id = d.player_id;

ALTER TABLE player_saves
    DROP COLUMN public_chat,
    DROP COLUMN private_chat,
    DROP COLUMN trade_chat;
