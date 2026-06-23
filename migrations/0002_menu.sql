-- Menu catalog in the database: categories and items. The renderer reads the
-- menu from here (DB is source of truth); `Resources/menu.json` is only used to
-- seed it once. Day-states (configs/states JSON) remain file-based.

-- A menu category, e.g. "Biryani's". `slug` is the stable key from menu.json.
-- Availability is per-category: a time window (from/to, HH:MM) and/or a set of
-- weekday codes; null/empty means always available.
CREATE TABLE IF NOT EXISTS menuboard.menu_categories (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT NOT NULL UNIQUE,           -- e.g. "biryani-s"
    name          TEXT NOT NULL,
    position      INTEGER NOT NULL DEFAULT 0,     -- display order
    avail_from    TEXT,                            -- "HH:MM" or NULL
    avail_to      TEXT,                            -- "HH:MM" or NULL
    avail_days    TEXT[] NOT NULL DEFAULT '{}',    -- e.g. {sat,sun}; empty = any
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A menu item. Price is either a single value (price_max NULL) or a range
-- (price_min..price_max). `in_stock = false` is the DB equivalent of 86'ing.
CREATE TABLE IF NOT EXISTS menuboard.menu_items (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category_id   UUID NOT NULL REFERENCES menuboard.menu_categories(id) ON DELETE CASCADE,
    slug          TEXT NOT NULL UNIQUE,           -- e.g. "chicken-dum-biryani"
    name          TEXT NOT NULL,
    price_min     NUMERIC(10,2) NOT NULL CHECK (price_min >= 0),
    price_max     NUMERIC(10,2) CHECK (price_max IS NULL OR price_max >= price_min),
    image         TEXT,                            -- photo URL or NULL
    description   TEXT,
    in_stock      BOOLEAN NOT NULL DEFAULT TRUE,
    position      INTEGER NOT NULL DEFAULT 0,      -- order within category
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS menu_items_category_idx
    ON menuboard.menu_items (category_id, position);
