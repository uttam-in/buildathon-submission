//! Converts challenge-format inputs into `dmbr-core`'s normalized schema.
//!
//! All state-driven gating (out-of-stock, category availability by day + time)
//! is resolved here, deterministically, so the engine only ever sees a flat
//! list of currently-available items. The engine's own meal-period machinery is
//! bypassed (an explicit `all` period with no rules) because category
//! `availability` carries a `days` list the time-only rule model can't express.

use dmbr_core::models::{
    Arrangement, DayState, FullMenu, MenuCategory, MenuItem, Orientation, ScreenConfig, ScreenDef,
};

use crate::challenge::{Availability, ChallengeConfig, ChallengeMenu, ChallengeState, PriceRange};

/// Errors raised while adapting challenge inputs.
#[derive(Debug)]
pub enum AdaptError {
    /// A time string was not `HH:MM`.
    BadTime(String),
    /// A screen orientation was neither `landscape` nor `portrait`.
    BadOrientation(String),
}

impl std::fmt::Display for AdaptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdaptError::BadTime(s) => write!(f, "invalid time '{s}', expected HH:MM"),
            AdaptError::BadOrientation(s) => {
                write!(f, "invalid orientation '{s}', expected landscape or portrait")
            }
        }
    }
}

impl std::error::Error for AdaptError {}

/// Parses `HH:MM` into minutes since midnight.
fn parse_hhmm(s: &str) -> Result<u32, AdaptError> {
    let (h, m) = s.split_once(':').ok_or_else(|| AdaptError::BadTime(s.into()))?;
    let h: u32 = h.parse().map_err(|_| AdaptError::BadTime(s.into()))?;
    let m: u32 = m.parse().map_err(|_| AdaptError::BadTime(s.into()))?;
    if h > 23 || m > 59 {
        return Err(AdaptError::BadTime(s.into()));
    }
    Ok(h * 60 + m)
}

/// Whether `now` falls in `[start, end)`, treating `end <= start` as overnight.
fn within_window(now: u32, start: u32, end: u32) -> bool {
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

/// Resolves whether a category is visible for the given day and time.
fn category_visible(
    availability: &Option<Availability>,
    day: &str,
    now_minutes: u32,
) -> Result<bool, AdaptError> {
    let Some(av) = availability else {
        return Ok(true); // no restriction
    };

    // Day restriction.
    if !av.days.is_empty() && !av.days.iter().any(|d| d.eq_ignore_ascii_case(day)) {
        return Ok(false);
    }

    // Time-window restriction.
    match (&av.from, &av.to) {
        (Some(from), Some(to)) => {
            let start = parse_hhmm(from)?;
            let end = parse_hhmm(to)?;
            Ok(within_window(now_minutes, start, end))
        }
        // A one-sided window: only `from` means "from that time onward",
        // only `to` means "until that time".
        (Some(from), None) => Ok(now_minutes >= parse_hhmm(from)?),
        (None, Some(to)) => Ok(now_minutes < parse_hhmm(to)?),
        (None, None) => Ok(true),
    }
}

/// Formats a price range as `$min–$max` (en dash), two decimals each.
fn format_range(r: &PriceRange) -> String {
    format!("${:.2}\u{2013}${:.2}", r.min, r.max)
}

/// A human-friendly meal-period label for the board header, derived purely
/// from the wall-clock minute-of-day (deterministic; cosmetic only).
fn period_label(now_minutes: u32) -> &'static str {
    match now_minutes {
        m if m < 11 * 60 => "Breakfast",   // before 11:00
        m if m < 17 * 60 => "Lunch",       // 11:00–17:00
        m if m < 22 * 60 => "Dinner",      // 17:00–22:00
        _ => "Late Night",                  // 22:00 onward
    }
}

/// The result of adapting all three inputs: the engine inputs plus the
/// resolved restaurant name for headers.
pub struct Adapted {
    /// Engine menu (flat, only currently-available categories' items).
    pub menu: FullMenu,
    /// Engine screen config.
    pub config: ScreenConfig,
    /// Engine day-state (out-of-stock carried through; period forced to `all`).
    pub state: DayState,
    /// Restaurant display name, for the board header.
    pub restaurant: String,
}

/// Builds a wall grid arrangement (cols × rows) for `n` screens.
///
/// Every provided wall is a single orientation laid out in one row, so the
/// grid is 1×n.
fn arrangement_for(n: u32) -> Arrangement {
    Arrangement { columns: n.max(1), rows: 1 }
}

/// Converts a challenge orientation string into the engine enum.
fn orientation_of(s: &str) -> Result<Orientation, AdaptError> {
    match s.to_ascii_lowercase().as_str() {
        "landscape" => Ok(Orientation::Landscape),
        "portrait" => Ok(Orientation::Portrait),
        other => Err(AdaptError::BadOrientation(other.into())),
    }
}

/// Adapts the three challenge inputs into engine inputs, resolving all
/// state-driven gating deterministically.
pub fn adapt(
    menu: &ChallengeMenu,
    config: &ChallengeConfig,
    state: &ChallengeState,
) -> Result<Adapted, AdaptError> {
    let now = parse_hhmm(&state.time)?;

    let mut categories: Vec<MenuCategory> = Vec::new();
    let mut items: Vec<MenuItem> = Vec::new();

    for (cat_idx, cat) in menu.categories.iter().enumerate() {
        if !category_visible(&cat.availability, &state.day, now)? {
            continue; // out-of-window category disappears entirely
        }
        categories.push(MenuCategory {
            id: cat.id.clone(),
            name: cat.name.clone(),
            display_order: cat_idx as i64,
        });
        for (item_idx, item) in cat.items.iter().enumerate() {
            let (price, price_display) = match (&item.price, &item.price_range) {
                (Some(p), _) => (*p, None),
                (None, Some(r)) => (r.min, Some(format_range(r))),
                (None, None) => (0.0, None),
            };
            items.push(MenuItem {
                id: item.id.clone(),
                name: item.name.clone(),
                price,
                category: cat.id.clone(),
                available: true,
                display_order: item_idx as i64,
                description: item.description.clone(),
                price_display,
                image: item.image.clone(),
                featured: false,
            });
        }
    }

    let restaurant = menu
        .restaurant
        .clone()
        .unwrap_or_else(|| "Menu".to_string());

    let full_menu = FullMenu {
        restaurant_id: restaurant.clone(),
        version: "challenge".into(),
        categories,
        items,
        meal_period_rules: Vec::new(),
    };

    let n = config.screens.len() as u32;
    let arrangement = arrangement_for(n);
    let mut screens: Vec<ScreenDef> = Vec::with_capacity(config.screens.len());
    for (i, s) in config.screens.iter().enumerate() {
        screens.push(ScreenDef {
            id: s.id.clone(),
            orientation: orientation_of(&s.orientation)?,
            width_px: s.width,
            height_px: s.height,
            col: i as u32,
            row: 0,
        });
    }

    let screen_config = ScreenConfig {
        screen_count: config.screens.len() as u8,
        arrangement,
        screens,
    };

    // Out-of-stock carried through to the engine filter; meal period forced to
    // an explicit `all` so the engine skips timezone/timestamp detection and
    // applies no extra category restriction (availability already resolved).
    let day_state = DayState {
        timestamp: "1970-01-01T00:00:00Z".into(),
        timezone: "UTC".into(),
        sold_out_item_ids: state.out_of_stock.clone(),
        // A friendly label for the header. `meal_period_rules` is empty, so the
        // engine applies no extra category restriction regardless of this value
        // (availability is already resolved above).
        active_meal_period: Some(period_label(now).to_string()),
        promotion_item_ids: Vec::new(),
    };

    Ok(Adapted {
        menu: full_menu,
        config: screen_config,
        state: day_state,
        restaurant,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::challenge::{ChallengeCategory, ChallengeItem, ChallengeScreen};

    fn item(id: &str) -> ChallengeItem {
        ChallengeItem {
            id: id.into(),
            name: id.into(),
            price: Some(1.0),
            price_range: None,
            image: None,
            description: None,
        }
    }

    fn menu_with(cats: Vec<ChallengeCategory>) -> ChallengeMenu {
        ChallengeMenu {
            restaurant: Some("Test".into()),
            currency: Some("USD".into()),
            categories: cats,
        }
    }

    fn config_solo() -> ChallengeConfig {
        ChallengeConfig {
            name: Some("solo".into()),
            screens: vec![ChallengeScreen {
                id: "screen-1".into(),
                width: 1920,
                height: 1080,
                orientation: "landscape".into(),
            }],
        }
    }

    fn state(day: &str, time: &str, oos: &[&str]) -> ChallengeState {
        ChallengeState {
            name: None,
            day: day.into(),
            time: time.into(),
            out_of_stock: oos.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn maps_basic_menu() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "c1".into(),
            name: "Cat One".into(),
            items: vec![item("a"), item("b")],
            availability: None,
        }]);
        let out = adapt(&menu, &config_solo(), &state("wed", "12:00", &[])).unwrap();
        assert_eq!(out.menu.categories.len(), 1);
        assert_eq!(out.menu.items.len(), 2);
        assert_eq!(out.config.screen_count, 1);
        assert_eq!(out.restaurant, "Test");
    }

    #[test]
    fn price_range_becomes_display_string() {
        let mut it = item("platter");
        it.price = None;
        it.price_range = Some(PriceRange { min: 19.99, max: 79.99 });
        let menu = menu_with(vec![ChallengeCategory {
            id: "c1".into(),
            name: "Platters".into(),
            items: vec![it],
            availability: None,
        }]);
        let out = adapt(&menu, &config_solo(), &state("wed", "12:00", &[])).unwrap();
        let mapped = &out.menu.items[0];
        assert_eq!(mapped.price, 19.99);
        assert_eq!(mapped.price_display.as_deref(), Some("$19.99\u{2013}$79.99"));
    }

    #[test]
    fn hides_category_outside_time_window() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "breakfast".into(),
            name: "Breakfast".into(),
            items: vec![item("eggs")],
            availability: Some(Availability {
                from: Some("06:00".into()),
                to: Some("11:00".into()),
                days: vec![],
            }),
        }]);
        // 12:45 is past the 11:00 cutoff → hidden.
        let out = adapt(&menu, &config_solo(), &state("wed", "12:45", &[])).unwrap();
        assert!(out.menu.categories.is_empty());
        assert!(out.menu.items.is_empty());
    }

    #[test]
    fn shows_category_inside_time_window() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "breakfast".into(),
            name: "Breakfast".into(),
            items: vec![item("eggs")],
            availability: Some(Availability {
                from: Some("06:00".into()),
                to: Some("11:00".into()),
                days: vec![],
            }),
        }]);
        let out = adapt(&menu, &config_solo(), &state("tue", "08:30", &[])).unwrap();
        assert_eq!(out.menu.items.len(), 1);
    }

    #[test]
    fn hides_category_on_wrong_day() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "weekend".into(),
            name: "Weekend Specials".into(),
            items: vec![item("special")],
            availability: Some(Availability {
                from: None,
                to: None,
                days: vec!["sat".into(), "sun".into()],
            }),
        }]);
        let weekday = adapt(&menu, &config_solo(), &state("wed", "19:00", &[])).unwrap();
        assert!(weekday.menu.items.is_empty());
        let weekend = adapt(&menu, &config_solo(), &state("sat", "19:00", &[])).unwrap();
        assert_eq!(weekend.menu.items.len(), 1);
    }

    #[test]
    fn out_of_stock_carried_to_state() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "c1".into(),
            name: "Cat".into(),
            items: vec![item("a"), item("b")],
            availability: None,
        }]);
        let out = adapt(&menu, &config_solo(), &state("wed", "12:00", &["a"])).unwrap();
        assert_eq!(out.state.sold_out_item_ids, vec!["a".to_string()]);
    }

    #[test]
    fn overnight_window_wraps() {
        let menu = menu_with(vec![ChallengeCategory {
            id: "late".into(),
            name: "Late Night".into(),
            items: vec![item("x")],
            availability: Some(Availability {
                from: Some("22:00".into()),
                to: Some("02:00".into()),
                days: vec![],
            }),
        }]);
        let at_1am = adapt(&menu, &config_solo(), &state("wed", "01:00", &[])).unwrap();
        assert_eq!(at_1am.menu.items.len(), 1);
        let at_noon = adapt(&menu, &config_solo(), &state("wed", "12:00", &[])).unwrap();
        assert!(at_noon.menu.items.is_empty());
    }
}
