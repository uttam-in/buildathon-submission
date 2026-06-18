//! Meal-period detection from the day-state timestamp and timezone.

use chrono::{DateTime, Timelike};
use chrono_tz::Tz;

use crate::error::{RenderError, Result};
use crate::models::{DayState, MealPeriodRule};

/// Parses an `HH:MM` 24-hour time string into minutes since midnight.
fn parse_hhmm(s: &str) -> Result<u32> {
    let (h, m) = s
        .split_once(':')
        .ok_or_else(|| RenderError::InvalidInput(format!("invalid time '{s}', expected HH:MM")))?;
    let hours: u32 = h
        .parse()
        .map_err(|_| RenderError::InvalidInput(format!("invalid hour in '{s}'")))?;
    let mins: u32 = m
        .parse()
        .map_err(|_| RenderError::InvalidInput(format!("invalid minute in '{s}'")))?;
    if hours > 23 || mins > 59 {
        return Err(RenderError::InvalidInput(format!("time '{s}' out of range")));
    }
    Ok(hours * 60 + mins)
}

/// Returns true if `now_minutes` falls within `[start, end)`, treating windows
/// where `end <= start` as overnight (wrapping past midnight).
fn within_window(now_minutes: u32, start: u32, end: u32) -> bool {
    if start < end {
        now_minutes >= start && now_minutes < end
    } else {
        // Overnight window: split into [start, 24:00) and [00:00, end).
        now_minutes >= start || now_minutes < end
    }
}

/// Resolves the active meal period for a render.
///
/// If [`DayState::active_meal_period`] is set, it is returned verbatim (the
/// caller's explicit override). Otherwise the timestamp is localised to the
/// configured timezone and matched against each [`MealPeriodRule`] in order;
/// the first matching rule's name is returned. Returns `None` when no rule
/// matches (meaning "show everything").
pub fn detect_meal_period(
    day_state: &DayState,
    rules: &[MealPeriodRule],
) -> Result<Option<String>> {
    if let Some(period) = &day_state.active_meal_period {
        return Ok(Some(period.clone()));
    }

    let tz: Tz = day_state.timezone.parse().map_err(|_| {
        RenderError::InvalidInput(format!("unknown timezone '{}'", day_state.timezone))
    })?;

    let parsed: DateTime<chrono::FixedOffset> =
        DateTime::parse_from_rfc3339(&day_state.timestamp).map_err(|e| {
            RenderError::InvalidInput(format!(
                "invalid timestamp '{}': {e}",
                day_state.timestamp
            ))
        })?;

    let local = parsed.with_timezone(&tz);
    let now_minutes = local.hour() * 60 + local.minute();

    for rule in rules {
        let start = parse_hhmm(&rule.start_time)?;
        let end = parse_hhmm(&rule.end_time)?;
        if within_window(now_minutes, start, end) {
            return Ok(Some(rule.name.clone()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Vec<MealPeriodRule> {
        vec![
            MealPeriodRule {
                name: "breakfast".into(),
                start_time: "06:00".into(),
                end_time: "11:00".into(),
                applicable_categories: vec![],
            },
            MealPeriodRule {
                name: "lunch".into(),
                start_time: "11:00".into(),
                end_time: "17:00".into(),
                applicable_categories: vec![],
            },
            MealPeriodRule {
                name: "late_night".into(),
                start_time: "22:00".into(),
                end_time: "02:00".into(),
                applicable_categories: vec![],
            },
        ]
    }

    fn state_at(ts: &str) -> DayState {
        DayState {
            timestamp: ts.into(),
            // Use UTC so the wall clock equals the timestamp's clock.
            timezone: "UTC".into(),
            sold_out_item_ids: vec![],
            active_meal_period: None,
            promotion_item_ids: vec![],
        }
    }

    #[test]
    fn breakfast_at_1059() {
        let got = detect_meal_period(&state_at("2026-06-18T10:59:00Z"), &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("breakfast"));
    }

    #[test]
    fn lunch_triggers_at_1100() {
        let got = detect_meal_period(&state_at("2026-06-18T11:00:00Z"), &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("lunch"));
    }

    #[test]
    fn overnight_window_after_midnight() {
        let got = detect_meal_period(&state_at("2026-06-18T01:30:00Z"), &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("late_night"));
    }

    #[test]
    fn overnight_window_before_midnight() {
        let got = detect_meal_period(&state_at("2026-06-18T23:30:00Z"), &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("late_night"));
    }

    #[test]
    fn no_match_returns_none() {
        // 03:00 falls outside every window.
        let got = detect_meal_period(&state_at("2026-06-18T03:00:00Z"), &rules()).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn explicit_override_skips_detection() {
        let mut s = state_at("2026-06-18T03:00:00Z");
        s.active_meal_period = Some("dinner".into());
        let got = detect_meal_period(&s, &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("dinner"));
    }

    #[test]
    fn timezone_conversion_changes_period() {
        // 16:30 UTC is 11:30 in America/Chicago (CDT, UTC-5) → lunch.
        let s = DayState {
            timestamp: "2026-06-18T16:30:00Z".into(),
            timezone: "America/Chicago".into(),
            sold_out_item_ids: vec![],
            active_meal_period: None,
            promotion_item_ids: vec![],
        };
        let got = detect_meal_period(&s, &rules()).unwrap();
        assert_eq!(got.as_deref(), Some("lunch"));
    }
}
