-- Recovery requests no longer ask for the account's skills: anyone can look
-- them up on the hiscores, so they prove nothing.
ALTER TABLE recovery_requests DROP COLUMN skills;
