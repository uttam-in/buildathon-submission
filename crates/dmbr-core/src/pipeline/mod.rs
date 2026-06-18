//! Rules pipeline: meal-period detection, filtering, and canonical ordering.
//!
//! The pipeline transforms the raw menu plus day-state into an ordered list of
//! [`CategoryWithItems`] ready for layout.

pub mod filter;
pub mod meal_period;
pub mod ordering;

use crate::models::{MenuCategory, MenuItem};

pub use filter::{filter_menu, Filtered};
pub use meal_period::detect_meal_period;
pub use ordering::{sort_categories, sort_items};

/// A category paired with its items, in render order.
///
/// `continued` marks a category fragment produced when a single category is too
/// large for one screen and is split across screens with a "(cont.)" marker.
#[derive(Debug, Clone)]
pub struct CategoryWithItems {
    /// The category metadata.
    pub category: MenuCategory,
    /// The category's items, already sorted.
    pub items: Vec<MenuItem>,
    /// Whether this fragment is a continuation of a split category.
    pub continued: bool,
}

impl CategoryWithItems {
    /// Number of items in this fragment.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Groups filtered items under their categories and returns them in canonical
/// order. Categories are sorted by `display_order` then `id`; items within each
/// category are sorted the same way. Categories with no items are omitted.
pub fn build_ordered_groups(filtered: Filtered) -> Vec<CategoryWithItems> {
    let mut categories = filtered.categories;
    sort_categories(&mut categories);

    categories
        .into_iter()
        .filter_map(|category| {
            let mut items: Vec<MenuItem> = filtered
                .items
                .iter()
                .filter(|i| i.category == category.id)
                .cloned()
                .collect();
            if items.is_empty() {
                return None;
            }
            sort_items(&mut items);
            Some(CategoryWithItems {
                category,
                items,
                continued: false,
            })
        })
        .collect()
}
