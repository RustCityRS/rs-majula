-- When the hold on a previous display name starts: one given up since then
-- still belongs to its owner (unclaimable by others, and still finds them).
CREATE FUNCTION display_name_hold_start() RETURNS TIMESTAMPTZ
    LANGUAGE sql STABLE AS $$ SELECT now() - interval '35 days' $$;
