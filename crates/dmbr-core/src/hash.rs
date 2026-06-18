//! Deterministic SHA-256 hashing of rendered output and canonical inputs.

use sha2::{Digest, Sha256};

use crate::error::{RenderError, Result};
use crate::models::{DayState, FullMenu, ScreenConfig};

/// Computes the SHA-256 hex digest of the concatenated HTML documents.
///
/// The caller must pass `html_contents` in canonical (screen-id lexicographic)
/// order so the hash is reproducible for identical inputs.
pub fn render_hash(html_contents: &[String]) -> String {
    let mut hasher = Sha256::new();
    for html in html_contents {
        hasher.update(html.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Recursively sorts the keys of a JSON value so serialization is canonical.
fn canonicalize(value: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in entries {
                sorted.insert(k, canonicalize(v));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

/// Computes a SHA-256 hex digest of the canonical JSON of all three inputs.
///
/// Keys are sorted recursively and whitespace removed so logically-equal inputs
/// hash identically. Useful as a cache key.
pub fn input_hash(menu: &FullMenu, config: &ScreenConfig, day_state: &DayState) -> Result<String> {
    let combined = serde_json::json!({
        "menu": menu,
        "config": config,
        "state": day_state,
    });
    let canonical = canonicalize(combined);
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|e| RenderError::RenderError(format!("failed to serialize inputs: {e}")))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_hash_is_stable() {
        let a = render_hash(&["<html>a</html>".into(), "<html>b</html>".into()]);
        let b = render_hash(&["<html>a</html>".into(), "<html>b</html>".into()]);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn render_hash_order_sensitive() {
        let a = render_hash(&["a".into(), "b".into()]);
        let b = render_hash(&["b".into(), "a".into()]);
        assert_ne!(a, b);
    }

    #[test]
    fn known_vector() {
        // SHA-256 of empty input.
        assert_eq!(
            render_hash(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
