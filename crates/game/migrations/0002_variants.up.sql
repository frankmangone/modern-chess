-- A game is a container; the variants inside it are what people actually play.
-- `game_pages` stays the game's row. `name` is the game's own name ("Go"), and
-- `preview_variant` is the variant its "Try it" board uses. Both may be empty: the name then
-- comes from the rules of the preview variant, and the preview is the first variant.
ALTER TABLE game_pages
    ADD COLUMN name TEXT,
    ADD COLUMN preview_variant TEXT;

-- One playable variant of a game. `slug` is the slug of its rules in `game_specs` (and of its
-- matches in `game_instances`), so existing matches need no change. No foreign key to
-- `game_specs`: its rows are versioned per slug. No cascade from the game: a game that still
-- has variants must not be deletable.
CREATE TABLE game_variants (
    slug        TEXT PRIMARY KEY,
    game_slug   TEXT    NOT NULL REFERENCES game_pages (slug),
    label       TEXT    NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    -- A full skin that replaces the game's. Empty means the variant uses the game's.
    skin        JSONB
);

CREATE INDEX idx_game_variants_game ON game_variants (game_slug, sort_order, slug);

-- Every existing page becomes a game with one variant of the same slug.
UPDATE game_pages p
SET name = COALESCE(
    (SELECT s.name FROM game_specs s WHERE s.slug = p.slug ORDER BY s.version DESC LIMIT 1),
    p.slug
);

INSERT INTO game_variants (slug, game_slug, label, sort_order)
SELECT slug, slug, name, 0 FROM game_pages;
