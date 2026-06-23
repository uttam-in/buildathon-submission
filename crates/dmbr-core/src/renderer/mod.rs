//! Rendering orchestration: ties the pipeline and layout engine together and
//! produces the final [`LayoutOutput`].

pub mod html;

use std::time::Instant;

use crate::error::Result;
use crate::hash::render_hash;
use crate::layout::capacity::{compute_capacity, GUTTER_PX, MARGIN_PX};
use crate::layout::{balance, negotiate_font, paginate, partition};
use crate::models::{
    DayState, FullMenu, LayoutOutput, RenderedScreen, ScreenConfig, ScreenDef, Warning,
};
use crate::pipeline::{build_ordered_groups, detect_meal_period, filter_menu, CategoryWithItems};

pub use html::{escape_html, render_screen, ScreenMeta};

/// Per-column content width in pixels for a screen with `column_count` columns.
fn column_width(screen: &ScreenDef, column_count: u32) -> u32 {
    let columns = column_count.max(1);
    let inner = screen.width_px.saturating_sub(2 * MARGIN_PX);
    let gutters = GUTTER_PX * (columns - 1);
    inner.saturating_sub(gutters) / columns
}

/// Longest item-name length (in characters) across all groups on a screen.
fn longest_name_chars(slots: &[CategoryWithItems]) -> usize {
    slots
        .iter()
        .flat_map(|g| g.items.iter())
        .map(|i| i.name.chars().count())
        .max()
        .unwrap_or(0)
}

/// Runs the full deterministic rendering pipeline.
///
/// Validates inputs, resolves the active meal period, filters and orders the
/// menu, partitions content across the configured screens, balances the load,
/// negotiates fonts, renders standalone HTML per screen, and computes the
/// SHA-256 render hash. Screens are emitted sorted by screen id.
pub fn render(
    menu: &FullMenu,
    config: &ScreenConfig,
    day_state: &DayState,
) -> Result<LayoutOutput> {
    let start = Instant::now();
    let mut warnings: Vec<Warning> = Vec::new();

    menu.validate()?;
    config.validate()?;

    let active_meal_period = detect_meal_period(day_state, &menu.meal_period_rules)?;

    let filtered = filter_menu(
        &menu.items,
        &menu.categories,
        &day_state.sold_out_item_ids,
        active_meal_period.as_deref(),
        &menu.meal_period_rules,
    );
    let groups = build_ordered_groups(filtered);

    // Screens sorted by id give canonical output and hash ordering.
    let mut ordered_screens: Vec<ScreenDef> = config.screens.clone();
    ordered_screens.sort_by(|a, b| a.id.cmp(&b.id));

    // Use the smallest per-screen slot budget so the partitioner does not
    // overfill any single screen. A zero budget signals an unrenderable screen.
    let min_slots = ordered_screens
        .iter()
        .map(|s| compute_capacity(s).total_slots)
        .min()
        .unwrap_or(0);

    let mut screen_slots = partition(&groups, ordered_screens.len(), min_slots);
    let balance_result = balance(&mut screen_slots);
    if balance_result.balance_score > 1.4 {
        warnings.push(Warning::warn(
            "imbalanced_layout",
            format!(
                "balance score {:.2} exceeds 1.40 after rebalancing",
                balance_result.balance_score
            ),
        ));
    }

    let mut fallback_used = false;
    let mut rendered_screens: Vec<RenderedScreen> = Vec::with_capacity(ordered_screens.len());
    let mut html_contents: Vec<String> = Vec::with_capacity(ordered_screens.len());

    for (screen, slots) in ordered_screens.iter().zip(screen_slots.iter()) {
        let capacity = compute_capacity(screen);
        let container_w = column_width(screen, capacity.column_count);

        // Split this screen's content into capacity-sized pages. When the menu
        // is denser than one screen can hold (e.g. the whole menu on a single
        // landscape screen), the pages cycle deterministically via CSS so every
        // item is shown legibly without clipping.
        //
        // A featured photo rail (shown when this screen has any photo-bearing
        // item) consumes ~24-26% of the content area, so the menu columns have
        // less room. Reduce the per-page capacity by a matching factor first so
        // the listing still fits beside the rail — keeping the no-clip
        // guarantee. The renderer applies the same "has a photo?" rule, so the
        // two always agree.
        let has_photo = slots
            .iter()
            .flat_map(|g| g.items.iter())
            .any(|i| i.image.is_some());
        let full_cap = capacity.total_slots.max(1) as usize;
        let page_cap = if has_photo {
            // Conservative: rail + its gutter/title leave ~70% for the columns.
            (full_cap * 70 / 100).max(1)
        } else {
            full_cap
        };
        let pages = paginate(slots, page_cap);
        if pages.len() > 1 {
            // Not a failure: the screen shows everything by cycling. Surfaced
            // via `fallback_used` so callers can note cycling is in effect.
            fallback_used = true;
            warnings.push(Warning::warn(
                "paged_cycling",
                format!(
                    "screen {} cycles through {} pages ({} items)",
                    screen.id,
                    pages.len(),
                    slots.iter().map(CategoryWithItems::item_count).sum::<usize>()
                ),
            ));
        }

        let font = negotiate_font(screen.height_px, container_w, longest_name_chars(slots));

        let meta = ScreenMeta {
            title: menu.restaurant_id.clone(),
            subtitle: active_meal_period.clone().unwrap_or_default(),
        };
        let html = render_screen(
            screen,
            &meta,
            &pages,
            font.size_px,
            capacity.column_count,
            container_w,
        );

        let item_ids: Vec<String> = slots
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|i| i.id.clone())
            .collect();

        html_contents.push(html.clone());
        rendered_screens.push(RenderedScreen {
            screen_id: screen.id.clone(),
            html_content: html,
            item_count: item_ids.len(),
            item_ids,
            font_size_px: font.size_px,
        });
    }

    let hash = render_hash(&html_contents);

    Ok(LayoutOutput {
        restaurant_id: menu.restaurant_id.clone(),
        menu_version: menu.version.clone(),
        active_meal_period,
        render_hash: hash,
        screens: rendered_screens,
        render_duration_ms: start.elapsed().as_millis() as u64,
        cache_hit: false,
        fallback_used,
        warnings,
    })
}
