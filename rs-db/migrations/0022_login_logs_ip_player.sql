-- no-transaction
-- How many accounts logged in from an address, read off the index alone (see
-- the crowded address checks). Built without blocking logins.
CREATE INDEX CONCURRENTLY IF NOT EXISTS login_logs_ip_player_idx ON login_logs (ip, player_id);
