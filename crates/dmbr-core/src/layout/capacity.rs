//! Pixel-to-slot capacity modelling for a single screen.

use crate::models::{Orientation, ScreenDef};

/// Outer margin around the content area, in pixels.
pub const MARGIN_PX: u32 = 40;
/// Reserved height for the screen header band (brand + meal period).
pub const HEADER_HEIGHT_PX: u32 = 96;
/// Reserved height for the screen footer band.
pub const FOOTER_HEIGHT_PX: u32 = 36;
/// Vertical space a single item occupies (one name/price line, compact).
pub const ITEM_SLOT_HEIGHT_PX: u32 = 38;
/// Vertical space a category header occupies (including its bottom rule).
pub const CATEGORY_HEADER_HEIGHT_PX: u32 = 46;
/// Gutter between columns, in pixels.
pub const GUTTER_PX: u32 = 36;

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
                4
            } else if screen.width_px >= 1280 {
                3
            } else if screen.width_px >= 720 {
                2
            } else {
                1
            }
        }
        Orientation::Portrait => {
            if screen.width_px >= 1080 {
                3
            } else if screen.width_px >= 600 {
                2
            } else {
                1
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
    fn landscape_1080p_four_columns() {
        let c = compute_capacity(&screen(Orientation::Landscape, 1920, 1080));
        assert_eq!(c.column_count, 4);
        // usable = 1080 - 96 - 36 - 80 = 868; 868/38 = 22
        assert_eq!(c.usable_height, 868);
        assert_eq!(c.max_items_per_column, 22);
        assert_eq!(c.total_slots, 88);
    }

    #[test]
    fn portrait_1080p_three_columns() {
        let c = compute_capacity(&screen(Orientation::Portrait, 1080, 1920));
        assert_eq!(c.column_count, 3);
        // usable = 1920 - 96 - 36 - 80 = 1708; 1708/38 = 44
        assert_eq!(c.max_items_per_column, 44);
        assert_eq!(c.total_slots, 132);
    }

    #[test]
    fn landscape_midsize_three_columns() {
        let c = compute_capacity(&screen(Orientation::Landscape, 1280, 720));
        assert_eq!(c.column_count, 3);
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
