-- Each world's heartbeat, upserted every few seconds while it runs. The
-- website lists a world as online only while its row is fresh, so a world
-- that crashes or hangs drops off without having to clean up after itself.
CREATE TABLE world_status (
    node_id    SMALLINT PRIMARY KEY,
    players    INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
