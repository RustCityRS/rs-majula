-- Cluster-wide session ownership and login rate limiting.

-- The world that owns each player's session: it claimed the player when it
-- loaded their profile, and it alone may save them until it releases the
-- claim (at logout, together with the final save). NULL while offline. This
-- is the authority that keeps two worlds from ever holding the same profile:
-- a world that loses the claim has its saves refused instead of overwriting
-- newer data.
--
-- The last logout's world and time drive the world-hop delay (the client's
-- "profile will be transferred" countdown).
ALTER TABLE players
    ADD COLUMN online_node      SMALLINT,
    ADD COLUMN online_since     TIMESTAMPTZ,
    ADD COLUMN last_logout_node SMALLINT,
    ADD COLUMN last_logout_at   TIMESTAMPTZ;
CREATE INDEX players_online_node_idx ON players (online_node) WHERE online_node IS NOT NULL;

-- Wrong-password logins, counted per IP across every world.
CREATE TABLE login_failures (
    ip INET        NOT NULL,
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX login_failures_ip_at_idx ON login_failures (ip, at);
