-- A game may draw its pages in colours of its own instead of the site's walnut: a few named
-- colours, written #rrggbb. Pages are seeded from files at every start, so it is filled there.
ALTER TABLE game_pages
    ADD COLUMN theme JSONB;
