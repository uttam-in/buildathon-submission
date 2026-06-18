//! Screen-wall configuration models.

use serde::{Deserialize, Serialize};

use crate::error::{RenderError, Result};

/// Physical orientation of a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Wider than tall.
    Landscape,
    /// Taller than wide.
    Portrait,
}

/// Grid arrangement of the screen wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Arrangement {
    /// Number of columns in the wall grid.
    pub columns: u32,
    /// Number of rows in the wall grid.
    pub rows: u32,
}

/// A single physical screen and its position in the wall grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScreenDef {
    /// Stable screen identifier; output is sorted by this lexicographically.
    pub id: String,
    /// Screen orientation.
    pub orientation: Orientation,
    /// Width in pixels.
    pub width_px: u32,
    /// Height in pixels.
    pub height_px: u32,
    /// Grid column index.
    pub col: u32,
    /// Grid row index.
    pub row: u32,
}

/// The full screen-wall configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScreenConfig {
    /// Number of screens; only 1, 2, 4, 8 or 16 are supported.
    pub screen_count: u8,
    /// Grid arrangement.
    pub arrangement: Arrangement,
    /// The individual screens.
    pub screens: Vec<ScreenDef>,
}

impl ScreenConfig {
    /// Validates the configuration: supported screen count, matching screen
    /// list length, and positive pixel dimensions.
    pub fn validate(&self) -> Result<()> {
        if !matches!(self.screen_count, 1 | 2 | 3 | 4 | 8 | 16) {
            return Err(RenderError::InvalidInput(format!(
                "unsupported screen_count {} (allowed: 1, 2, 3, 4, 8, 16)",
                self.screen_count
            )));
        }
        if self.screens.len() != self.screen_count as usize {
            return Err(RenderError::InvalidInput(format!(
                "screen_count {} does not match screens array length {}",
                self.screen_count,
                self.screens.len()
            )));
        }
        for screen in &self.screens {
            if screen.width_px == 0 || screen.height_px == 0 {
                return Err(RenderError::InvalidInput(format!(
                    "screen {} has zero pixel dimension",
                    screen.id
                )));
            }
        }
        Ok(())
    }
}
