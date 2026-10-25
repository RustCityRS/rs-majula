-- Playing several accounts at once, and passing items between your own
-- accounts: what the staff console's checks look for.

-- When each login's session ended, and the login a player is online with
-- now (`players.online_login`): sessions that overlap from the same browser,
-- device or address are one person on two accounts. NULL ended_at: still
-- on (it's someone's online_login), or it never entered the world.
--
-- `fingerprint`: a hash of what the web client page measured about the
-- device (its canvas and WebGL drawing, audio, fonts, cores and time zone),
-- the same in a private window or after the browser's storage is cleared,
-- where the browser id (`uid`) starts over. Identical devices share one, so
-- it links nobody once crowded, like an address.
--
-- `refused`: why a login that got the password right was turned away
-- anyway: `multilog`, another account being online from the same browser.
--
-- What else the page measured, to check against what the browser claims
-- in its headers: its rendering `engine` (blink, gecko, webkit),
-- `navigator.platform` (`js_platform`) and graphics card (`gpu`).
-- `tamper`: how the two disagree, or that the check was missing or forged
-- (`no_check`, `bad_seal`, `user_agent`, `engine`, `platform`, `graphics`);
-- empty when they agree, NULL for the desktop client.
ALTER TABLE login_logs
    ADD COLUMN ended_at    TIMESTAMPTZ,
    ADD COLUMN fingerprint BIGINT,
    ADD COLUMN refused     TEXT,
    ADD COLUMN engine      TEXT,
    ADD COLUMN js_platform TEXT,
    ADD COLUMN gpu         TEXT,
    ADD COLUMN tamper      TEXT[];

-- When the player chose an action (clicked something, walked, pressed a
-- button), one bit per 3 seconds from the recording's start, lowest bit
-- first: two accounts played by one person take turns.
ALTER TABLE replays ADD COLUMN activity BYTEA;
ALTER TABLE players ADD COLUMN online_login BIGINT;

CREATE INDEX login_logs_fingerprint_idx ON login_logs (fingerprint, created_at)
    WHERE fingerprint IS NOT NULL;

-- Each device fingerprint an account logged in with, as login_ips and
-- login_uids keep addresses and browsers.
CREATE TABLE login_fingerprints (
    fingerprint BIGINT      NOT NULL,
    player_id   BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    last_at     TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (fingerprint, player_id)
);
CREATE INDEX login_fingerprints_player_idx ON login_fingerprints (player_id);
CREATE INDEX login_fingerprints_last_idx ON login_fingerprints (last_at);

CREATE OR REPLACE FUNCTION login_pairs() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO login_ips (ip, player_id, last_at) VALUES (NEW.ip, NEW.player_id, NEW.created_at)
    ON CONFLICT (ip, player_id) DO UPDATE SET last_at = greatest(login_ips.last_at, EXCLUDED.last_at);
    IF NEW.uid IS NOT NULL THEN
        INSERT INTO login_uids (uid, player_id, last_at) VALUES (NEW.uid, NEW.player_id, NEW.created_at)
        ON CONFLICT (uid, player_id) DO UPDATE SET last_at = greatest(login_uids.last_at, EXCLUDED.last_at);
    END IF;
    IF NEW.fingerprint IS NOT NULL THEN
        INSERT INTO login_fingerprints (fingerprint, player_id, last_at)
        VALUES (NEW.fingerprint, NEW.player_id, NEW.created_at)
        ON CONFLICT (fingerprint, player_id)
        DO UPDATE SET last_at = greatest(login_fingerprints.last_at, EXCLUDED.last_at);
    END IF;
    RETURN NULL;
END
$$;

-- Whatever ends a session (a logout save, a release, a takeover by another
-- world) clears online_node, or claims it with another login: the login it
-- was ends then. No foreign key: pruning a login mustn't touch players.
CREATE FUNCTION end_online_login() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.online_node IS NULL THEN
        NEW.online_login := NULL;
    END IF;
    IF OLD.online_login IS NOT NULL AND NEW.online_login IS DISTINCT FROM OLD.online_login THEN
        UPDATE login_logs SET ended_at = now() WHERE id = OLD.online_login AND ended_at IS NULL;
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER players_end_online_login BEFORE UPDATE OF online_node, online_login ON players
    FOR EACH ROW EXECUTE FUNCTION end_online_login();

-- A shop buy's sale: the latest of that item at that shop before it, as a
-- pickup's drop (wealth_events_drop_idx).
CREATE INDEX wealth_events_sell_idx ON wealth_events (node_id, item, created_at)
    WHERE event_type = 7;
