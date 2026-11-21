ALTER TABLE world_status
    ADD COLUMN lobby_instance BIGINT,
    ADD COLUMN revision INTEGER,
    ADD COLUMN world_id SMALLINT,
    ADD COLUMN address TEXT,
    ADD COLUMN members BOOLEAN;

ALTER TABLE players
    ADD COLUMN online_ip INET,
    ADD COLUMN lobby_session BIGINT,
    ADD COLUMN lobby_ip INET,
    ADD COLUMN lobby_revision INTEGER,
    ADD COLUMN lobby_target SMALLINT,
    ADD COLUMN lobby_until TIMESTAMPTZ;

CREATE INDEX players_online_ip ON players (online_ip) WHERE online_node IS NOT NULL;
