//! Pixel-to-slot capacity modelling for a single screen.

use crate::models::{Orientation, ScreenDef};

/// Outer margin around the content area, in pixels.
pub const MARGIN_PX: u32 = 32;
/// Reserved height for the screen header band.
pub const HEADER_HEIGHT_PX: u32 = 80;
/// Reserved height for the screen footer band.
pub const FOOTER_HEIGHT_PX: u32 = 40;
/// Vertical space a single item occupies (name + price line).
pub const ITEM_SLOT_HEIGHT_PX: u32 = 72;
/// Vertical space a category header occupies.
pub const CATEGORY_HEADER_HEIGHT_PX: u32 = 48;
/// Gutter between columns, in pixels.
pub const GUTTER_PX: u32 = 24;

/// The slot budget derived from a screen's geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacity {
    /// Number of content columns.
    pub column_count: u32,
    /// Pixel height available for content after subtracting chrome.
    pub usable_height: u32,
    /// Items that fit in one column.
    pub max_items_per_column: u32,
    /// Total item slots across all columns.
    pub total_slots: u32,
}

/// Computes the number of content columns for a screen from its orientation and
/// width.
fn column_count(screen: &ScreenDef) -> u32 {
    match screen.orientation {
        Orientation::Landscape => {
            if screen.width_px >= 1920 {
                3
            } else if screen.width_px >= 960 {
                2
            } else {
                1
            }
        }
        Orientation::Portrait => {
            if screen.width_px < 600 {
                1
            } else {
                2
            }
        }
    }
}

/// Computes the [`Capacity`] for a single screen.
///
/// The usable height saturates at zero for pathologically small screens so the
/// arithmetic never underflows.
pub fn compute_capacity(screen: &ScreenDef) -> Capacity {
    let chrome = HEADER_HEIGHT_PX + FOOTER_HEIGHT_PX + 2 * MARGIN_PX;
    let usable_height = screen.height_px.saturating_sub(chrome);
    let column_count = column_count(screen);
    let max_items_per_column = usable_height / ITEM_SLOT_HEIGHT_PX;
    let total_slots = column_count * max_items_per_column;
    Capacity {
        column_count,
        usable_height,
        max_items_per_column,
        total_slots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(o: Orientation, w: u32, h: u32) -> ScreenDef {
        ScreenDef {
            id: "s".into(),
            orientation: o,
            width_px: w,
            height_px: h,
            col: 0,
            row: 0,
        }
    }

    #[test]
    fn landscape_1080p_three_columns() {
        let c = compute_capacity(&screen(Orientation::Landscape, 1920, 1080));
        assert_eq!(c.column_count, 3);
        // usable = 1080 - 80 - 40 - 64 = 896; 896/72 = 12
        assert_eq!(c.usable_height, 896);
        assert_eq!(c.max_items_per_column, 12);
        assert_eq!(c.total_slots, 36);
    }

    #[test]
    fn landscape_midsize_two_columns() {
        let c = compute_capacity(&screen(Orientation::Landscape, 1280, 720));
        assert_eq!(c.column_count, 2);
    }

    #[test]
    fn portrait_narrow_one_column() {
        let c = compute_capacity(&screen(Orientation::Portrait, 540, 960));
        assert_eq!(c.column_count, 1);
    }

    #[test]
    fn tiny_screen_no_underflow() {
        let c = compute_capacity(&screen(Orientation::Landscape, 100, 50));
        assert_eq!(c.usable_height, 0);
        assert_eq!(c.total_slots, 0);
    }
}
