-- Staff moderation commands (::mute, ::unmute, ::ban, ::unban, ::kick): what
-- each moderator did to whom. Only actions that took are recorded.
CREATE TABLE moderation_logs (
    id           BIGINT      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    moderator_id BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    target_id    BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    action       TEXT        NOT NULL
        CHECK (action IN ('mute', 'unmute', 'ban', 'unban', 'kick')),
    -- How long a mute or ban lasts. NULL for the other actions.
    hours        INT,
    -- The world the moderator ran it on.
    node_id      SMALLINT    NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX moderation_logs_target_idx ON moderation_logs (target_id, created_at);
