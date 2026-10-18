-- Hiscores, kept in step with player_saves: every save rewrites its player's
-- rows in the same transaction.

-- One row per player per hiscore table. `category` is the website's table
-- number: 0 is Overall (total level and XP), any other is a stat id + 1.
-- Stats 18 and 19 are unused, so there are no tables 19 and 20.
CREATE TABLE hiscores (
    player_id  BIGINT      NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    category   SMALLINT    NOT NULL,
    level      SMALLINT    NOT NULL,
    -- 10x real XP, as the save stores it.
    xp         BIGINT      NOT NULL,
    -- When `xp` last changed. Equal XP ranks whoever got there first.
    reached_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, category)
);
-- Rank order, all ascending so a row comparison can range-scan it.
CREATE INDEX hiscores_rank_idx ON hiscores (category, (-level), (-xp), reached_at, player_id);

-- The rows that are ranked: trained, and not a J-Mod or banned player.
CREATE VIEW ranked_hiscores AS
SELECT h.*, p.name_hash, p.display_name
FROM hiscores h JOIN players p ON p.id = h.player_id
WHERE h.xp > 0
  AND p.staff_mod_level < 2
  AND (p.banned_until IS NULL OR p.banned_until <= now());

-- Rows for the saves that already exist. The thresholds are rs-stat's XP
-- table (10x real XP to reach levels 1..=99).
WITH skills AS (
    SELECT s.player_id, c.category, x.xp, s.updated_at,
           (SELECT count(*) FROM unnest('{
                0, 830, 1740, 2760, 3880, 5120, 6500, 8010, 9690, 11540,
                13580, 15840, 18330, 21070, 24110, 27460, 31150, 35230, 39730, 44700,
                50180, 56240, 62910, 70280, 78420, 87400, 97300, 108240, 120310, 133630,
                148330, 164560, 182470, 202240, 224060, 248150, 274730, 304080, 336480, 372240,
                411710, 455290, 503390, 556490, 615120, 679830, 751270, 830140, 917210, 1013330,
                1119450, 1236600, 1365940, 1508720, 1666360, 1840400, 2032540, 2244660, 2478860, 2737420,
                3022880, 3338040, 3685990, 4070150, 4494280, 4962540, 5479530, 6050320, 6680510, 7376270,
                8144450, 8992570, 9928950, 10962780, 12104210, 13364430, 14755810, 16292000, 17988080, 19860680,
                21928180, 24210870, 26731140, 29513730, 32585940, 35977920, 39722940, 43857760, 48422950, 53463320,
                59028310, 65172530, 71956290, 79446140, 87715580, 96845770, 106926290, 118056060, 130344310
            }'::int[]) t WHERE t <= x.xp)::smallint AS level
    FROM player_saves s
    CROSS JOIN unnest('{1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,21}'::smallint[]) AS c (category)
    -- Postgres arrays are 1-based, so stats[category] is stat category - 1.
    CROSS JOIN LATERAL (SELECT greatest(coalesce(s.stats[c.category], 0), 0)::bigint AS xp) x
)
INSERT INTO hiscores (player_id, category, level, xp, reached_at)
SELECT player_id, category, level, xp, updated_at FROM skills
UNION ALL
SELECT player_id, 0, sum(level)::smallint, sum(xp)::bigint, max(updated_at)
FROM skills GROUP BY player_id;
