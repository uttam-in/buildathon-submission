//! Day-state input model: the live runtime context for a render.

use serde::{Deserialize, Serialize};

/// Runtime state for a single render: current time, sold-out items, and any
/// explicit overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DayState {
    /// Current instant as an ISO 8601 / RFC 3339 string.
    pub timestamp: String,
    /// IANA timezone name (e.g. "America/Chicago") used to localise
    /// [`timestamp`](Self::timestamp) for meal-period matching.
    pub timezone: String,
    /// Item IDs that are temporarily sold out and must be hidden.
    #[serde(default)]
    pub sold_out_item_ids: Vec<String>,
    /// Explicit meal-period override; when present, time-based detection is
    /// skipped entirely.
    #[serde(default)]
    pub active_meal_period: Option<String>,
    /// Item IDs to flag as promoted (highlighted in the rendered output).
    #[serde(default)]
    pub promotion_item_ids: Vec<String>,
}
