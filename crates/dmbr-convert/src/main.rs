//! `dmbr-convert` — runs the menu-board renderer on the *challenge* input
//! format (the `Resources/` files the judges supply).
//!
//! It reads the nested challenge `menu.json`, a `configs/*.json` wall, and a
//! `states/*.json` day-state, adapts them into `dmbr-core`'s normalized schema
//! (resolving out-of-stock and category availability deterministically), runs
//! the engine, and writes one standalone HTML file per screen plus an
//! `index.html` launcher. The render is byte-identical for identical inputs.

mod adapt;
mod challenge;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use serde::Deserialize;

use crate::adapt::adapt;
use crate::challenge::{ChallengeConfig, ChallengeMenu, ChallengeState};

/// Render menu boards from the challenge input format.
#[derive(Parser, Debug)]
#[command(name = "dmbr-convert", version, about)]
struct Args {
    /// Path to the challenge menu.json.
    #[arg(long)]
    menu: String,
    /// Path to a challenge config (configs/*.json).
    #[arg(long)]
    config: String,
    /// Path to a challenge state (states/*.json).
    #[arg(long)]
    state: String,
    /// Output directory for the rendered HTML (created if absent).
    #[arg(long, default_value = "out")]
    out: String,
    /// Open the generated index.html in the default browser when done.
    #[arg(long)]
    open: bool,
}

/// Reads and deserializes a JSON file into `T`.
fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("failed to read '{path}': {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("failed to parse '{path}': {e}"))
}

/// Slugifies a screen id into a safe filename stem.
fn safe_stem(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect()
}

/// Minimal HTML escaping for text interpolated into the launcher page.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Builds the launcher `index.html` linking each screen at its native size.
fn build_index(
    restaurant: &str,
    config_name: &str,
    state_name: &str,
    render_hash: &str,
    screens: &[(String, String, u32, u32)], // (screen_id, filename, w, h)
) -> String {
    let mut cards = String::new();
    for (id, file, w, h) in screens {
        cards.push_str(&format!(
            "<li><a href=\"./{file}\"><span class=\"sid\">{id}</span>\
             <span class=\"dim\">{w}×{h}</span></a></li>",
            file = file,
            id = html_escape(id),
            w = w,
            h = h
        ));
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{restaurant} — {config_name}</title>\n<style>\n\
*{{box-sizing:border-box;margin:0;padding:0;}}\n\
body{{background:#0b0b0c;color:#f4f4f5;font-family:system-ui,-apple-system,'Segoe UI',Arial,sans-serif;\
padding:48px;line-height:1.5;}}\n\
h1{{font-size:32px;font-weight:800;letter-spacing:-0.02em;}}\n\
.meta{{color:#9a9aa2;margin:8px 0 4px;}}\n\
.hash{{color:#6b6b73;font-family:'Courier New',monospace;font-size:13px;word-break:break-all;margin-bottom:32px;}}\n\
ul{{list-style:none;display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:16px;}}\n\
a{{display:flex;flex-direction:column;gap:6px;text-decoration:none;color:#f4f4f5;\
background:#161618;border:1px solid #262629;border-radius:14px;padding:22px;transition:border-color .15s,transform .15s;}}\n\
a:hover{{border-color:#d4773a;transform:translateY(-2px);}}\n\
.sid{{font-size:20px;font-weight:700;}}\n\
.dim{{color:#9a9aa2;font-family:'Courier New',monospace;font-size:14px;}}\n\
</style>\n</head>\n<body>\n\
<h1>{restaurant}</h1>\n\
<div class=\"meta\">config <b>{config_name}</b> · state <b>{state_name}</b> · {n} screen(s)</div>\n\
<div class=\"hash\">render_hash: {render_hash}</div>\n\
<ul>{cards}</ul>\n\
</body>\n</html>",
        restaurant = html_escape(restaurant),
        config_name = html_escape(config_name),
        state_name = html_escape(state_name),
        render_hash = render_hash,
        n = screens.len(),
        cards = cards,
    )
}

/// Opens `path` in the OS default browser (best-effort; ignored if it fails).
fn open_in_browser(path: &Path) {
    let p = path.to_string_lossy().to_string();
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", &p]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&p).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(&p).spawn()
    };
    if let Err(e) = result {
        eprintln!("note: could not open browser ({e}); open {p} manually");
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    let menu: ChallengeMenu = read_json(&args.menu)?;
    let config: ChallengeConfig = read_json(&args.config)?;
    let state: ChallengeState = read_json(&args.state)?;

    let config_name = config.name.clone().unwrap_or_else(|| "config".into());
    let state_name = state.name.clone().unwrap_or_else(|| "state".into());

    let adapted = adapt(&menu, &config, &state).map_err(|e| e.to_string())?;
    let output = dmbr_core::render(&adapted.menu, &adapted.config, &adapted.state)
        .map_err(|e| e.to_string())?;

    let out_dir = PathBuf::from(&args.out);
    fs::create_dir_all(&out_dir).map_err(|e| format!("failed to create '{}': {e}", args.out))?;

    let mut index_screens: Vec<(String, String, u32, u32)> = Vec::new();
    for screen in &output.screens {
        let stem = safe_stem(&screen.screen_id);
        let filename = format!("{stem}.html");
        let path = out_dir.join(&filename);
        fs::write(&path, &screen.html_content)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))?;
        // Look up the configured dimensions for the launcher card.
        let dims = adapted
            .config
            .screens
            .iter()
            .find(|s| s.id == screen.screen_id)
            .map(|s| (s.width_px, s.height_px))
            .unwrap_or((0, 0));
        index_screens.push((screen.screen_id.clone(), filename, dims.0, dims.1));
    }

    let index = build_index(
        &adapted.restaurant,
        &config_name,
        &state_name,
        &output.render_hash,
        &index_screens,
    );
    let index_path = out_dir.join("index.html");
    fs::write(&index_path, index)
        .map_err(|e| format!("failed to write '{}': {e}", index_path.display()))?;

    // Report a concise summary to stderr (stdout stays clean for piping).
    let total_items: usize = output.screens.iter().map(|s| s.item_count).sum();
    eprintln!(
        "rendered {} screen(s), {} items total — render_hash {}",
        output.screens.len(),
        total_items,
        output.render_hash
    );
    if output.fallback_used {
        eprintln!(
            "WARNING: capacity overflow — some screens hold more items than fit (fallback_used)"
        );
    }
    for w in &output.warnings {
        eprintln!("  [{:?}] {}: {}", w.level, w.code, w.message);
    }
    eprintln!("wrote {}", index_path.display());

    println!("{}", index_path.display());

    if args.open {
        open_in_browser(&index_path);
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
