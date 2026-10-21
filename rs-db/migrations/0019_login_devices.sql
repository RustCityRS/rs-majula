-- What each login's client said about the device it ran on, for staff: the
-- web client's browser (user agent), platform and language, and whether it
-- said it's a phone or tablet. The desktop client says nothing (web false).
-- NULL web: logged before devices were recorded.
ALTER TABLE login_logs
    ADD COLUMN web        BOOLEAN,
    ADD COLUMN user_agent TEXT,
    ADD COLUMN platform   TEXT,
    ADD COLUMN language   TEXT,
    ADD COLUMN touch      BOOLEAN NOT NULL DEFAULT false;
