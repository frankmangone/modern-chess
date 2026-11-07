-- A person using the product. In v1 every user is a guest.
CREATE TABLE users (
    id          UUID PRIMARY KEY,
    handle      TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL
);

-- Handles are unique regardless of case, so "Guest-1" and "guest-1" cannot both exist.
CREATE UNIQUE INDEX uq_users_handle_lower ON users (lower(handle));
