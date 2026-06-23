//! Splits a single screen's content into capacity-sized *pages* for
//! deterministic in-panel cycling.
//!
//! When a screen carries more items than it can show at once, the renderer
//! cycles through pages with a pure-CSS animation. Pagination here is a pure
//! function of the input (category order, item order, capacity), so the page
//! sequence — and therefore the rendered HTML and its hash — is deterministic.

use crate::pipeline::CategoryWithItems;

/// Per-page surcharge for a category header, in item-slot units, so a page's
/// header overhead is budgeted rather than ignored.
const HEADER_SLOT_COST: usize = 1;

/// Splits an oversized category into chunks of at most `chunk` items, marking
/// every chunk after the first as `continued`.
fn split_category(c: &CategoryWithItems, chunk: usize) -> Vec<CategoryWithItems> {
    let chunk = chunk.max(1);
    let mut out = Vec::new();
    let mut first = true;
    let mut start = 0;
    while start < c.items.len() {
        let end = (start + chunk).min(c.items.len());
        out.push(CategoryWithItems {
            category: c.category.clone(),
            items: c.items[start..end].to_vec(),
            continued: !first,
        });
        first = false;
        start = end;
    }
    out
}

/// Paginates a screen's category groups so each page costs at most
/// `slots_per_page` (items + per-header surcharge). Category grouping is
/// preserved; a category larger than a page is split with `(cont.)` markers.
///
/// Returns at least one page. When everything fits in one page the result is a
/// single page equal to the input.
pub fn paginate(
    groups: &[CategoryWithItems],
    slots_per_page: usize,
) -> Vec<Vec<CategoryWithItems>> {
    let cap = slots_per_page.max(1);

    // Expand oversized categories into page-sized fragments first.
    let mut units: Vec<CategoryWithItems> = Vec::new();
    for g in groups {
        let unit_cost = g.items.len() + HEADER_SLOT_COST;
        if unit_cost > cap {
            // Leave room for the header in each fragment.
            let chunk = cap.saturating_sub(HEADER_SLOT_COST).max(1);
            units.extend(split_category(g, chunk));
        } else {
            units.push(g.clone());
        }
    }

    let mut pages: Vec<Vec<CategoryWithItems>> = Vec::new();
    let mut current: Vec<CategoryWithItems> = Vec::new();
    let mut current_cost = 0usize;

    for unit in units {
        let cost = unit.items.len() + HEADER_SLOT_COST;
        if current_cost + cost > cap && !current.is_empty() {
            pages.push(std::mem::take(&mut current));
            current_cost = 0;
        }
        current_cost += cost;
        current.push(unit);
    }
    if !current.is_empty() {
        pages.push(current);
    }
    if pages.is_empty() {
        pages.push(Vec::new());
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MenuCategory, MenuItem};

    fn group(id: &str, n: usize) -> CategoryWithItems {
        CategoryWithItems {
            category: MenuCategory {
                id: id.into(),
                name: id.into(),
                display_order: 0,
            },
            items: (0..n)
                .map(|i| MenuItem {
                    id: format!("{id}-{i}"),
                    name: format!("{id} {i}"),
                    price: 1.0,
                    category: id.into(),
                    available: true,
                    display_order: i as i64,
                    description: None,
                    price_display: None,
                    image: None,
                    featured: false,
                })
                .collect(),
            continued: false,
        }
    }

    fn total_items(pages: &[Vec<CategoryWithItems>]) -> usize {
        pages.iter().flatten().map(|g| g.items.len()).sum()
    }

    #[test]
    fn single_page_when_it_all_fits() {
        let groups = vec![group("a", 3), group("b", 2)];
        let pages = paginate(&groups, 88);
        assert_eq!(pages.len(), 1);
        assert_eq!(total_items(&pages), 5);
    }

    #[test]
    fn splits_into_multiple_pages() {
        // 30 items, cap 10 (incl. header surcharge) -> several pages.
        let groups = vec![group("a", 30)];
        let pages = paginate(&groups, 10);
        assert!(pages.len() >= 3);
        assert_eq!(total_items(&pages), 30); // every item preserved exactly once
    }

    #[test]
    fn oversized_category_is_split_with_continued() {
        let groups = vec![group("big", 25)];
        let pages = paginate(&groups, 10);
        // First fragment keeps the header; later fragments are marked continued.
        let continued = pages.iter().flatten().filter(|g| g.continued).count();
        assert!(continued >= 1);
    }

    #[test]
    fn always_at_least_one_page() {
        let pages = paginate(&[], 88);
        assert_eq!(pages.len(), 1);
    }
}
