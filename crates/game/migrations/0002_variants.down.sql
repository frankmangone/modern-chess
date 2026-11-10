DROP TABLE IF EXISTS game_variants;
ALTER TABLE game_pages
    DROP COLUMN IF EXISTS preview_variant,
    DROP COLUMN IF EXISTS name;
