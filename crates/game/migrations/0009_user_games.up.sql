-- A game made by a person. A draft is private work in progress, and the only status that is
-- edited; the other statuses are for publishing, which comes later. `author_id` is an opaque id
-- from the social module: no foreign key, since that lives in another database.
-- `rules` and `skin` are text, not JSON: a draft may be half written.
CREATE TABLE user_games (
    id           UUID PRIMARY KEY,
    author_id    UUID        NOT NULL,
    name         TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 80),
    rules        TEXT        NOT NULL CHECK (octet_length(rules) <= 131072),
    skin         TEXT        NOT NULL CHECK (octet_length(skin) <= 32768),
    status       TEXT        NOT NULL CHECK (status IN ('DRAFT', 'PUBLISHED', 'UNPUBLISHED', 'REMOVED')),
    version      BIGINT      NOT NULL DEFAULT 1 CHECK (version > 0),
    -- The secret of a private link. Cleared to stop sharing.
    share_token  TEXT        UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL,
    updated_at   TIMESTAMPTZ NOT NULL
);

-- Serves "my drafts, newest first".
CREATE INDEX idx_user_games_author ON user_games (author_id, updated_at DESC);

-- Every status a game has had: who moved it, when, and from where (null for its creation).
-- A draft that is deleted takes its history with it.
CREATE TABLE user_game_status_history (
    id           BIGSERIAL PRIMARY KEY,
    game_id      UUID        NOT NULL REFERENCES user_games (id) ON DELETE CASCADE,
    from_status  TEXT,
    to_status    TEXT        NOT NULL,
    actor_id     UUID        NOT NULL,
    reason       TEXT,
    at           TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_user_game_status_history_game ON user_game_status_history (game_id, id);

-- The state machine of a game's status (`crates/game/src/domain/lifecycle.rs`) refuses an illegal
-- move in the application; this refuses it in the database too, so a bug cannot write one. Who may
-- make a move is the application's to decide: this only knows which moves exist.
CREATE FUNCTION user_games_check_status_move() RETURNS trigger AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;
    IF (OLD.status, NEW.status) IN (
        ('DRAFT', 'PUBLISHED'),
        ('PUBLISHED', 'UNPUBLISHED'),
        ('UNPUBLISHED', 'PUBLISHED'),
        ('PUBLISHED', 'REMOVED'),
        ('UNPUBLISHED', 'REMOVED'),
        ('REMOVED', 'UNPUBLISHED')
    ) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'a game cannot move from % to %', OLD.status, NEW.status
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER user_games_status_move
    BEFORE UPDATE OF status ON user_games
    FOR EACH ROW EXECUTE FUNCTION user_games_check_status_move();
