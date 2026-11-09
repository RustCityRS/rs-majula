-- Where a player saved inside a region (instance) logs back in: the region's exit coord, else
-- the map tile the player came from. -1 when saved outside region space.
ALTER TABLE player_saves ADD COLUMN exit_x SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE player_saves ADD COLUMN exit_z SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE player_saves ADD COLUMN exit_y SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE save_history ADD COLUMN exit_x SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE save_history ADD COLUMN exit_z SMALLINT NOT NULL DEFAULT -1;
ALTER TABLE save_history ADD COLUMN exit_y SMALLINT NOT NULL DEFAULT -1;
