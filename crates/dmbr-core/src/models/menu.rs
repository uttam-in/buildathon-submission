//! Menu input models: [`FullMenu`] and its constituent records.

use serde::{Deserialize, Serialize};

use crate::error::{RenderError, Result};

/// A single menu category (e.g. "Burgers").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MenuCategory {
    /// Stable category identifier, referenced by [`MenuItem::category`].
    pub id: String,
    /// Human-readable category name.
    pub name: String,
    /// Sort key; lower values render first.
    pub display_order: i64,
}

/// A single purchasable menu item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MenuItem {
    /// Stable item identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Price in the restaurant's currency; must be `>= 0.0`.
    pub price: f64,
    /// Identifier of the owning [`MenuCategory`].
    pub category: String,
    /// Whether the item is currently sellable.
    pub available: bool,
    /// Sort key within its category; lower values render first.
    pub display_order: i64,
    /// Optional short description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A time-window rule selecting which categories are shown during a meal period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MealPeriodRule {
    /// Meal period name (e.g. "breakfast", "lunch").
    pub name: String,
    /// Inclusive start time, `HH:MM` 24-hour clock, local to the day-state tz.
    pub start_time: String,
    /// Exclusive end time, `HH:MM`; if earlier than `start_time` the window is
    /// treated as overnight.
    pub end_time: String,
    /// Categories visible during this period. Empty means "all categories".
    #[serde(default)]
    pub applicable_categories: Vec<String>,
}

/// The complete menu for a restaurant at a given version.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FullMenu {
    /// Restaurant identifier, echoed into the output.
    pub restaurant_id: String,
    /// Menu version string, echoed into the output.
    pub version: String,
    /// All categories.
    pub categories: Vec<MenuCategory>,
    /// All items across all categories.
    pub items: Vec<MenuItem>,
    /// Meal-period selection rules.
    #[serde(default)]
    pub meal_period_rules: Vec<MealPeriodRule>,
}

impl FullMenu {
    /// Validates structural invariants that serde cannot express.
    ///
    /// Ensures every item price is non-negative.
    pub fn validate(&self) -> Result<()> {
        for item in &self.items {
            if !(item.price >= 0.0) {
                return Err(RenderError::InvalidInput(format!(
                    "item {} has invalid price {}",
                    item.id, item.price
                )));
            }
        }
        Ok(())
    }
}
