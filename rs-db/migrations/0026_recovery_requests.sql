-- Requests to get back into an account, sent from the website by players
-- who can't log in. The staff console takes each into a case of its own;
-- approving one issues a password reset that only the request's recovery
-- key redeems, and the decision comes back here for the sender to see.
CREATE TABLE recovery_requests (
    id               BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    player_id        BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    -- The recovery key its sender was shown, hashed as passwords are.
    key_hash         TEXT        NOT NULL,
    -- Why they can't log in.
    reason           TEXT        NOT NULL CHECK (reason IN ('forgot', 'hijacked', 'other')),
    details          TEXT        NOT NULL,
    -- How many passwords they remembered using, and whether one is the
    -- account's now. The passwords themselves aren't kept.
    passwords_given  SMALLINT    NOT NULL DEFAULT 0,
    password_matched BOOLEAN     NOT NULL DEFAULT false,
    -- Their answers about the account, each left out if they didn't know.
    -- `created_month` is the first of the month they say they made it.
    created_month    DATE,
    last_played      DATE,
    device           TEXT        CHECK (device IN ('computer', 'phone', 'client')),
    skills           TEXT,
    items            TEXT,
    location         TEXT,
    friends          TEXT,
    traded_with      TEXT,
    other_accounts   TEXT,
    -- Where it was sent from: the address, the web client's browser ids
    -- (one per world, as each world's /uid page reported it) and what the
    -- browser said it was.
    ip               INET        NOT NULL,
    uids             INT[]       NOT NULL DEFAULT '{}',
    user_agent       TEXT,
    platform         TEXT,
    language         TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The console's case for it, once taken in.
    case_id          BIGINT,
    decision         TEXT        CHECK (decision IN ('approved', 'denied')),
    decision_note    TEXT,
    decided_by       TEXT,
    decided_at       TIMESTAMPTZ
);
CREATE INDEX recovery_requests_player_idx ON recovery_requests (player_id, created_at);
CREATE INDEX recovery_requests_ip_idx ON recovery_requests (ip, created_at);
-- What the console hasn't taken in yet.
CREATE INDEX recovery_requests_waiting_idx ON recovery_requests (id) WHERE case_id IS NULL;
