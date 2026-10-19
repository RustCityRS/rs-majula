-- Player identity and game state.
--
-- Replaces the schema the engine used to build at runtime (one wide
-- `player_saves` row per name, password included). Those tables are dropped
-- and their data discarded. rs-ether's `friends`/`ignores` are untouched: the
-- ether service owns them through its own Ecto migrations.
DROP TABLE IF EXISTS player_saves, player_varps, player_inventories;

-- One row per player name. This is what the website registers and what the
-- game login authenticates against.
CREATE TABLE players (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- base37 of the name: the identity the protocol and friend lists use.
    -- Unique by construction for case and space/underscore variants.
    name_hash       BIGINT      NOT NULL UNIQUE,
    display_name    TEXT        NOT NULL,
    password_hash   TEXT        NOT NULL,
    staff_mod_level SMALLINT    NOT NULL DEFAULT 0,
    banned_until    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A player's persistent game state, written by the engine. A player has no
-- row here until their first save; the engine applies new-player defaults
-- when none exists.
--
-- Varps and inventories are index-aligned arrays: varp_ids[i] = varp_values[i],
-- and inv_types/inv_slots/inv_objs/inv_counts describe one item per index.
CREATE TABLE player_saves (
    player_id     BIGINT      PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    x             SMALLINT    NOT NULL,
    z             SMALLINT    NOT NULL,
    y             SMALLINT    NOT NULL,
    body          SMALLINT[]  NOT NULL,
    colors        SMALLINT[]  NOT NULL,
    gender        SMALLINT    NOT NULL,
    runenergy     INT         NOT NULL,
    playtime      INT         NOT NULL,
    stats         INT[]       NOT NULL,
    levels        SMALLINT[]  NOT NULL,
    afk_zones     INT[]       NOT NULL,
    last_afk_zone SMALLINT    NOT NULL,
    public_chat   SMALLINT    NOT NULL,
    private_chat  SMALLINT    NOT NULL,
    trade_chat    SMALLINT    NOT NULL,
    last_date     BIGINT      NOT NULL,
    varp_ids      INT[]       NOT NULL,
    varp_values   INT[]       NOT NULL,
    inv_types     INT[]       NOT NULL,
    inv_slots     SMALLINT[]  NOT NULL,
    inv_objs      INT[]       NOT NULL,
    inv_counts    INT[]       NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
