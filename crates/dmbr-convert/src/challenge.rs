//! Input models for the *challenge* JSON format shipped in `Resources/`.
//!
//! These mirror the data the judges supply (`menu.json`, `configs/*.json`,
//! `states/*.json`) — a different shape from `dmbr-core`'s normalized schema.
//! [`crate::adapt`] converts these into the engine's models.

use serde::Deserialize;

/// A `{min, max}` price range (e.g. platters sold by size).
#[derive(Debug, Clone, Deserialize)]
pub struct PriceRange {
    /// Lowest price.
    pub min: f64,
    /// Highest price.
    pub max: f64,
}

/// A menu item in the challenge format.
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeItem {
    /// Stable item id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Single price (when the item is not range-priced).
    #[serde(default)]
    pub price: Option<f64>,
    /// Range price (when the item is sold by size).
    #[serde(default, rename = "priceRange")]
    pub price_range: Option<PriceRange>,
    /// Optional stock photo URL, carried through to the renderer for inline
    /// thumbnails and the featured rail.
    #[serde(default)]
    pub image: Option<String>,
    /// Optional short description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A category-level availability window.
#[derive(Debug, Clone, Deserialize)]
pub struct Availability {
    /// Inclusive start time `HH:MM`, local clock.
    #[serde(default)]
    pub from: Option<String>,
    /// Exclusive end time `HH:MM`, local clock.
    #[serde(default)]
    pub to: Option<String>,
    /// Days of the week the category is available (e.g. `["sat","sun"]`).
    /// Empty/absent means "every day".
    #[serde(default)]
    pub days: Vec<String>,
}

/// A menu category in the challenge format (categories own their items).
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeCategory {
    /// Stable category id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Items in this category, in authored order.
    #[serde(default)]
    pub items: Vec<ChallengeItem>,
    /// Optional availability window/day restriction.
    #[serde(default)]
    pub availability: Option<Availability>,
}

/// The full challenge menu.
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeMenu {
    /// Restaurant display name.
    #[serde(default)]
    pub restaurant: Option<String>,
    /// Currency code (informational; prices are rendered with a `$` prefix).
    #[serde(default)]
    #[allow(dead_code)]
    pub currency: Option<String>,
    /// All categories, each owning its items.
    pub categories: Vec<ChallengeCategory>,
}

/// A single screen in a challenge wall config.
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeScreen {
    /// Stable screen id.
    pub id: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `"landscape"` or `"portrait"`.
    pub orientation: String,
}

/// A challenge wall configuration (`configs/*.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeConfig {
    /// Config name (e.g. "wall").
    #[serde(default)]
    pub name: Option<String>,
    /// The screens on the wall.
    pub screens: Vec<ChallengeScreen>,
}

/// A challenge day-state (`states/*.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct ChallengeState {
    /// State name (e.g. "weekday-lunch-rush").
    #[serde(default)]
    pub name: Option<String>,
    /// Day of week, lowercase three-letter (e.g. "wed").
    pub day: String,
    /// Wall-clock time `HH:MM`.
    pub time: String,
    /// Item ids currently 86'd.
    #[serde(default, rename = "outOfStock")]
    pub out_of_stock: Vec<String>,
}
