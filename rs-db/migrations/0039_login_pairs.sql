-- Each address and client install an account logged in with, once, and its
-- latest login with it: what the account link lookups read to find who else
-- used one, instead of every login. Kept by a trigger on login_logs, and
-- pruned with it.
CREATE TABLE login_ips (
    ip        INET        NOT NULL,
    player_id BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    last_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (ip, player_id)
);
CREATE INDEX login_ips_player_idx ON login_ips (player_id);
CREATE INDEX login_ips_last_idx ON login_ips (last_at);

CREATE TABLE login_uids (
    uid       INT         NOT NULL,
    player_id BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    last_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (uid, player_id)
);
CREATE INDEX login_uids_player_idx ON login_uids (player_id);
CREATE INDEX login_uids_last_idx ON login_uids (last_at);

CREATE FUNCTION login_pairs() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO login_ips (ip, player_id, last_at) VALUES (NEW.ip, NEW.player_id, NEW.created_at)
    ON CONFLICT (ip, player_id) DO UPDATE SET last_at = greatest(login_ips.last_at, EXCLUDED.last_at);
    IF NEW.uid IS NOT NULL THEN
        INSERT INTO login_uids (uid, player_id, last_at) VALUES (NEW.uid, NEW.player_id, NEW.created_at)
        ON CONFLICT (uid, player_id) DO UPDATE SET last_at = greatest(login_uids.last_at, EXCLUDED.last_at);
    END IF;
    RETURN NULL;
END
$$;

-- The trigger first: it holds off new logins until this commits, so the
-- fill below sees every one.
CREATE TRIGGER login_logs_pairs AFTER INSERT ON login_logs
    FOR EACH ROW EXECUTE FUNCTION login_pairs();

INSERT INTO login_ips (ip, player_id, last_at)
SELECT ip, player_id, max(created_at) FROM login_logs GROUP BY ip, player_id;
INSERT INTO login_uids (uid, player_id, last_at)
SELECT uid, player_id, max(created_at) FROM login_logs WHERE uid IS NOT NULL GROUP BY uid, player_id;
