//! Canonical, deterministic ordering of categories and items.

use crate::models::{MenuCategory, MenuItem};

/// Sorts categories in place by `display_order` ascending, then `id` ascending
/// as a tiebreaker. This guarantees a stable, deterministic order regardless of
/// input ordering.
pub fn sort_categories(categories: &mut [MenuCategory]) {
    categories.sort_by(|a, b| {
        a.display_order
            .cmp(&b.display_order)
            .then_with(|| a.id.cmp(&b.id))
    });
}

/// Sorts items in place by `display_order` ascending, then `id` ascending as a
/// tiebreaker.
pub fn sort_items(items: &mut [MenuItem]) {
    items.sort_by(|a, b| {
        a.display_order
            .cmp(&b.display_order)
            .then_with(|| a.id.cmp(&b.id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(id: &str, order: i64) -> MenuCategory {
        MenuCategory {
            id: id.into(),
            name: id.into(),
            display_order: order,
        }
    }

    fn item(id: &str, order: i64) -> MenuItem {
        MenuItem {
            id: id.into(),
            name: id.into(),
            price: 1.0,
            category: "c".into(),
            available: true,
            display_order: order,
            description: None,
            price_display: None,
            image: None,
            featured: false,
        }
    }

    #[test]
    fn categories_sorted_by_order_then_id() {
        let mut cats = vec![cat("z", 2), cat("a", 1), cat("b", 1)];
        sort_categories(&mut cats);
        let ids: Vec<&str> = cats.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "z"]);
    }

    #[test]
    fn items_sorted_by_order_then_id() {
        let mut items = vec![item("c", 1), item("a", 1), item("b", 0)];
        sort_items(&mut items);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "a", "c"]);
    }

    #[test]
    fn deterministic_across_shuffled_inputs() {
        let mut a = vec![cat("a", 1), cat("b", 1), cat("c", 2)];
        let mut b = vec![cat("c", 2), cat("b", 1), cat("a", 1)];
        sort_categories(&mut a);
        sort_categories(&mut b);
        let ids_a: Vec<&str> = a.iter().map(|c| c.id.as_str()).collect();
        let ids_b: Vec<&str> = b.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids_a, ids_b);
    }
}
