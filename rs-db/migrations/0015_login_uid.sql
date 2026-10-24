-- The id each login's client install sent: the desktop client's uid.dat,
-- or the web client's per-browser id (once a deploy patches it in). NULL when
-- it sent none worth keeping. Accounts that log in from the same install are
-- likely the same person, as with a shared address.
ALTER TABLE login_logs ADD COLUMN uid INT;
CREATE INDEX login_logs_uid_idx ON login_logs (uid, created_at) WHERE uid IS NOT NULL;
