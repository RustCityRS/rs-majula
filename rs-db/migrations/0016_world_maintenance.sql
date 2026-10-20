-- Whether a world is closed to new logins for maintenance (staff can still
-- log in), from its heartbeat, so the website can say so.
ALTER TABLE world_status ADD COLUMN maintenance BOOLEAN NOT NULL DEFAULT false;
