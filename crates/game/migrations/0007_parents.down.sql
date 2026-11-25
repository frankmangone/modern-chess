ALTER TABLE game_pages
    ADD COLUMN family TEXT;
ALTER TABLE game_pages
    DROP COLUMN IF EXISTS parent;
