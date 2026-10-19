-- Chat mutes and abuse report snapshots.

-- A muted player can still log in, but cannot public or private chat until
-- `muted_until` passes. Moderators set it in game from the abuse report's
-- "Mute player for 48 hours" option.
ALTER TABLE players ADD COLUMN muted_until TIMESTAMPTZ;

-- Snapshots: reports from the in-game "Report Abuse" screen, for moderators to review.
-- The offender is whatever name the reporter typed, registered or not.
CREATE TABLE snapshots (
    id                 BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    reporter_id        BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    offender_name_hash BIGINT      NOT NULL,
    -- The client's rule index: 0 (offensive language) to 11 (real-world
    -- trading).
    reason             SMALLINT    NOT NULL,
    -- Where the reporter was, and on which world.
    node_id            SMALLINT    NOT NULL,
    x                  SMALLINT    NOT NULL,
    z                  SMALLINT    NOT NULL,
    y                  SMALLINT    NOT NULL,
    -- Whether the reporter, a moderator, muted the offender with it.
    muted              BOOLEAN     NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX snapshots_offender_idx ON snapshots (offender_name_hash, created_at);
