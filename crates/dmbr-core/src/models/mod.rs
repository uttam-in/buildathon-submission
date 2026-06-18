//! Serde data models for inputs (menu, screen config, day state) and output.

pub mod day_state;
pub mod menu;
pub mod output;
pub mod screen;

pub use day_state::DayState;
pub use menu::{FullMenu, MealPeriodRule, MenuCategory, MenuItem};
pub use output::{LayoutOutput, RenderedScreen, Warning, WarningLevel};
pub use screen::{Arrangement, Orientation, ScreenConfig, ScreenDef};
