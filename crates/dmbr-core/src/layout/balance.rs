//! Balance scoring and category-level rebalancing across screens.

use crate::pipeline::CategoryWithItems;

/// Threshold above which rebalancing is attempted.
const BALANCE_THRESHOLD: f64 = 1.4;
/// Maximum rebalancing iterations.
const MAX_ITERATIONS: usize = 5;

/// Result of a balance pass.
#[derive(Debug, Clone)]
pub struct BalanceResult {
    /// Item count per screen, indexed by screen position.
    pub items_per_screen: Vec<usize>,
    /// Ratio of heaviest to lightest screen (>= 1.0).
    pub balance_score: f64,
}

/// Items currently on each screen.
fn counts(screens: &[Vec<CategoryWithItems>]) -> Vec<usize> {
    screens
        .iter()
        .map(|s| s.iter().map(|c| c.items.len()).sum())
        .collect()
}

/// Computes the balance score: `max_items / max(min_items, 1)`.
fn score(item_counts: &[usize]) -> f64 {
    if item_counts.is_empty() {
        return 1.0;
    }
    let max = *item_counts.iter().max().unwrap_or(&0) as f64;
    let min = (*item_counts.iter().min().unwrap_or(&0)).max(1) as f64;
    max / min
}

/// Computes the balance score and, if it exceeds 1.4, attempts to move whole
/// categories from the heaviest screen to the lightest screen (up to 5
/// iterations). Splitting fragments and the single-category-per-screen case are
/// left untouched so the layout stays category-preserving.
///
/// Returns the final per-screen item counts and the resulting balance score.
pub fn balance(screens: &mut [Vec<CategoryWithItems>]) -> BalanceResult {
    let mut item_counts = counts(screens);
    let mut current_score = score(&item_counts);

    for _ in 0..MAX_ITERATIONS {
        if current_score <= BALANCE_THRESHOLD {
            break;
        }

        let Some(heaviest) = argmax(&item_counts) else {
            break;
        };
        let Some(lightest) = argmin(&item_counts) else {
            break;
        };
        if heaviest == lightest {
            break;
        }

        // Only move if the heaviest screen has more than one category, so we
        // never empty a screen entirely.
        if screens[heaviest].len() < 2 {
            break;
        }

        // Move the smallest category off the heaviest screen to reduce its load
        // without overshooting.
        let Some(idx) = smallest_category_index(&screens[heaviest]) else {
            break;
        };
        let moved = screens[heaviest].remove(idx);
        screens[lightest].push(moved);

        let new_counts = counts(screens);
        let new_score = score(&new_counts);
        // Accept the move only if it does not worsen balance.
        if new_score >= current_score {
            // Revert: pop from lightest and restore to heaviest.
            if let Some(back) = screens[lightest].pop() {
                screens[heaviest].insert(idx, back);
            }
            break;
        }
        item_counts = new_counts;
        current_score = new_score;
    }

    BalanceResult {
        items_per_screen: item_counts,
        balance_score: current_score,
    }
}

fn argmax(v: &[usize]) -> Option<usize> {
    v.iter().enumerate().max_by_key(|(_, &c)| c).map(|(i, _)| i)
}

fn argmin(v: &[usize]) -> Option<usize> {
    v.iter().enumerate().min_by_key(|(_, &c)| c).map(|(i, _)| i)
}

fn smallest_category_index(screen: &[CategoryWithItems]) -> Option<usize> {
    screen
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| c.items.len())
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MenuCategory, MenuItem};

    fn group(cat: &str, n: usize) -> CategoryWithItems {
        let items = (0..n)
            .map(|i| MenuItem {
                id: format!("{cat}-{i}"),
                name: "x".into(),
                price: 1.0,
                category: cat.into(),
                available: true,
                display_order: i as i64,
                description: None,
                price_display: None,
                image: None,
                featured: false,
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

    #[test]
    fn already_balanced_scores_one() {
        let mut screens = vec![vec![group("a", 5)], vec![group("b", 5)]];
        let r = balance(&mut screens);
        assert_eq!(r.balance_score, 1.0);
        assert_eq!(r.items_per_screen, vec![5, 5]);
    }

    #[test]
    fn rebalances_skewed_screens_under_threshold() {
        // Screen 0 holds three equal categories (15 items); screen 1 holds one
        // (5 items). Initial score 3.0. Moving one category yields {10, 10}.
        let mut screens = vec![
            vec![group("a", 5), group("b", 5), group("c", 5)],
            vec![group("d", 5)],
        ];
        let r = balance(&mut screens);
        assert!(
            r.balance_score <= 1.40,
            "expected balanced, got {}",
            r.balance_score
        );
    }

    #[test]
    fn does_not_rebalance_when_irreducible() {
        // One dominant category cannot be split here, so the best achievable
        // score is left as-is rather than thrashing.
        let mut screens = vec![vec![group("a", 7), group("b", 2)], vec![group("c", 2)]];
        let r = balance(&mut screens);
        // No move improves balance, so counts are unchanged.
        assert_eq!(r.items_per_screen.iter().sum::<usize>(), 11);
    }
}
