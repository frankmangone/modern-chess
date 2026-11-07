-- One stored version of a game's rules. Immutable once written; only `active` ever changes.
CREATE TABLE game_specs (
    id          UUID PRIMARY KEY,
    slug        TEXT        NOT NULL,
    version     INTEGER     NOT NULL CHECK (version > 0),
    name        TEXT        NOT NULL,
    rules       JSONB       NOT NULL,
    rules_hash  TEXT        NOT NULL,
    active      BOOLEAN     NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL,
    UNIQUE (slug, version)
);

-- At most one active version per game.
CREATE UNIQUE INDEX uq_game_specs_one_active_per_slug ON game_specs (slug) WHERE active;

-- The tunable front page of a game. A game is listed if and only if it has a row here.
CREATE TABLE game_pages (
    slug         TEXT PRIMARY KEY,
    tagline      TEXT    NOT NULL,
    description  TEXT    NOT NULL,
    tags         TEXT[]  NOT NULL,
    accent       TEXT,
    cover_image  TEXT,
    skin         JSONB,
    sections     JSONB   NOT NULL,
    featured     BOOLEAN NOT NULL,
    sort_order   INTEGER NOT NULL
);

CREATE INDEX idx_game_pages_sort_order ON game_pages (sort_order, slug);

-- A lobby or a match. `spec_id` pins the exact rules it was started with.
CREATE TABLE game_instances (
    id            UUID PRIMARY KEY,
    spec_id       UUID        NOT NULL REFERENCES game_specs (id),
    slug          TEXT        NOT NULL,
    spec_version  INTEGER     NOT NULL,
    status        TEXT        NOT NULL CHECK (status IN ('LOBBY', 'ACTIVE', 'FINISHED')),
    seats         TEXT[]      NOT NULL,
    state         JSONB       NOT NULL,
    revision      BIGINT      NOT NULL DEFAULT 0 CHECK (revision >= 0),
    outcome       JSONB,
    created_at    TIMESTAMPTZ NOT NULL,
    started_at    TIMESTAMPTZ,
    finished_at   TIMESTAMPTZ
);

CREATE INDEX idx_game_instances_spec_id ON game_instances (spec_id);
-- Serves "open lobbies of this game, newest first".
CREATE INDEX idx_game_instances_slug_status_created ON game_instances (slug, status, created_at DESC);

-- Who sits where. `user_id` is an opaque id from the social module: no foreign key, since
-- that lives in another database. `handle` is the copy taken from the token at join time.
CREATE TABLE game_players (
    instance_id  UUID        NOT NULL REFERENCES game_instances (id) ON DELETE CASCADE,
    seat         TEXT        NOT NULL,
    user_id      UUID        NOT NULL,
    handle       TEXT        NOT NULL,
    joined_at    TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (instance_id, seat),
    UNIQUE (instance_id, user_id)
);

CREATE INDEX idx_game_players_user_id ON game_players (user_id);

-- Append-only log of what players asked for, accepted or not.
CREATE TABLE game_commands (
    instance_id  UUID        NOT NULL REFERENCES game_instances (id) ON DELETE CASCADE,
    seq          BIGINT      NOT NULL CHECK (seq > 0),
    actor        UUID        NOT NULL,
    kind         JSONB       NOT NULL,
    accepted     BOOLEAN     NOT NULL,
    error        TEXT,
    created_at   TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (instance_id, seq)
);

CREATE INDEX idx_game_commands_actor ON game_commands (actor);
