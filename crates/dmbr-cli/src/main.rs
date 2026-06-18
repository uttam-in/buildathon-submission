//! Command-line front-end for the Digital Menu Board Layout Renderer.
//!
//! Reads the three JSON inputs either from files (via flags) or as a single
//! combined object on stdin, runs the deterministic render pipeline, and prints
//! the resulting `LayoutOutput` as JSON to stdout.

use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use clap::Parser;
use serde::Deserialize;

use dmbr_core::models::{DayState, FullMenu, ScreenConfig};

/// Digital Menu Board Layout Renderer CLI.
#[derive(Parser, Debug)]
#[command(
    name = "dmbr-cli",
    version,
    about = "Render deterministic digital menu board layouts to HTML/CSS."
)]
struct Args {
    /// Path to the FullMenu JSON file.
    #[arg(long)]
    menu: Option<String>,

    /// Path to the ScreenConfig JSON file.
    #[arg(long)]
    config: Option<String>,

    /// Path to the DayState JSON file.
    #[arg(long)]
    state: Option<String>,

    /// Pretty-print the output JSON.
    #[arg(long)]
    pretty: bool,
}

/// Shape of the combined stdin payload: `{ "menu": ..., "config": ..., "state": ... }`.
#[derive(Deserialize)]
struct CombinedInput {
    menu: FullMenu,
    config: ScreenConfig,
    state: DayState,
}

/// Resolves the three inputs from either the file flags or a combined stdin
/// object. All three flags must be present together, or none (stdin mode).
fn load_inputs(args: &Args) -> Result<(FullMenu, ScreenConfig, DayState), String> {
    match (&args.menu, &args.config, &args.state) {
        (Some(menu_path), Some(config_path), Some(state_path)) => {
            let menu = read_json(menu_path)?;
            let config = read_json(config_path)?;
            let state = read_json(state_path)?;
            Ok((menu, config, state))
        }
        (None, None, None) => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("failed to read stdin: {e}"))?;
            let combined: CombinedInput = serde_json::from_str(&buf)
                .map_err(|e| format!("failed to parse stdin JSON: {e}"))?;
            Ok((combined.menu, combined.config, combined.state))
        }
        _ => Err(
            "provide all of --menu, --config, --state, or none (to read a combined object \
             from stdin)"
                .to_string(),
        ),
    }
}

/// Reads and deserializes a JSON file into `T`.
fn read_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("failed to read '{path}': {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("failed to parse '{path}': {e}"))
}

fn run() -> Result<(), String> {
    let args = Args::parse();
    let (menu, config, state) = load_inputs(&args)?;

    let output = dmbr_core::render(&menu, &config, &state).map_err(|e| e.to_string())?;

    let json = if args.pretty {
        serde_json::to_string_pretty(&output)
    } else {
        serde_json::to_string(&output)
    }
    .map_err(|e| format!("failed to serialize output: {e}"))?;

    println!("{json}");
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
