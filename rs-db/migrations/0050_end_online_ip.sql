-- Ending a session clears the address it was played from, with its login.
CREATE OR REPLACE FUNCTION end_online_login() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.online_node IS NULL THEN
        NEW.online_login := NULL;
        NEW.online_ip := NULL;
    END IF;
    IF OLD.online_login IS NOT NULL AND NEW.online_login IS DISTINCT FROM OLD.online_login THEN
        UPDATE login_logs SET ended_at = now() WHERE id = OLD.online_login AND ended_at IS NULL;
    END IF;
    RETURN NEW;
END
$$;

UPDATE players SET online_ip = NULL WHERE online_node IS NULL AND online_ip IS NOT NULL;
