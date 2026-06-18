# Digital Menu Board Layout Renderer

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

| Crate       | Kind    | Responsibility                                                        |
|-------------|---------|-----------------------------------------------------------------------|
| `dmbr-core` | library | Data models, rules pipeline, layout engine, HTML renderer, hashing.   |
| `dmbr-cli`  | binary  | CLI front-end: parses args/JSON, runs `dmbr-core`, prints the output. |

## Build

```sh
cargo build --release
```

## Run

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

```sh
cargo test
```

Unit tests live alongside the modules (`#[cfg(test)]`) and an end-to-end
integration test lives in `crates/dmbr-core/tests/integration_test.rs`.

## License

MIT
