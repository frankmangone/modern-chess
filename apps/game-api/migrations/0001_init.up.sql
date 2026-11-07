CREATE TABLE IF NOT EXISTS game_specs (
    id UUID PRIMARY KEY,
    slug TEXT NOT NULL,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    rules_json JSONB NOT NULL,
    rules_hash TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (slug, version)
);

CREATE INDEX IF NOT EXISTS idx_game_specs_slug_active_version
    ON game_specs (slug, is_active, version DESC);

CREATE TABLE IF NOT EXISTS game_instances (
    id UUID PRIMARY KEY,
    spec_id UUID NOT NULL REFERENCES game_specs(id),
    spec_slug TEXT NOT NULL,
    spec_version INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('LOBBY', 'ACTIVE', 'FINISHED')),
    state_json JSONB NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_game_instances_spec_id ON game_instances (spec_id);
CREATE INDEX IF NOT EXISTS idx_game_instances_status_created_at ON game_instances (status, created_at DESC);

CREATE TABLE IF NOT EXISTS game_players (
    game_id UUID NOT NULL REFERENCES game_instances(id) ON DELETE CASCADE,
    seat TEXT NOT NULL,
    user_id TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, seat),
    UNIQUE (game_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_game_players_user_id ON game_players (user_id);

CREATE TABLE IF NOT EXISTS game_commands (
    game_id UUID NOT NULL REFERENCES game_instances(id) ON DELETE CASCADE,
    seq BIGINT NOT NULL,
    actor TEXT NOT NULL,
    command_type TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    accepted BOOLEAN NOT NULL,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (game_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_game_commands_game_created_at ON game_commands (game_id, created_at DESC);
