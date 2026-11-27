-- A new account, or a guest that has just become one, has a short set-up to go through (choosing
-- a username, and later more). The flag stays on until the person finishes it. Everyone who exists
-- already has nothing to set up.
ALTER TABLE users ADD COLUMN setup_pending BOOLEAN NOT NULL DEFAULT FALSE;
