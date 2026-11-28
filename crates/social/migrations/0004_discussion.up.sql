-- Discussion on a game's page (`docs/superpowers/specs/2026-11-28-discussion-design.md`).
--
-- A thread belongs to a game page by its slug. The social service has no games, so the slug is
-- not a foreign key: whether a page exists and is open is asked of the game service when a
-- thread is read or written. A thread's body is its first post (`opening`), so a thread is never
-- empty.
CREATE TABLE threads (
    id               UUID PRIMARY KEY,
    game_slug        TEXT        NOT NULL,
    author_id        UUID        NOT NULL REFERENCES users (id),
    title            TEXT        NOT NULL CHECK (char_length(title) BETWEEN 3 AND 120),
    created_at       TIMESTAMPTZ NOT NULL,
    last_activity_at TIMESTAMPTZ NOT NULL
);

-- The list on a game's page: newest activity first, a page at a time by keyset.
CREATE INDEX idx_threads_game_activity ON threads (game_slug, last_activity_at DESC, id DESC);

-- `seq` is the order posts were written in, and the cursor a thread is read by.
CREATE TABLE posts (
    seq        BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    id         UUID PRIMARY KEY,
    thread_id  UUID        NOT NULL REFERENCES threads (id),
    author_id  UUID        NOT NULL REFERENCES users (id),
    opening    BOOLEAN     NOT NULL,
    body       TEXT        NOT NULL CHECK (char_length(body) BETWEEN 1 AND 5000),
    created_at TIMESTAMPTZ NOT NULL,
    status     TEXT        NOT NULL DEFAULT 'VISIBLE'
               CHECK (status IN ('VISIBLE', 'HIDDEN', 'DELETED'))
);

CREATE INDEX idx_posts_thread ON posts (thread_id, seq);
-- The rate limit reads a person's latest post.
CREATE INDEX idx_posts_author_created ON posts (author_id, created_at DESC);
-- A thread has exactly one opening post.
CREATE UNIQUE INDEX uq_posts_opening ON posts (thread_id) WHERE opening;

-- Every change of a post's status, written with the change: who, when, from, to, and why.
CREATE TABLE post_status_history (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    post_id     UUID        NOT NULL REFERENCES posts (id),
    from_status TEXT        NOT NULL,
    to_status   TEXT        NOT NULL,
    actor       TEXT        NOT NULL,
    by_user     UUID        NOT NULL REFERENCES users (id),
    reason      TEXT,
    at          TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_post_status_history_post ON post_status_history (post_id, id);

-- The state machine of a post's status (`crates/social/src/domain/post.rs`) refuses an illegal move
-- in the application; this refuses it in the database too, so a bug cannot write one. Who may make
-- a move is the application's to decide: this only knows which moves exist. A deleted post never
-- comes back.
CREATE FUNCTION posts_check_status_move() RETURNS trigger AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;
    IF (OLD.status, NEW.status) IN (
        ('VISIBLE', 'HIDDEN'),
        ('VISIBLE', 'DELETED'),
        ('HIDDEN', 'VISIBLE')
    ) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'a post cannot move from % to %', OLD.status, NEW.status
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER posts_status_move
    BEFORE UPDATE OF status ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_check_status_move();

-- A post's words never change (editing is not built); only its status does.
CREATE FUNCTION posts_refuse_edit() RETURNS trigger AS $$
BEGIN
    IF NEW.body IS DISTINCT FROM OLD.body
       OR NEW.author_id IS DISTINCT FROM OLD.author_id
       OR NEW.thread_id IS DISTINCT FROM OLD.thread_id THEN
        RAISE EXCEPTION 'a post cannot be edited' USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER posts_no_edit
    BEFORE UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_refuse_edit();
