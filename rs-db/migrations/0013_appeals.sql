-- Appeals against mutes and bans, sent by players from the website. The
-- staff console takes each into a case of its own, and its decision comes
-- back here for the player to see.
CREATE TABLE appeals (
    id                BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id         BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    sanction          TEXT        NOT NULL CHECK (sanction IN ('mute', 'ban')),
    -- The sanction as it stood when appealed: when it was to end, the
    -- moderation log entry that set it and who that was. NULL for one with
    -- no entry (an abuse report's mute, the anti-cheat's automatic ban).
    sanction_until    TIMESTAMPTZ NOT NULL,
    moderation_log_id BIGINT      REFERENCES moderation_logs (id) ON DELETE SET NULL,
    issued_by         TEXT,
    statement         TEXT        NOT NULL,
    ip                INET,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The console's case for it, once taken in.
    case_id           BIGINT,
    -- 'upheld' or 'overturned', with what the player is told.
    decision          TEXT        CHECK (decision IN ('upheld', 'overturned')),
    decision_note     TEXT,
    decided_by        TEXT,
    decided_at        TIMESTAMPTZ
);
CREATE INDEX appeals_player_idx ON appeals (player_id, created_at);
-- What the console hasn't taken in yet.
CREATE INDEX appeals_waiting_idx ON appeals (id) WHERE case_id IS NULL;
-- One undecided appeal per player and kind of sanction.
CREATE UNIQUE INDEX appeals_one_open_idx ON appeals (player_id, sanction) WHERE decision IS NULL;
