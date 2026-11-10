-- A signed-in account. The id stays ours: a provider's user id lives only in this table, so
-- nothing else in the product depends on which sign-in provider is in use.
CREATE TABLE identities (
    provider    TEXT        NOT NULL,
    subject     TEXT        NOT NULL,
    user_id     UUID        NOT NULL,
    -- A verified email, kept only so a later provider can be matched to existing users.
    email       TEXT,
    created_at  TIMESTAMPTZ NOT NULL,
    CONSTRAINT pk_identities PRIMARY KEY (provider, subject),
    CONSTRAINT uq_identities_user_provider UNIQUE (user_id, provider),
    CONSTRAINT fk_identities_user FOREIGN KEY (user_id) REFERENCES users (id)
);

-- A deleted account keeps its row (finished matches still point at it) but loses its
-- identities and its handle, and no longer counts as a user.
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;
