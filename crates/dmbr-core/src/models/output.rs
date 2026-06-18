//! Output models: the [`LayoutOutput`] returned by the renderer.

use serde::{Deserialize, Serialize};

/// Severity of a [`Warning`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningLevel {
    /// Informational; output is fine.
    Info,
    /// Something was adjusted (e.g. truncation, rebalancing).
    Warning,
}

/// A non-fatal note emitted during rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Warning {
    /// Severity.
    pub level: WarningLevel,
    /// Stable machine code (e.g. "name_truncated").
    pub code: String,
    /// Human-readable detail.
    pub message: String,
}

impl Warning {
    /// Builds a [`WarningLevel::Warning`] note.
    pub fn warn(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: WarningLevel::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// The rendered result for a single screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RenderedScreen {
    /// Screen identifier this output belongs to.
    pub screen_id: String,
    /// Self-contained HTML5 document for the screen.
    pub html_content: String,
    /// Item IDs placed on this screen, in render order.
    pub item_ids: Vec<String>,
    /// Number of items on this screen.
    pub item_count: usize,
    /// Negotiated font size used for item text, in pixels.
    pub font_size_px: u32,
}

/// The complete output of a render across all screens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LayoutOutput {
    /// Restaurant identifier echoed from the menu.
    pub restaurant_id: String,
    /// Menu version echoed from the menu.
    pub menu_version: String,
    /// Active meal period resolved for this render, if any.
    pub active_meal_period: Option<String>,
    /// SHA-256 hex digest of all screens' HTML, in screen-id order.
    pub render_hash: String,
    /// Per-screen rendered output, sorted by screen id.
    pub screens: Vec<RenderedScreen>,
    /// Wall-clock render duration in milliseconds.
    pub render_duration_ms: u64,
    /// Whether the result came from a cache (always `false` here).
    pub cache_hit: bool,
    /// Whether a fallback layout was used due to capacity overflow.
    pub fallback_used: bool,
    /// Non-fatal warnings accumulated during rendering.
    pub warnings: Vec<Warning>,
}
