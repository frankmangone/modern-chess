-- What publishing a game keeps (builder slice 3a).
--
-- A published game is the same thing the catalogue already has: rules in `game_specs`, a page in
-- `game_pages` and a variant in `game_variants`. The slug ties them to the person's game. It is
-- made once, when the first revision is made, and never changes.
ALTER TABLE user_games
    ADD COLUMN slug        TEXT UNIQUE CHECK (slug ~ '^[a-z0-9-]{1,64}$'),
    -- What the page says, written when the game is handed in.
    ADD COLUMN tagline     TEXT CHECK (char_length(tagline) BETWEEN 1 AND 140),
    ADD COLUMN description TEXT CHECK (char_length(description) BETWEEN 1 AND 2000),
    -- The licence the author accepted, by its identifier.
    ADD COLUMN licence     TEXT,
    -- The author's handle when the game was handed in: staff approve it later, from their own
    -- token, and the page says "by" the author's name.
    ADD COLUMN author_handle TEXT;

CREATE FUNCTION user_games_keep_slug() RETURNS trigger AS $$
BEGIN
    IF OLD.slug IS NOT NULL AND NEW.slug IS DISTINCT FROM OLD.slug THEN
        RAISE EXCEPTION 'the slug of a game is never changed'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_games_slug_kept
    BEFORE UPDATE OF slug ON user_games
    FOR EACH ROW EXECUTE FUNCTION user_games_keep_slug();

-- One row for every time a game is published: a fixed thing that a fork can point at ("based on
-- revision 3"). Written once and never changed (decision 3 of the ownership plan). `author_handle`
-- is the author's handle at that moment, as `game_players` does for a seat.
CREATE TABLE user_game_revisions (
    game_id        UUID        NOT NULL REFERENCES user_games (id) ON DELETE RESTRICT,
    revision       INTEGER     NOT NULL CHECK (revision > 0),
    spec_id        UUID        NOT NULL REFERENCES game_specs (id),
    rules_hash     TEXT        NOT NULL,
    author_handle  TEXT        NOT NULL,
    name           TEXT        NOT NULL,
    tagline        TEXT        NOT NULL,
    description    TEXT        NOT NULL,
    skin           JSONB,
    licence        TEXT        NOT NULL,
    published_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (game_id, revision)
);

CREATE FUNCTION user_game_revisions_never_change() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'a published revision never changes'
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_game_revisions_fixed
    BEFORE UPDATE OR DELETE ON user_game_revisions
    FOR EACH ROW EXECUTE FUNCTION user_game_revisions_never_change();
