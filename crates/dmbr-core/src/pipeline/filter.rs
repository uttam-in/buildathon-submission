//! Availability, sold-out, and meal-period filtering.

use std::collections::HashSet;

use crate::models::{MealPeriodRule, MenuCategory, MenuItem};

/// Result of filtering: the surviving items and categories.
#[derive(Debug, Clone)]
pub struct Filtered {
    /// Items that survived all filters.
    pub items: Vec<MenuItem>,
    /// Categories that still have at least one surviving item.
    pub categories: Vec<MenuCategory>,
}

/// Filters items and categories for a render.
///
/// Removes items that are unavailable or sold out, restricts items to the
/// categories applicable to `active_meal_period` (when that period has a
/// non-empty `applicable_categories` list), then drops any category left with
/// no items ("empty category hiding").
pub fn filter_menu(
    items: &[MenuItem],
    categories: &[MenuCategory],
    sold_out_item_ids: &[String],
    active_meal_period: Option<&str>,
    rules: &[MealPeriodRule],
) -> Filtered {
    let sold_out: HashSet<&str> = sold_out_item_ids.iter().map(String::as_str).collect();

    // Determine the set of allowed categories for the active meal period.
    // None => no restriction; Some(set) => only these categories are visible.
    let allowed_categories: Option<HashSet<&str>> = active_meal_period.and_then(|period| {
        rules
            .iter()
            .find(|r| r.name == period)
            .filter(|r| !r.applicable_categories.is_empty())
            .map(|r| r.applicable_categories.iter().map(String::as_str).collect())
    });

    let kept_items: Vec<MenuItem> = items
        .iter()
        .filter(|item| item.available)
        .filter(|item| !sold_out.contains(item.id.as_str()))
        .filter(|item| match &allowed_categories {
            Some(allowed) => allowed.contains(item.category.as_str()),
            None => true,
        })
        .cloned()
        .collect();

    let non_empty: HashSet<&str> = kept_items.iter().map(|i| i.category.as_str()).collect();

    let kept_categories: Vec<MenuCategory> = categories
        .iter()
        .filter(|c| non_empty.contains(c.id.as_str()))
        .cloned()
        .collect();

    Filtered {
        items: kept_items,
        categories: kept_categories,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, cat: &str, available: bool) -> MenuItem {
        MenuItem {
            id: id.into(),
            name: id.into(),
            price: 1.0,
            category: cat.into(),
            available,
            display_order: 1,
            description: None,
            price_display: None,
            image: None,
        }
    }

    fn cat(id: &str) -> MenuCategory {
        MenuCategory {
            id: id.into(),
            name: id.into(),
            display_order: 1,
        }
    }

    #[test]
    fn removes_unavailable_items() {
        let items = vec![item("a", "c1", true), item("b", "c1", false)];
        let cats = vec![cat("c1")];
        let out = filter_menu(&items, &cats, &[], None, &[]);
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].id, "a");
    }

    #[test]
    fn removes_sold_out_items() {
        let items = vec![item("a", "c1", true), item("b", "c1", true)];
        let cats = vec![cat("c1")];
        let out = filter_menu(&items, &cats, &["b".into()], None, &[]);
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].id, "a");
    }

    #[test]
    fn hides_empty_categories() {
        let items = vec![item("a", "c1", false)];
        let cats = vec![cat("c1"), cat("c2")];
        let out = filter_menu(&items, &cats, &[], None, &[]);
        assert!(out.items.is_empty());
        assert!(out.categories.is_empty());
    }

    #[test]
    fn restricts_to_meal_period_categories() {
        let items = vec![item("a", "c1", true), item("b", "c2", true)];
        let cats = vec![cat("c1"), cat("c2")];
        let rules = vec![MealPeriodRule {
            name: "lunch".into(),
            start_time: "11:00".into(),
            end_time: "17:00".into(),
            applicable_categories: vec!["c1".into()],
        }];
        let out = filter_menu(&items, &cats, &[], Some("lunch"), &rules);
        assert_eq!(out.items.len(), 1);
        assert_eq!(out.items[0].id, "a");
        assert_eq!(out.categories.len(), 1);
        assert_eq!(out.categories[0].id, "c1");
    }
}
