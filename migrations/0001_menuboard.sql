-- Schema for the menu-board admin app: stores, their screen monitors, and
-- admin users. Lives in its own `menuboard` schema, separate from `public`.

CREATE SCHEMA IF NOT EXISTS menuboard;

-- A restaurant location ("store-042"). Maps to the renderer's restaurantId.
CREATE TABLE IF NOT EXISTS menuboard.stores (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug        TEXT NOT NULL UNIQUE,            -- URL key, e.g. "store-042"
    name        TEXT NOT NULL,                   -- display name
    timezone    TEXT NOT NULL DEFAULT 'America/Chicago',
    state_key   TEXT NOT NULL DEFAULT 'weekday-lunch-rush', -- day-state to render
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A physical screen / TV at a store. A store's set of screens IS its wall
-- configuration for the renderer (orientation + resolution per screen).
CREATE TABLE IF NOT EXISTS menuboard.screens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id    UUID NOT NULL REFERENCES menuboard.stores(id) ON DELETE CASCADE,
    label       TEXT NOT NULL,                   -- e.g. "Counter Left"
    orientation TEXT NOT NULL CHECK (orientation IN ('landscape', 'portrait')),
    width_px    INTEGER NOT NULL CHECK (width_px > 0),
    height_px   INTEGER NOT NULL CHECK (height_px > 0),
    position    INTEGER NOT NULL DEFAULT 0,       -- ordering within the wall
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS screens_store_id_idx
    ON menuboard.screens (store_id, position);

-- Admin users for the management UI. Password is Argon2-hashed.
CREATE TABLE IF NOT EXISTS menuboard.admin_users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
