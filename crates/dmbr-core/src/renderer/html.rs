//! Self-contained HTML5/CSS generation for a single screen.

use std::fmt::Write as _;

use crate::layout::font::max_chars;
use crate::models::ScreenDef;
use crate::pipeline::CategoryWithItems;

const FONT_STACK: &str =
    "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif";
const MONO_STACK: &str = "'Courier New', Courier, monospace";

/// Escapes the five HTML-significant characters in user-supplied text.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Truncates `name` to at most `limit` characters, appending an ellipsis when
/// clipped. Operates on `char` boundaries so multibyte text is never split.
fn truncate_name(name: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    let count = name.chars().count();
    if count <= limit {
        return name.to_string();
    }
    let keep = limit.saturating_sub(1);
    let mut s: String = name.chars().take(keep).collect();
    s.push('…');
    s
}

/// Formats a price as a fixed two-decimal string with a leading `$`.
fn format_price(price: f64) -> String {
    format!("${:.2}", price)
}

/// Renders one screen into a standalone HTML5 document.
///
/// The document inlines all styling, uses CSS Grid for the column layout, and
/// references no external resources. Item names that would overflow the column
/// at `font_size_px` are truncated with an ellipsis. All user-supplied text is
/// HTML-escaped.
pub fn render_screen(
    screen: &ScreenDef,
    slots: &[CategoryWithItems],
    font_size_px: u32,
    column_count: u32,
    container_width: u32,
) -> String {
    let cat_font = font_size_px + 4;
    let columns = column_count.max(1);
    let char_limit = max_chars(container_width, font_size_px);

    let mut body = String::new();
    for group in slots {
        let mut cat_name = escape_html(&group.category.name);
        if group.continued {
            cat_name.push_str(" (cont.)");
        }
        let _ = write!(
            body,
            "<div class=\"cat-header\">{cat_name}</div>",
            cat_name = cat_name
        );
        for item in &group.items {
            let display = truncate_name(&item.name, char_limit);
            let name = escape_html(&display);
            let price = escape_html(&format_price(item.price));
            let _ = write!(
                body,
                "<div class=\"menu-item\"><span class=\"item-name\">{name}</span>\
                 <span class=\"item-price\">{price}</span></div>"
            );
        }
    }

    let title = escape_html(&screen.id);

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width={width}, height={height}\">\n\
<title>{title}</title>\n<style>\n\
*{{margin:0;padding:0;box-sizing:border-box;}}\n\
html,body{{width:{width}px;height:{height}px;background:#111;color:#fff;\
font-family:{font_stack};overflow:hidden;}}\n\
.board{{width:{width}px;height:{height}px;padding:{margin}px;display:flex;\
flex-direction:column;}}\n\
.header{{height:{header}px;display:flex;align-items:center;\
font-size:{header_font}px;font-weight:700;overflow:hidden;}}\n\
.content{{flex:1;display:grid;grid-template-columns:repeat({columns},1fr);\
gap:{gutter}px;overflow:hidden;}}\n\
.cat-header{{height:{cat_h}px;display:flex;align-items:center;\
font-size:{cat_font}px;font-weight:700;border-bottom:2px solid #444;\
margin-bottom:8px;overflow:hidden;white-space:nowrap;}}\n\
.menu-item{{height:{item_h}px;display:flex;justify-content:space-between;\
align-items:center;font-size:{item_font}px;overflow:hidden;}}\n\
.item-name{{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;\
flex:1;margin-right:12px;}}\n\
.item-price{{font-family:{mono_stack};font-weight:700;overflow:hidden;\
white-space:nowrap;}}\n\
.footer{{height:{footer}px;display:flex;align-items:center;\
font-size:{footer_font}px;color:#999;overflow:hidden;}}\n\
</style>\n</head>\n<body>\n\
<div class=\"board\">\n\
<div class=\"header\">{title}</div>\n\
<div class=\"content\">{body}</div>\n\
<div class=\"footer\">{title}</div>\n\
</div>\n</body>\n</html>",
        width = screen.width_px,
        height = screen.height_px,
        title = title,
        font_stack = FONT_STACK,
        mono_stack = MONO_STACK,
        margin = crate::layout::capacity::MARGIN_PX,
        header = crate::layout::capacity::HEADER_HEIGHT_PX,
        footer = crate::layout::capacity::FOOTER_HEIGHT_PX,
        header_font = cat_font + 4,
        footer_font = font_size_px.saturating_sub(6).max(12),
        cat_h = crate::layout::capacity::CATEGORY_HEADER_HEIGHT_PX,
        item_h = crate::layout::capacity::ITEM_SLOT_HEIGHT_PX,
        gutter = crate::layout::capacity::GUTTER_PX,
        columns = columns,
        cat_font = cat_font,
        item_font = font_size_px,
        body = body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MenuCategory, MenuItem, Orientation};

    fn screen() -> ScreenDef {
        ScreenDef {
            id: "s0".into(),
            orientation: Orientation::Landscape,
            width_px: 1920,
            height_px: 1080,
            col: 0,
            row: 0,
        }
    }

    fn group_with(name: &str, item_name: &str, price: f64) -> CategoryWithItems {
        CategoryWithItems {
            category: MenuCategory {
                id: "c".into(),
                name: name.into(),
                display_order: 1,
            },
            items: vec![MenuItem {
                id: "i".into(),
                name: item_name.into(),
                price,
                category: "c".into(),
                available: true,
                display_order: 1,
                description: None,
            }],
            continued: false,
        }
    }

    #[test]
    fn escapes_all_significant_chars() {
        assert_eq!(
            escape_html("a & b < c > d \" '"),
            "a &amp; b &lt; c &gt; d &quot; &#39;"
        );
    }

    #[test]
    fn html_contains_name_and_price() {
        let groups = vec![group_with("Burgers", "Cheeseburger", 8.99)];
        let html = render_screen(&screen(), &groups, 28, 3, 600);
        assert!(html.contains("Cheeseburger"));
        assert!(html.contains("$8.99"));
        assert!(html.contains("Burgers"));
        assert!(html.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn escapes_item_name_in_output() {
        let groups = vec![group_with("Cat", "Fish & <Chips>", 5.0)];
        let html = render_screen(&screen(), &groups, 28, 3, 600);
        assert!(html.contains("Fish &amp; &lt;Chips&gt;"));
        assert!(!html.contains("Fish & <Chips>"));
    }

    #[test]
    fn no_script_or_external_urls() {
        let groups = vec![group_with("Burgers", "Cheeseburger", 8.99)];
        let html = render_screen(&screen(), &groups, 28, 3, 600);
        assert!(!html.contains("<script"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }

    #[test]
    fn continuation_marker_appended() {
        let mut groups = vec![group_with("Burgers", "Cheeseburger", 8.99)];
        groups[0].continued = true;
        let html = render_screen(&screen(), &groups, 28, 3, 600);
        assert!(html.contains("Burgers (cont.)"));
    }

    #[test]
    fn long_name_truncated_with_ellipsis() {
        let long = "X".repeat(200);
        let groups = vec![group_with("Cat", &long, 1.0)];
        let html = render_screen(&screen(), &groups, 24, 1, 100);
        assert!(html.contains('…'));
    }
}
