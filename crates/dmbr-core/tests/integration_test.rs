//! End-to-end integration test for the full render pipeline.

use std::collections::HashSet;

use dmbr_core::models::{
    Arrangement, DayState, FullMenu, MealPeriodRule, MenuCategory, MenuItem, Orientation,
    ScreenConfig, ScreenDef,
};

fn ten_item_menu() -> FullMenu {
    let categories = vec![
        MenuCategory {
            id: "cat-burgers".into(),
            name: "Burgers".into(),
            display_order: 1,
        },
        MenuCategory {
            id: "cat-sides".into(),
            name: "Sides".into(),
            display_order: 2,
        },
    ];

    let mut items = Vec::new();
    for i in 0..5 {
        items.push(MenuItem {
            id: format!("burger-{i:02}"),
            name: format!("Burger {i}"),
            price: 8.0 + i as f64,
            category: "cat-burgers".into(),
            available: true,
            display_order: i as i64,
            description: None,
            price_display: None,
            image: None,
        });
    }
    for i in 0..5 {
        items.push(MenuItem {
            id: format!("side-{i:02}"),
            name: format!("Side {i}"),
            price: 2.0 + i as f64,
            category: "cat-sides".into(),
            available: true,
            display_order: i as i64,
            description: None,
            price_display: None,
            image: None,
        });
    }

    FullMenu {
        restaurant_id: "store-042".into(),
        version: "1.0.0".into(),
        categories,
        items,
        meal_period_rules: vec![MealPeriodRule {
            name: "lunch".into(),
            start_time: "11:00".into(),
            end_time: "17:00".into(),
            applicable_categories: vec![],
        }],
    }
}

fn two_screen_config() -> ScreenConfig {
    ScreenConfig {
        screen_count: 2,
        arrangement: Arrangement {
            columns: 2,
            rows: 1,
        },
        screens: vec![
            ScreenDef {
                id: "s0".into(),
                orientation: Orientation::Landscape,
                width_px: 1920,
                height_px: 1080,
                col: 0,
                row: 0,
            },
            ScreenDef {
                id: "s1".into(),
                orientation: Orientation::Landscape,
                width_px: 1920,
                height_px: 1080,
                col: 1,
                row: 0,
            },
        ],
    }
}

fn lunch_state() -> DayState {
    DayState {
        timestamp: "2026-06-18T12:00:00Z".into(),
        timezone: "UTC".into(),
        sold_out_item_ids: vec![],
        active_meal_period: None,
        promotion_item_ids: vec![],
    }
}

#[test]
fn full_pipeline_distributes_all_items_without_duplicates() {
    let menu = ten_item_menu();
    let config = two_screen_config();
    let state = lunch_state();

    let output = dmbr_core::render(&menu, &config, &state).expect("render should succeed");

    assert_eq!(output.restaurant_id, "store-042");
    assert_eq!(output.menu_version, "1.0.0");
    assert_eq!(output.active_meal_period.as_deref(), Some("lunch"));
    assert_eq!(output.screens.len(), 2);

    // All 10 items placed exactly once across both screens.
    let mut all_ids: Vec<String> = output
        .screens
        .iter()
        .flat_map(|s| s.item_ids.iter().cloned())
        .collect();
    assert_eq!(all_ids.len(), 10, "all items placed");
    let unique: HashSet<&String> = all_ids.iter().collect();
    assert_eq!(unique.len(), 10, "no duplicate items across screens");

    all_ids.sort();
    let expected: Vec<String> = menu.items.iter().map(|i| i.id.clone()).collect();
    let mut expected_sorted = expected;
    expected_sorted.sort();
    assert_eq!(all_ids, expected_sorted);

    // Even split of two equal categories across two screens.
    assert_eq!(output.screens[0].item_count, 5);
    assert_eq!(output.screens[1].item_count, 5);

    // Valid 64-char hex SHA-256 render hash.
    assert_eq!(output.render_hash.len(), 64);
    assert!(output.render_hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Screens are emitted in canonical (id-sorted) order.
    assert_eq!(output.screens[0].screen_id, "s0");
    assert_eq!(output.screens[1].screen_id, "s1");

    // Each HTML doc is self-contained with no external resources or scripts.
    for screen in &output.screens {
        assert!(screen.html_content.starts_with("<!DOCTYPE html>"));
        assert!(!screen.html_content.contains("<script"));
        assert!(!screen.html_content.contains("http://"));
        assert!(!screen.html_content.contains("https://"));
    }
}

#[test]
fn render_is_deterministic() {
    let menu = ten_item_menu();
    let config = two_screen_config();
    let state = lunch_state();

    let a = dmbr_core::render(&menu, &config, &state).unwrap();
    let b = dmbr_core::render(&menu, &config, &state).unwrap();

    assert_eq!(a.render_hash, b.render_hash);
    for (sa, sb) in a.screens.iter().zip(b.screens.iter()) {
        assert_eq!(sa.html_content, sb.html_content);
    }
}

#[test]
fn explicit_meal_period_override_is_used() {
    let menu = ten_item_menu();
    let config = two_screen_config();
    let mut state = lunch_state();
    state.timestamp = "2026-06-18T03:00:00Z".into(); // outside lunch window
    state.active_meal_period = Some("dinner".into());

    let output = dmbr_core::render(&menu, &config, &state).unwrap();
    assert_eq!(output.active_meal_period.as_deref(), Some("dinner"));
}
