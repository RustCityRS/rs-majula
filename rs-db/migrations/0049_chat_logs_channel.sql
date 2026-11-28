-- Where a chat line was said: 0 public, 1 private (to_name_hash is the
-- recipient), 2 friends chat (to_name_hash is the channel's owner), 3 clan chat.
ALTER TABLE chat_logs ADD COLUMN channel SMALLINT NOT NULL DEFAULT 0;

UPDATE chat_logs SET channel = 1 WHERE to_name_hash IS NOT NULL;
