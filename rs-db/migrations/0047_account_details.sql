-- An account's details beyond its login, one row per player, made with the
-- player by a trigger. New accounts start free to play, and a world grants
-- membership on the first login to a members world. Accounts that have
-- played already (they have a save) were members, as every account was. A
-- null date is none.
CREATE TABLE account_details (
    player_id        BIGINT      PRIMARY KEY REFERENCES players (id) ON DELETE CASCADE,
    chat_consent     BOOLEAN     NOT NULL DEFAULT false,
    advert_consent   BOOLEAN     NOT NULL DEFAULT false,
    date_of_birth    DATE,
    dob_requested    BOOLEAN     NOT NULL DEFAULT false,
    subscribed       BOOLEAN     NOT NULL DEFAULT false,
    members_until    TIMESTAMPTZ,
    jcoins           INTEGER     NOT NULL DEFAULT 0,
    loyalty_points   INTEGER     NOT NULL DEFAULT 0,
    email_status     SMALLINT    NOT NULL DEFAULT 3,
    account_guardian BOOLEAN     NOT NULL DEFAULT true,
    card_expires     DATE,
    grace_expires    DATE
);

-- As its owner: a restricted role that registers players can't write here.
CREATE FUNCTION account_details_row() RETURNS trigger LANGUAGE plpgsql
    SECURITY DEFINER SET search_path = public AS $$
BEGIN
    INSERT INTO account_details (player_id) VALUES (NEW.id);
    RETURN NULL;
END
$$;

CREATE TRIGGER players_account_details AFTER INSERT ON players
    FOR EACH ROW EXECUTE FUNCTION account_details_row();

INSERT INTO account_details (player_id, subscribed)
SELECT p.id, EXISTS (SELECT 1 FROM player_saves s WHERE s.player_id = p.id) FROM players p;
