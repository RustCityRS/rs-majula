-- Each item's varobjs, index-aligned with inv_objs; empty while every item's are 0.
ALTER TABLE player_saves ADD COLUMN inv_vars INT[] NOT NULL DEFAULT '{}';
ALTER TABLE save_history ADD COLUMN inv_vars INT[] NOT NULL DEFAULT '{}';
