//! Framework-agnostic web logic for serving menu boards as live webpages.
//!
//! This crate holds everything the HTTP servers (`dmbr-server-axum` /
//! `dmbr-server-actix`) need, so each server is a thin routing shim:
//!
//! - discover the available configs and states under a `Resources/` directory,
//! - load + adapt + render a board for a `(config, state)` pair (no database;
//!   files are read fresh on each request so editing a state and refreshing
//!   reflects the change live),
//! - build the picker and gallery HTML pages.
//!
//! All rendering reuses `dmbr-core` and the `dmbr-convert` adapter verbatim, so
//! the served pages are byte-identical to the CLI's output for the same inputs.

pub mod admin_pages;
pub mod db;

use std::path::{Path, PathBuf};

use dmbr_convert::adapt::adapt;
use dmbr_convert::challenge::{ChallengeConfig, ChallengeMenu, ChallengeScreen, ChallengeState};
use dmbr_core::models::LayoutOutput;

/// An error serving a board, with an HTTP-friendly classification.
#[derive(Debug)]
pub enum WebError {
    /// The requested config/state/screen does not exist (HTTP 404).
    NotFound(String),
    /// An input file was malformed or rendering failed (HTTP 500).
    Internal(String),
}

impl std::fmt::Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebError::NotFound(m) => write!(f, "not found: {m}"),
            WebError::Internal(m) => write!(f, "error: {m}"),
        }
    }
}

impl std::error::Error for WebError {}

/// A discovered config or state: its URL key (the file stem) and display name.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Entry {
    /// URL key, e.g. `wall` or `weekday-lunch-rush` (the filename without
    /// extension).
    pub key: String,
    /// Human-readable name from the JSON `name` field, falling back to `key`.
    pub name: String,
}

/// The set of configs and states available under a Resources directory.
#[derive(Debug, Clone)]
pub struct Catalog {
    /// Wall configs (from `configs/*.json`), sorted by key.
    pub configs: Vec<Entry>,
    /// Day-states (from `states/*.json`), sorted by key.
    pub states: Vec<Entry>,
}

/// Holds the Resources directory and serves boards from it.
#[derive(Debug, Clone)]
pub struct Resources {
    root: PathBuf,
}

/// Reads the optional top-level `"name"` string from a JSON file (async,
/// non-blocking).
async fn read_name(path: &Path) -> Option<String> {
    let text = tokio::fs::read_to_string(path).await.ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("name")?.as_str().map(|s| s.to_string())
}

/// Lists `*.json` files in `dir` as [`Entry`]s, sorted by key (async,
/// non-blocking).
async fn list_entries(dir: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(mut read) = tokio::fs::read_dir(dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = read.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let name = read_name(&path).await.unwrap_or_else(|| stem.to_string());
        out.push(Entry {
            key: stem.to_string(),
            name,
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

impl Resources {
    /// Creates a handle for the Resources directory at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Discovers the available configs and states (async, non-blocking).
    pub async fn catalog(&self) -> Catalog {
        Catalog {
            configs: list_entries(&self.root.join("configs")).await,
            states: list_entries(&self.root.join("states")).await,
        }
    }

    /// Loads `menu.json`, the named config, and the named state, then adapts and
    /// renders the full board. Files are read fresh on every call.
    pub async fn render(
        &self,
        config_key: &str,
        state_key: &str,
    ) -> Result<LayoutOutput, WebError> {
        if !valid_key(config_key) {
            return Err(WebError::NotFound(format!("config '{config_key}'")));
        }
        if !valid_key(state_key) {
            return Err(WebError::NotFound(format!("state '{state_key}'")));
        }
        let menu: ChallengeMenu = self.load_json(&self.root.join("menu.json")).await?;
        let config: ChallengeConfig = self
            .load_json(&self.root.join("configs").join(format!("{config_key}.json")))
            .await?;
        let state: ChallengeState = self
            .load_json(&self.root.join("states").join(format!("{state_key}.json")))
            .await?;

        let adapted = adapt(&menu, &config, &state)
            .map_err(|e| WebError::Internal(format!("adapt failed: {e}")))?;
        dmbr_core::render(&adapted.menu, &adapted.config, &adapted.state)
            .map_err(|e| WebError::Internal(format!("render failed: {e}")))
    }

    /// Renders a board and returns the HTML for a single screen by id.
    pub async fn render_screen(
        &self,
        config_key: &str,
        state_key: &str,
        screen_id: &str,
    ) -> Result<String, WebError> {
        let output = self.render(config_key, state_key).await?;
        output
            .screens
            .into_iter()
            .find(|s| s.screen_id == screen_id)
            .map(|s| s.html_content)
            .ok_or_else(|| WebError::NotFound(format!("screen '{screen_id}'")))
    }

    /// Renders a board for a *store*: the wall is built from the store's screen
    /// monitors (from the DB), and the day-state comes from the store's
    /// `state_key`. `screens` is `(id, orientation, width_px, height_px)` in
    /// wall order. Falls back to the menu's restaurant name in the header via
    /// the adapter, but callers can override the display title afterward.
    pub async fn render_store_screens(
        &self,
        state_key: &str,
        screens: &[(String, String, u32, u32)],
    ) -> Result<LayoutOutput, WebError> {
        if !valid_key(state_key) {
            return Err(WebError::NotFound(format!("state '{state_key}'")));
        }
        let menu: ChallengeMenu = self.load_json(&self.root.join("menu.json")).await?;
        let state: ChallengeState = self
            .load_json(&self.root.join("states").join(format!("{state_key}.json")))
            .await?;

        let config = ChallengeConfig {
            name: Some("store".into()),
            screens: screens
                .iter()
                .map(|(id, orientation, w, h)| ChallengeScreen {
                    id: id.clone(),
                    width: *w,
                    height: *h,
                    orientation: orientation.clone(),
                })
                .collect(),
        };

        let adapted = adapt(&menu, &config, &state)
            .map_err(|e| WebError::Internal(format!("adapt failed: {e}")))?;
        dmbr_core::render(&adapted.menu, &adapted.config, &adapted.state)
            .map_err(|e| WebError::Internal(format!("render failed: {e}")))
    }

    /// Renders a store's wall using the **database** menu (DB is the source of
    /// truth). The day-state (`day` + `time`) is still read from the store's
    /// `state_key` file; the menu, prices, photos, availability and in-stock
    /// flags all come from Postgres. `screens` is `(screen_id, orientation, w, h)`
    /// in wall order.
    pub async fn render_store_from_db(
        &self,
        pool: &sqlx::PgPool,
        state_key: &str,
        restaurant_id: &str,
        screens: &[(String, String, u32, u32)],
    ) -> Result<LayoutOutput, WebError> {
        if !valid_key(state_key) {
            return Err(WebError::NotFound(format!("state '{state_key}'")));
        }
        let state: ChallengeState = self
            .load_json(&self.root.join("states").join(format!("{state_key}.json")))
            .await?;

        let menu = crate::db::build_full_menu(pool, restaurant_id, &state.day, &state.time)
            .await
            .map_err(|e| WebError::Internal(format!("db menu: {e}")))?;

        let config = build_screen_config(screens)
            .map_err(|e| WebError::Internal(format!("bad screen config: {e}")))?;

        dmbr_core::render(&menu, &config, &db_day_state(&state.out_of_stock))
            .map_err(|e| WebError::Internal(format!("render failed: {e}")))
    }

    /// Reads and deserializes a JSON file (async, non-blocking), mapping a
    /// missing file to a 404.
    async fn load_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &Path,
    ) -> Result<T, WebError> {
        let text = tokio::fs::read_to_string(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WebError::NotFound(format!("{}", path.display()))
            } else {
                WebError::Internal(format!("read {}: {e}", path.display()))
            }
        })?;
        serde_json::from_str(&text)
            .map_err(|e| WebError::Internal(format!("parse {}: {e}", path.display())))
    }
}

/// HTML-escapes text for safe interpolation into HTML markup and attribute
/// values. Escapes all five significant characters, including the single quote
/// (`'`), so values are safe inside single-quoted attributes and inline JS
/// string arguments. Delegates to the engine's `escape_html` for one source of
/// truth.
pub(crate) fn esc(s: &str) -> String {
    dmbr_core::escape_html(s)
}

/// Shared page chrome: a dark page with a title, wrapping `body`.
pub(crate) fn page(title: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{title}</title>\n<style>\n\
*{{box-sizing:border-box;margin:0;padding:0;}}\n\
body{{background:radial-gradient(120% 90% at 0% 0%,#1c140e 0%,#0e0b09 55%,#070605 100%);\
color:#f3ece1;font-family:system-ui,-apple-system,'Segoe UI',Arial,sans-serif;\
min-height:100vh;padding:56px;line-height:1.5;}}\n\
h1{{font-size:34px;font-weight:800;letter-spacing:-0.02em;color:#fbf5ea;}}\n\
h2{{font-size:14px;font-weight:700;text-transform:uppercase;letter-spacing:0.16em;\
color:#e7b15a;margin:32px 0 14px;}}\n\
.sub{{color:#9a8f7d;margin-top:6px;}}\n\
a{{color:#f4c87a;text-decoration:none;}}\n\
.grid{{list-style:none;display:grid;\
grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:14px;}}\n\
.card{{display:flex;flex-direction:column;gap:6px;background:#181410;\
border:1px solid #3a2e1f;border-radius:14px;padding:20px;\
transition:border-color .15s,transform .15s;}}\n\
.card:hover{{border-color:#c8862f;transform:translateY(-2px);}}\n\
.card .k{{font-size:19px;font-weight:700;color:#fbf5ea;}}\n\
.card .d{{font-size:13px;color:#9a8f7d;font-family:'Courier New',monospace;}}\n\
.crumbs{{margin-bottom:8px;color:#9a8f7d;font-size:14px;}}\n\
.hash{{color:#6b6b73;font-family:'Courier New',monospace;font-size:12px;\
word-break:break-all;margin-top:6px;}}\n\
</style>\n</head>\n<body>\n{body}\n</body>\n</html>",
        title = esc(title),
        body = body,
    )
}

/// Builds the home picker page: pick a config, then a state.
pub fn picker_page(catalog: &Catalog) -> String {
    let mut configs = String::new();
    for c in &catalog.configs {
        configs.push_str(&format!(
            "<li><a class=\"card\" href=\"/config/{key}\"><span class=\"k\">{name}</span>\
             <span class=\"d\">{key}</span></a></li>",
            key = esc(&c.key),
            name = esc(&c.name),
        ));
    }
    let body = format!(
        "<h1>Saffron Junction — Menu Boards</h1>\
<p class=\"sub\">Pick a wall configuration, then a day-state. Files are read \
live from <code>Resources/</code> — edit a state and refresh.</p>\
<h2>Wall configurations</h2><ul class=\"grid\">{configs}</ul>",
        configs = configs
    );
    page("Menu Boards", &body)
}

/// Builds the per-config page: pick a state for the chosen config.
pub fn config_page(config: &Entry, catalog: &Catalog) -> String {
    let mut states = String::new();
    for s in &catalog.states {
        states.push_str(&format!(
            "<li><a class=\"card\" href=\"/board/{cfg}/{skey}\"><span class=\"k\">{sname}</span>\
             <span class=\"d\">{skey}</span></a></li>",
            cfg = esc(&config.key),
            skey = esc(&s.key),
            sname = esc(&s.name),
        ));
    }
    let body = format!(
        "<div class=\"crumbs\"><a href=\"/\">← all configs</a></div>\
<h1>{name}</h1><p class=\"sub\">Choose a day-state to render this wall.</p>\
<h2>Day states</h2><ul class=\"grid\">{states}</ul>",
        name = esc(&config.name),
        states = states,
    );
    page(&config.name, &body)
}

/// Builds the gallery page for a rendered board: one card per screen linking to
/// the full-resolution screen page.
pub fn gallery_page(config: &Entry, state: &Entry, output: &LayoutOutput) -> String {
    let mut cards = String::new();
    for screen in &output.screens {
        cards.push_str(&format!(
            "<li><a class=\"card\" target=\"_blank\" \
href=\"/screen/{cfg}/{st}/{sid}\"><span class=\"k\">{sid}</span>\
<span class=\"d\">{count} items</span></a></li>",
            cfg = esc(&config.key),
            st = esc(&state.key),
            sid = esc(&screen.screen_id),
            count = screen.item_count,
        ));
    }
    let period = output.active_meal_period.as_deref().unwrap_or("—");
    let body = format!(
        "<div class=\"crumbs\"><a href=\"/\">all configs</a> · \
<a href=\"/config/{cfg}\">{cname}</a></div>\
<h1>{cname} · {sname}</h1>\
<p class=\"sub\">{n} screen(s) · meal period: {period} · open each screen at its \
native resolution.</p>\
<p class=\"hash\">render_hash: {hash}</p>\
<h2>Screens</h2><ul class=\"grid\">{cards}</ul>",
        cfg = esc(&config.key),
        cname = esc(&config.name),
        sname = esc(&state.name),
        n = output.screens.len(),
        period = esc(period),
        hash = esc(&output.render_hash),
        cards = cards,
    );
    page(&format!("{} · {}", config.name, state.name), &body)
}

/// Looks up an [`Entry`] by key in a slice, for resolving display names.
pub fn find_entry<'a>(entries: &'a [Entry], key: &str) -> Option<&'a Entry> {
    entries.iter().find(|e| e.key == key)
}

/// Whether a config/state key is a safe single path segment: non-empty and made
/// only of ASCII alphanumerics, `-`, or `_`. Rejects anything that could
/// traverse the filesystem (`/`, `\`, `.`, absolute paths) before it is joined
/// into a `Resources/` path.
fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Builds an engine `ScreenConfig` from `(screen_id, orientation, w, h)` tuples
/// laid out in a single row (every provided wall is one orientation).
fn build_screen_config(
    screens: &[(String, String, u32, u32)],
) -> Result<dmbr_core::models::ScreenConfig, String> {
    use dmbr_core::models::{Arrangement, Orientation, ScreenConfig, ScreenDef};
    let mut defs = Vec::with_capacity(screens.len());
    for (i, (id, orientation, w, h)) in screens.iter().enumerate() {
        let o = match orientation.to_ascii_lowercase().as_str() {
            "landscape" => Orientation::Landscape,
            "portrait" => Orientation::Portrait,
            other => return Err(format!("bad orientation '{other}'")),
        };
        defs.push(ScreenDef {
            id: id.clone(),
            orientation: o,
            width_px: *w,
            height_px: *h,
            col: i as u32,
            row: 0,
        });
    }
    Ok(ScreenConfig {
        screen_count: screens.len() as u8,
        arrangement: Arrangement {
            columns: (screens.len() as u32).max(1),
            rows: 1,
        },
        screens: defs,
    })
}

/// Builds a `DayState` for the DB render path. Category availability and in
/// -stock are already resolved when building the menu, so meal-period detection
/// is short-circuited with an explicit `all` period. Any extra ids in the
/// state file's `outOfStock` still hide those items (belt-and-suspenders).
fn db_day_state(out_of_stock: &[String]) -> dmbr_core::models::DayState {
    dmbr_core::models::DayState {
        timestamp: "1970-01-01T00:00:00Z".into(),
        timezone: "UTC".into(),
        sold_out_item_ids: out_of_stock.to_vec(),
        active_meal_period: Some("all".into()),
        promotion_item_ids: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_escapes_markup() {
        assert_eq!(esc("a<b>&\"c"), "a&lt;b&gt;&amp;&quot;c");
    }

    #[test]
    fn page_wraps_body() {
        let html = page("T", "<p>hi</p>");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>T</title>"));
        assert!(html.contains("<p>hi</p>"));
    }

    #[test]
    fn picker_lists_configs() {
        let cat = Catalog {
            configs: vec![Entry {
                key: "wall".into(),
                name: "wall".into(),
            }],
            states: vec![],
        };
        let html = picker_page(&cat);
        assert!(html.contains("/config/wall"));
        assert!(html.contains("wall"));
    }
}
