//! Category-preserving greedy partitioning of content across screens.

use crate::layout::capacity::{CATEGORY_HEADER_HEIGHT_PX, ITEM_SLOT_HEIGHT_PX};
use crate::pipeline::CategoryWithItems;

/// Overflow factor: a screen may exceed its target weight by up to 6% before
/// the partitioner rolls over to the next screen. Kept tight so no single
/// screen drifts far past its slot budget and clips.
const OVERFLOW_FACTOR_NUM: u32 = 106;
const OVERFLOW_FACTOR_DEN: u32 = 100;

/// Rendered weight of a category fragment in pixels: its header plus its items.
fn category_weight(c: &CategoryWithItems) -> u32 {
    CATEGORY_HEADER_HEIGHT_PX + c.items.len() as u32 * ITEM_SLOT_HEIGHT_PX
}

/// Splits a single oversized category into fragments that each fit within
/// `slots_per_screen` items. The first fragment keeps the original header; the
/// rest are marked `continued`.
fn split_category(c: &CategoryWithItems, slots_per_screen: u32) -> Vec<CategoryWithItems> {
    let chunk = slots_per_screen.max(1) as usize;
    let mut fragments = Vec::new();
    let mut first = true;
    let mut start = 0;
    while start < c.items.len() {
        let end = (start + chunk).min(c.items.len());
        fragments.push(CategoryWithItems {
            category: c.category.clone(),
            items: c.items[start..end].to_vec(),
            continued: !first,
        });
        first = false;
        start = end;
    }
    fragments
}

/// Distributes ordered categories across `screen_count` screens using a
/// category-preserving greedy algorithm.
///
/// Categories are kept intact where possible. A category that alone would not
/// fit on one screen (more items than `slots_per_screen`) is split into
/// continuation fragments. The greedy pass targets an even pixel weight per
/// screen and rolls over to the next screen once the current one exceeds
/// 1.15× the target (provided it already holds at least one category).
///
/// Every input item appears exactly once in the output.
pub fn partition(
    groups: &[CategoryWithItems],
    screen_count: usize,
    slots_per_screen: u32,
) -> Vec<Vec<CategoryWithItems>> {
    let mut screens: Vec<Vec<CategoryWithItems>> = vec![Vec::new(); screen_count.max(1)];
    if screen_count == 0 || groups.is_empty() {
        return screens;
    }

    // Expand oversized categories into per-screen-sized fragments first, so the
    // greedy pass works with placeable units.
    let mut units: Vec<CategoryWithItems> = Vec::new();
    for g in groups {
        if slots_per_screen > 0 && g.items.len() as u32 > slots_per_screen {
            units.extend(split_category(g, slots_per_screen));
        } else {
            units.push(g.clone());
        }
    }

    let total_weight: u32 = units.iter().map(category_weight).sum();
    let target = (total_weight / screen_count as u32).max(1);
    let overflow_limit = target * OVERFLOW_FACTOR_NUM / OVERFLOW_FACTOR_DEN;

    let mut current = 0usize;
    let mut current_weight = 0u32;

    for unit in units {
        let w = category_weight(&unit);
        let would_be = current_weight + w;
        let has_room_for_more_screens = current + 1 < screen_count;
        let current_non_empty = !screens[current].is_empty();

        if would_be > overflow_limit && current_non_empty && has_room_for_more_screens {
            current += 1;
            current_weight = 0;
        }

        current_weight += w;
        screens[current].push(unit);
    }

    screens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MenuCategory, MenuItem};

    fn group(cat: &str, n: usize) -> CategoryWithItems {
        let items = (0..n)
            .map(|i| MenuItem {
                id: format!("{cat}-{i:03}"),
                name: format!("item {i}"),
                price: 1.0,
                category: cat.into(),
                available: true,
                display_order: i as i64,
                description: None,
                price_display: None,
                image: None,
            })
            .collect();
        CategoryWithItems {
            category: MenuCategory {
                id: cat.into(),
                name: cat.into(),
                display_order: 1,
            },
            items,
            continued: false,
        }
    }

    fn count_items(screens: &[Vec<CategoryWithItems>]) -> usize {
        screens
            .iter()
            .flat_map(|s| s.iter())
            .map(|c| c.items.len())
            .sum()
    }

    fn collect_ids(screens: &[Vec<CategoryWithItems>]) -> Vec<String> {
        let mut ids: Vec<String> = screens
            .iter()
            .flat_map(|s| s.iter())
            .flat_map(|c| c.items.iter())
            .map(|i| i.id.clone())
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn ten_items_two_categories_two_screens_splits_evenly() {
        let groups = vec![group("a", 5), group("b", 5)];
        let screens = partition(&groups, 2, 12);
        assert_eq!(count_items(&screens), 10);
        assert_eq!(screens[0].iter().map(|c| c.items.len()).sum::<usize>(), 5);
        assert_eq!(screens[1].iter().map(|c| c.items.len()).sum::<usize>(), 5);
    }

    #[test]
    fn fifty_three_items_three_screens_all_assigned_no_duplicates() {
        let groups = vec![group("a", 20), group("b", 18), group("c", 15)];
        let screens = partition(&groups, 3, 12);
        assert_eq!(count_items(&screens), 53);
        let ids = collect_ids(&screens);
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "no duplicate item ids");
        assert_eq!(ids.len(), 53);
    }

    #[test]
    fn oversized_category_is_split_with_continuation() {
        let groups = vec![group("big", 30)];
        let screens = partition(&groups, 3, 12);
        assert_eq!(count_items(&screens), 30);
        // First fragment is not continued; later fragments are.
        let continued: Vec<bool> = screens
            .iter()
            .flat_map(|s| s.iter())
            .map(|c| c.continued)
            .collect();
        assert_eq!(continued.iter().filter(|&&b| !b).count(), 1);
        assert!(continued.iter().any(|&b| b));
    }
}
