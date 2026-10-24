-- Player moderators (staff level 1) are players too, so they're ranked
-- again; only Jagex moderators and developers (2 and up) are left off.
CREATE OR REPLACE VIEW ranked_hiscores AS
SELECT h.*, p.name_hash, p.display_name
FROM hiscores h JOIN players p ON p.id = h.player_id
WHERE h.xp > 0
  AND p.staff_mod_level < 2
  AND NOT p.hiscores_hidden
  AND (p.banned_until IS NULL OR p.banned_until <= now())
  AND NOT EXISTS (
      SELECT 1 FROM hiscores_removed r
      WHERE r.player_id = h.player_id AND r.category = h.category);
