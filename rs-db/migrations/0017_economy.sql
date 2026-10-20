-- The economy: items passing between players, and hiscores staff keep
-- accounts off.

-- The engine logs items handed over in trades and duel stakes, dropped and
-- picked up from the floor as wealth events of its own (types 100 and up;
-- the content's ^wealth_* constants are below). `counterpart_id` is the
-- player a handover went to.
ALTER TABLE wealth_events
    ADD COLUMN counterpart_id BIGINT REFERENCES players (id) ON DELETE SET NULL;
CREATE INDEX wealth_events_counterpart_idx ON wealth_events (counterpart_id, created_at)
    WHERE counterpart_id IS NOT NULL;
-- Handovers by time, for the trade graph's busiest receivers.
CREATE INDEX wealth_events_handover_idx ON wealth_events (created_at)
    WHERE counterpart_id IS NOT NULL;
-- A pickup's drop: the latest of that item on that tile before it.
CREATE INDEX wealth_events_drop_idx ON wealth_events (node_id, x, z, y, item, created_at)
    WHERE event_type = 103;

-- Accounts staff keep off every hiscore table, and the tables a cheater's
-- entries were removed from. Saves still rewrite their rows; they're just
-- not ranked.
ALTER TABLE players ADD COLUMN hiscores_hidden BOOLEAN NOT NULL DEFAULT false;
CREATE TABLE hiscores_removed (
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    category   SMALLINT    NOT NULL,
    removed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, category)
);

-- Ranked: trained, and not staff (any staff level), banned, hidden or
-- removed from that table.
CREATE OR REPLACE VIEW ranked_hiscores AS
SELECT h.*, p.name_hash, p.display_name
FROM hiscores h JOIN players p ON p.id = h.player_id
WHERE h.xp > 0
  AND p.staff_mod_level = 0
  AND NOT p.hiscores_hidden
  AND (p.banned_until IS NULL OR p.banned_until <= now())
  AND NOT EXISTS (
      SELECT 1 FROM hiscores_removed r
      WHERE r.player_id = h.player_id AND r.category = h.category);
