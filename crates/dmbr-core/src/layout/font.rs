//! Font-size negotiation and name-truncation estimation.

/// Average glyph width as a fraction of font size, used to estimate text width.
/// Expressed as a permille (×1000) integer to keep layout math in `u32`.
const AVG_CHAR_WIDTH_PERMILLE: u32 = 550;

/// The negotiated font specification for a screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontSpec {
    /// Chosen font size in pixels.
    pub size_px: u32,
    /// Whether any item name needed truncation at the floor size.
    pub truncated: bool,
}

/// Preferred font size for a screen height.
fn preferred_size(height_px: u32) -> u32 {
    if height_px >= 2160 {
        32
    } else if height_px >= 1080 {
        26
    } else {
        20
    }
}

/// Hard floor font size for a screen height.
fn floor_size(height_px: u32) -> u32 {
    if height_px >= 1080 {
        24
    } else {
        18
    }
}

/// Estimated rendered width (px) of `char_count` glyphs at `font_size`.
fn estimated_width(char_count: usize, font_size: u32) -> u32 {
    char_count as u32 * AVG_CHAR_WIDTH_PERMILLE * font_size / 1000
}

/// Returns the maximum number of characters that fit in `container_width` at
/// `font_size`, leaving room for a one-character ellipsis.
pub fn max_chars(container_width: u32, font_size: u32) -> usize {
    if font_size == 0 {
        return 0;
    }
    let per_char = (AVG_CHAR_WIDTH_PERMILLE * font_size / 1000).max(1);
    (container_width / per_char) as usize
}

/// Negotiates a font size for a screen given the widest item-name length and
/// the per-column content width.
///
/// The size is the preferred size for the screen height, but if the longest
/// name does not fit even at the floor size, truncation is flagged so the
/// renderer can clip names with an ellipsis.
pub fn negotiate_font(height_px: u32, container_width: u32, longest_name_chars: usize) -> FontSpec {
    let preferred = preferred_size(height_px);
    let floor = floor_size(height_px);

    let mut size = preferred;
    while size > floor && estimated_width(longest_name_chars, size) > container_width {
        size -= 1;
    }

    let truncated = estimated_width(longest_name_chars, size) > container_width;
    FontSpec {
        size_px: size,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_keep_preferred_size() {
        let spec = negotiate_font(1080, 600, 10);
        assert_eq!(spec.size_px, 26);
        assert!(!spec.truncated);
    }

    #[test]
    fn long_names_shrink_to_floor_and_truncate() {
        let spec = negotiate_font(1080, 80, 60);
        assert_eq!(spec.size_px, 24);
        assert!(spec.truncated);
    }

    #[test]
    fn never_below_floor() {
        let spec = negotiate_font(720, 10, 100);
        assert_eq!(spec.size_px, 18);
    }

    #[test]
    fn max_chars_reasonable() {
        // container 600px at 24px: per_char = 550*24/1000 = 13; 600/13 = 46
        assert_eq!(max_chars(600, 24), 46);
    }
}
