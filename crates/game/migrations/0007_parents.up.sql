-- A game may be a variant of another ("atomic" of "chess"): the front page lists it under that
-- game, and that game's page lists it among its variants. This replaces the free-text family,
-- which only grouped games on the front page. Pages are seeded from files at every start, so
-- the new column is filled there.
ALTER TABLE game_pages
    ADD COLUMN parent TEXT;
ALTER TABLE game_pages
    DROP COLUMN IF EXISTS family;
