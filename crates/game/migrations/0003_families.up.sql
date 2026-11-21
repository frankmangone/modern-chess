-- The family a game belongs to ("Chess and its variants"), which the front page groups games
-- by. Empty means the game stands on its own.
ALTER TABLE game_pages
    ADD COLUMN family TEXT;
