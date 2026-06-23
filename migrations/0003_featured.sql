-- Adds an admin-controlled "feature this item" flag. The renderer's
-- "Today's Features" rail prefers flagged, currently-available items and falls
-- back to photo-bearing items in order when there aren't enough.
ALTER TABLE menuboard.menu_items
    ADD COLUMN IF NOT EXISTS featured BOOLEAN NOT NULL DEFAULT FALSE;
