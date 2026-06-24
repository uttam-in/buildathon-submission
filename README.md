# Digital Menu Board Layout Renderer

**Live demo:** https://dmbr-16802728288.us-central1.run.app/ — public boards at `/`, admin console at `/admin`.

A deterministic Rust engine that turns a restaurant menu, a screen wall
configuration, and the current day-state into self-contained HTML/CSS menu
boards — one per physical screen — with a reproducible SHA-256 render hash.

Given the same inputs, it always produces byte-identical output. There are no
external fonts, scripts, or network calls: every rendered screen is a single
standalone HTML5 document safe to push to a digital-signage player.

## Architecture

```
                 ┌──────────────────────────────────────────────────┐
   FullMenu ────▶│                    dmbr-core                      │
 ScreenConfig ──▶│                                                    │
   DayState ────▶│  ┌──────────┐   ┌──────────────┐   ┌───────────┐  │
                 │  │ pipeline │──▶│    layout     │──▶│ renderer  │  │
                 │  │          │   │               │   │           │  │
                 │  │ • meal   │   │ • capacity    │   │ • HTML/CSS│  │
                 │  │   period │   │ • partitioner │   │   per     │  │
                 │  │ • filter │   │ • font        │   │   screen  │  │
                 │  │ • order  │   │ • balance     │   │           │  │
                 │  └──────────┘   └──────────────┘   └─────┬─────┘  │
                 │                                          │        │
                 │                                    ┌─────▼─────┐  │
                 │                                    │   hash    │  │
                 │                                    │ (SHA-256) │  │
                 │                                    └─────┬─────┘  │
                 └──────────────────────────────────────────┼───────┘
                                                             ▼
                                                       LayoutOutput
                                                       (JSON: per-screen
                                                        HTML + render_hash)
```

The `dmbr-cli` crate is a thin wrapper that reads the three JSON inputs (from
files or stdin) and prints the `LayoutOutput` JSON to stdout.

The `dmbr-convert` crate runs the engine directly on the **challenge input
format** (the `Resources/` files): it reads the nested `menu.json`, a
`configs/*.json` wall, and a `states/*.json` day-state, adapts them to the
engine schema (resolving out-of-stock and day/time category availability),
renders, and writes one standalone HTML file per screen plus an `index.html`
launcher. **This is the entry point for the live demo — see
[Running on the challenge data](#running-on-the-challenge-data).**

### Density and deterministic cycling

The full Saffron Junction menu (≈300 items) fits statically on the larger walls
(`wall`, `totem`) but is physically denser than a single landscape or 2-screen
wall can show at a legible size. For those configs the renderer splits each
over-dense screen into capacity-sized **pages** and cross-fades them on a fixed,
input-seeded CSS timeline (`@keyframes`, no JavaScript, no clock, no
randomness). Every available item is shown legibly during the cycle, nothing is
clipped, and the rendered HTML — and its `render_hash` — is byte-identical on
every run. See [`COSTS.md`](COSTS.md) for the economics (zero AI per change).

### Pipeline stages

1. **Meal period** — Resolves the active meal period from `DayState.timestamp`
   converted into the configured IANA timezone, matched against each
   `MealPeriodRule` time window (overnight windows supported). An explicit
   `active_meal_period` override in the day-state short-circuits detection.
2. **Filter** — Drops unavailable items, sold-out items, items outside the
   active meal period's applicable categories, then hides now-empty categories.
3. **Ordering** — Canonical sort (`display_order` then `id`) for both
   categories and items, guaranteeing determinism.

### Layout engine

1. **Capacity** — Converts each screen's pixel geometry into a slot budget
   (columns × items-per-column) from fixed margin/header/footer/slot constants.
2. **Partitioner** — Category-preserving greedy distribution of categories
   across screens, balancing by rendered weight and splitting oversized
   categories with a `(cont.)` marker.
3. **Font** — Negotiates a font size between a per-resolution preferred size and
   a hard floor (24px at ≥1080p), truncating overly long names with `…`.
4. **Balance** — Computes a `balance_score` (heaviest/lightest screen ratio) and
   rebalances whole categories until the score is acceptable or iterations run
   out.

## Crate overview

| Crate                | Kind    | Responsibility                                                              |
|----------------------|---------|-----------------------------------------------------------------------------|
| `dmbr-core`          | library | Data models, rules pipeline, layout engine, pagination, HTML renderer, hash.|
| `dmbr-cli`           | binary  | CLI front-end for the engine's native schema; prints `LayoutOutput` JSON.   |
| `dmbr-convert`       | binary  | Runs the engine on the **challenge** `Resources/` format; writes HTML files.|
| `dmbr-web`           | library | Shared web logic: render helpers, picker/gallery HTML, Postgres data layer (stores, screens, admin, menu), admin pages, and `build_full_menu` (DB → engine menu). Also the `dmbr-migrate` bin. |
| `dmbr-server-axum`   | binary  | **Axum** HTTP server: serves boards as live pages **and** the Postgres-backed admin UI for stores, screen monitors, and the menu. |
| `dmbr-server-actix`  | binary  | **Actix Web** HTTP server: serves the same boards (DB-free renderer demo).  |

## Build

```sh
cargo build --release
```

## Web servers

Two interchangeable HTTP servers render the boards as live webpages (read the
`Resources/` files fresh per request — edit a state and refresh to see the wall
reflow). Both share their logic via `dmbr-web`:

```sh
# Axum (default port 8080) — also hosts the admin UI (see below)
RESOURCES_DIR=../Resources cargo run -p dmbr-server-axum

# Actix Web (default port 8081) — DB-free renderer
RESOURCES_DIR=../Resources cargo run -p dmbr-server-actix
```

Routes (both servers): `GET /` (config picker) · `GET /config/{config}` ·
`GET /board/{config}/{state}` (gallery) ·
`GET /screen/{config}/{state}/{screen}` (one screen at native resolution).

## Admin app (Postgres-backed)

The Axum server adds a management UI, behind an admin login, to manage:

- **Stores** and their **screen monitors** — a store's set of monitors *is* its
  wall configuration. `GET /store/{slug}` renders that store's wall live.
- **The menu** — categories and items, prices (single or range), photos,
  per-category availability (time window + weekday set), a per-item **in-stock**
  toggle (the DB equivalent of 86'ing), and a per-item **featured** flag that
  drives the "Today's Features" rail.

**The database is the source of truth for the menu** (schema `menuboard`):
stores, screens, admin users, `menu_categories`, and `menu_items` all live in
Postgres. `Resources/menu.json` is only a one-time seed; day-states and wall
configs (`states/`, `configs/`) remain file-based.

**One-time setup** — create the schema, seed the menu from `menu.json`, and
seed an admin:

```sh
DATABASE_URL=postgres://… \
ADMIN_USER=admin ADMIN_PASSWORD=change-me \
MENU_JSON=../Resources/menu.json \
cargo run -p dmbr-web --bin dmbr-migrate
```

Idempotent: re-running re-applies the schema and re-hashes the admin password,
but skips menu seeding once the tables are populated. Migrations:
`migrations/0001_menuboard.sql` (stores, screens, admin_users),
`0002_menu.sql` (menu_categories, menu_items), `0003_featured.sql`
(featured flag). Passwords are Argon2-hashed.

**Run the server with the DB wired:**

```sh
DATABASE_URL=postgres://… \
SESSION_SECRET=a-long-random-string-at-least-32-chars \
RESOURCES_DIR=../Resources \
cargo run -p dmbr-server-axum
```

Then open `http://localhost:8080/admin` and sign in. Admin routes:
`/admin/login` · `/admin/logout` · `/admin/stores` (list/create) ·
`/admin/stores/{id}` (edit store + monitors) · `/admin/menu` (categories) ·
`/admin/menu/{id}` (edit a category + full item CRUD, incl. in-stock and
featured toggles). Editing the menu changes every store's wall on next refresh.
Sessions are HMAC-signed cookies; no server-side session store. The server
**refuses to start** unless `SESSION_SECRET` is set and at least 32 characters
(a short or absent secret would let anyone forge admin cookies).

### Today's Features

The featured rail prefers admin-flagged items (tagged **Chef's Special**), then
fills remaining slots from photo-bearing items in canonical order, capped at
three. Because filtering runs first, a flagged item that is sold out or out of
its availability window never appears — the fallback quietly fills its place, so
the rail stays full.

## Running on the challenge data

`dmbr-convert` is the demo entry point. Point it at the provided `Resources/`
files — any of the six configs, any state — and it writes one HTML file per
screen plus an `index.html` launcher, then (with `--open`) opens the launcher in
your browser. Each screen page is sized to that screen's exact resolution.

```sh
# from the buildathon-submission/ directory, after `cargo build --release`
./target/release/dmbr-convert \
  --menu   ../Resources/menu.json \
  --config ../Resources/configs/wall.json \
  --state  ../Resources/states/weekday-lunch-rush.json \
  --out    out/wall-lunch \
  --open
```

Swap `--config` for any of `solo | duo | wall | tower | twins | totem` and
`--state` for any of `weekday-morning | weekday-lunch-rush | weekend-evening`.
Open each `out/<dir>/screen-*.html` in a browser window at the screen's native
resolution (the launcher lists them with their dimensions).

| Flag             | Description                                            |
|------------------|--------------------------------------------------------|
| `--menu <file>`  | Challenge `menu.json`.                                 |
| `--config <file>`| A `configs/*.json` wall.                               |
| `--state <file>` | A `states/*.json` day-state.                           |
| `--out <dir>`    | Output directory (created if absent; default `out`).   |
| `--open`         | Open the generated `index.html` in the browser.        |

The same inputs always produce byte-identical HTML and the same `render_hash`
printed to stderr — run it twice and diff the files to confirm.

## Run (native schema)

The CLI accepts three input files:

```sh
dmbr-cli --menu menu.json --config screen.json --state day_state.json
```

Add `--pretty` to pretty-print the JSON output. You can also pipe a single
combined JSON object on stdin:

```sh
echo '{"menu":{...},"config":{...},"state":{...}}' | dmbr-cli
```

### CLI flags

| Flag              | Description                            |
|-------------------|----------------------------------------|
| `--menu <file>`   | Path to the `FullMenu` JSON file.      |
| `--config <file>` | Path to the `ScreenConfig` JSON file.  |
| `--state <file>`  | Path to the `DayState` JSON file.      |
| `--pretty`        | Pretty-print the output JSON.          |
| `--help`          | Show usage.                            |

Provide all three of `--menu`, `--config`, and `--state` together, or none of
them (in which case a single combined object is read from stdin). Errors are
written to stderr and the process exits with code `1`.

### Example input

`menu.json`

```json
{
  "restaurant_id": "store-042",
  "version": "1.0.0",
  "categories": [{ "id": "cat-burgers", "name": "Burgers", "display_order": 1 }],
  "items": [
    { "id": "item-001", "name": "Classic Cheeseburger", "price": 8.99, "category": "cat-burgers", "available": true, "display_order": 1 }
  ],
  "meal_period_rules": [
    { "name": "lunch", "start_time": "11:00", "end_time": "17:00", "applicable_categories": ["cat-burgers"] }
  ]
}
```

`screen.json`

```json
{
  "screen_count": 1,
  "arrangement": { "columns": 1, "rows": 1 },
  "screens": [
    { "id": "s0", "orientation": "landscape", "width_px": 1920, "height_px": 1080, "col": 0, "row": 0 }
  ]
}
```

`day_state.json`

```json
{
  "timestamp": "2026-06-18T11:05:00Z",
  "timezone": "America/Chicago",
  "sold_out_item_ids": [],
  "active_meal_period": null,
  "promotion_item_ids": ["item-001"]
}
```

### Example output

```json
{
  "restaurant_id": "store-042",
  "menu_version": "1.0.0",
  "active_meal_period": "lunch",
  "render_hash": "<sha256-hex>",
  "screens": [
    {
      "screen_id": "s0",
      "html_content": "<!DOCTYPE html>...",
      "item_ids": ["item-001"],
      "item_count": 1,
      "font_size_px": 28
    }
  ],
  "render_duration_ms": 0,
  "cache_hit": false,
  "fallback_used": false,
  "warnings": []
}
```

## Test

### Rust (unit + integration)

```sh
cargo test
```

Unit tests live alongside the modules (`#[cfg(test)]`) and an end-to-end
integration test lives in `crates/dmbr-core/tests/integration_test.rs`.

### Playwright (browser end-to-end)

`e2e/` holds a Playwright suite that drives the running server in a real
browser — the public renderer (all six configs, every-item-exactly-once,
determinism) and the admin app (auth guard, store + monitor CRUD, menu editor,
featured flag). Admin-app tests create throwaway records and clean them up, so
they don't disturb seeded data.

Start the DB-backed Axum server (see above), then:

```sh
cd e2e
npm install
npx playwright install chromium

# point the tests at the server; creds come from env (no secrets in the repo)
BASE_URL=http://localhost:8080 ADMIN_USER=admin ADMIN_PASSWORD=… npx playwright test
```

Or let Playwright boot the server itself by setting `START_SERVER=1` and
`DATABASE_URL` (see `e2e/playwright.config.ts`). `npm run report` opens the HTML
report.

## License

Business Source License 1.1 (BUSL-1.1) — see [LICENSE](LICENSE).

Free for development, testing, and other non-production use; production use
requires a commercial license from the Licensor. On the Change Date
(2028-06-24) the code converts to GPL v2 or later.
