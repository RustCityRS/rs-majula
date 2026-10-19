-- Tables and columns the website needs.

-- Where a player was registered from, for per-IP registration rate limiting.
-- NULL for players registered in-game (dev auto-register).
ALTER TABLE players ADD COLUMN registration_ip INET;
CREATE INDEX players_registration_ip_created_at_idx ON players (registration_ip, created_at);

CREATE TABLE news_categories (
    id    SMALLINT PRIMARY KEY,
    name  TEXT NOT NULL,
    -- CSS class the website colours the category with.
    style TEXT NOT NULL
);

INSERT INTO news_categories (id, name, style) VALUES
    (1, 'Game Updates', 'red'),
    (2, 'Website', 'lblue'),
    (3, 'Customer Support', 'yellow'),
    (4, 'Technical', 'dblue'),
    (5, 'Community', 'green'),
    (6, 'Behind the Scenes', 'purple'),
    (7, 'Archived', 'white');

CREATE TABLE news_posts (
    id          INT         GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    category_id SMALLINT    NOT NULL REFERENCES news_categories (id),
    title       TEXT        NOT NULL,
    -- Trusted HTML, rendered as-is.
    content     TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX news_posts_category_id_id_idx ON news_posts (category_id, id);
