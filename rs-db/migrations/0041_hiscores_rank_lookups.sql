-- A player's rank counts the rows ahead of them straight from the rank
-- index, then takes off the few of those that aren't ranked, instead of
-- checking every row ahead against its player.

-- Level and XP ride along, so counting never visits the table.
DROP INDEX hiscores_rank_idx;
CREATE INDEX hiscores_rank_idx ON hiscores (category, (-level), (-xp), reached_at, player_id)
    INCLUDE (level, xp);

-- The players who may be kept off every table: staff, hidden, and anyone
-- banned (whose ban may since have run out).
CREATE INDEX players_unranked_idx ON players (id)
    WHERE staff_mod_level >= 2 OR hiscores_hidden OR banned_until IS NOT NULL;
