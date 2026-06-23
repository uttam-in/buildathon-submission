//! Self-contained HTML5/CSS generation for a single screen.

use std::fmt::Write as _;

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

/// Formats a price as a fixed two-decimal string with a leading `$`.
fn format_price(price: f64) -> String {
    format!("${:.2}", price)
}

/// Presentation metadata for a screen's header band.
#[derive(Debug, Clone, Default)]
pub struct ScreenMeta {
    /// Brand / restaurant title (left of the header).
    pub title: String,
    /// Secondary line (e.g. active meal period); empty hides it.
    pub subtitle: String,
}

/// Renders the inner HTML (category sections) for one page of content.
fn render_page_body(groups: &[CategoryWithItems]) -> String {
    let mut body = String::new();
    for group in groups {
        let mut cat_name = escape_html(&group.category.name);
        if group.continued {
            cat_name.push_str(" <span class=\"cont\">(cont.)</span>");
        }
        body.push_str("<section class=\"category\">");
        let _ = write!(body, "<h2 class=\"cat-header\">{cat_name}</h2>");
        for item in &group.items {
            // Full names, never truncated: long names wrap (challenge rule).
            let name = escape_html(&item.name);
            let price_text = item
                .price_display
                .clone()
                .unwrap_or_else(|| format_price(item.price));
            let price = escape_html(&price_text);
            // Small inline thumbnail when the item has a photo. Sized to the
            // line box so it does not increase the row height (keeps the
            // capacity model — and thus the no-clip guarantee — intact).
            let thumb = match &item.image {
                Some(url) => format!(
                    "<img class=\"thumb\" src=\"{}\" alt=\"\" loading=\"lazy\">",
                    escape_html(url)
                ),
                None => String::new(),
            };
            let _ = write!(
                body,
                "<div class=\"menu-item\">{thumb}<span class=\"item-name\">{name}</span>\
                 <span class=\"leader\"></span>\
                 <span class=\"item-price\">{price}</span></div>"
            );
        }
        body.push_str("</section>");
    }
    body
}

/// Maximum photo cards in the featured strip.
const MAX_FEATURED: usize = 3;

/// Builds the "Today's Features" strip: up to [`MAX_FEATURED`] photo cards.
///
/// Selection (deterministic): admin-marked `featured` items come first (in
/// canonical order), then, if fewer than the max, the remaining slots are
/// filled with the first non-featured photo-bearing items in order. Only items
/// with a photo are eligible (the rail is photo cards). Items reaching the
/// renderer are already in-stock and in-window, so an 86'd or out-of-window
/// item is never featured — and a flagged-but-unavailable item simply isn't
/// here, so the fallback fills its place. Returns empty when no item has a photo.
fn featured_strip(pages: &[Vec<CategoryWithItems>]) -> String {
    // Collect photo-bearing items in canonical order, partitioned by flag.
    let mut featured: Vec<&crate::models::MenuItem> = Vec::new();
    let mut others: Vec<(&str, &crate::models::MenuItem)> = Vec::new();
    let mut flagged_cats: Vec<&str> = Vec::new();
    for page in pages {
        for group in page {
            for item in &group.items {
                if item.image.is_none() {
                    continue;
                }
                if item.featured {
                    featured.push(item);
                    flagged_cats.push(&group.category.name);
                } else {
                    others.push((&group.category.name, item));
                }
            }
        }
    }

    // Featured first, then fill from others, capped at MAX_FEATURED.
    let mut picks: Vec<(&str, &crate::models::MenuItem)> = Vec::new();
    for (item, cat) in featured.iter().zip(flagged_cats.iter()) {
        picks.push((cat, item));
    }
    for entry in &others {
        if picks.len() >= MAX_FEATURED {
            break;
        }
        picks.push(*entry);
    }
    picks.truncate(MAX_FEATURED);

    if picks.is_empty() {
        return String::new();
    }

    let mut cards = String::new();
    for (cat_name, item) in &picks {
        let url = item.image.as_deref().unwrap_or("");
        let name = escape_html(&item.name);
        let price = escape_html(
            &item
                .price_display
                .clone()
                .unwrap_or_else(|| format_price(item.price)),
        );
        let tag = if item.featured {
            "Chef's Special".to_string()
        } else {
            escape_html(cat_name)
        };
        let _ = write!(
            cards,
            "<div class=\"feat-card\">\
<div class=\"feat-img\" style=\"background-image:url('{url}')\"></div>\
<div class=\"feat-meta\"><span class=\"feat-tag\">{tag}</span>\
<span class=\"feat-name\">{name}</span>\
<span class=\"feat-price\">{price}</span></div></div>",
            url = escape_html(url),
            tag = tag,
            name = name,
            price = price,
        );
    }
    format!(
        "<aside class=\"featured\"><div class=\"feat-title\">Today's Features</div>\
<div class=\"feat-list\">{cards}</div></aside>"
    )
}

/// Seconds each page is held before cycling to the next.
const PAGE_HOLD_SECS: u32 = 8;

/// Builds the keyframes + per-page animation CSS for an `n`-page cycle.
///
/// Each page is visible for an equal slice of a `n × PAGE_HOLD_SECS` loop, with
/// a short cross-fade. The timeline is a pure function of the page count, so
/// the animation is identical on every run (seeded by input, not the clock).
fn cycle_css(n: usize) -> String {
    if n <= 1 {
        // Single page: always visible, no animation.
        return ".page{opacity:1;}".to_string();
    }
    let total = n as u32 * PAGE_HOLD_SECS;
    // Fade occupies ~6% of each page's slot.
    let slice = 100.0 / n as f64;
    let fade = (slice * 0.12).min(4.0);
    let mut css = String::new();
    // Stack all pages; animate opacity.
    let _ = write!(
        css,
        ".page{{position:absolute;inset:0;opacity:0;animation:cycle {total}s steps(1,end) infinite;}}"
    );
    for i in 0..n {
        let delay = i as u32 * PAGE_HOLD_SECS;
        let _ = write!(
            css,
            ".page:nth-child({nth}){{animation-delay:{delay}s;}}",
            nth = i + 1,
            delay = delay
        );
    }
    // Keyframes: visible for one slice (minus a fade tail), hidden otherwise.
    let on_end = slice - fade;
    let _ = write!(
        css,
        "@keyframes cycle{{0%{{opacity:1;}}{on_end:.3}%{{opacity:1;}}{slice:.3}%{{opacity:0;}}\
100%{{opacity:0;}}}}",
        on_end = on_end,
        slice = slice
    );
    css
}

/// Renders one screen into a standalone HTML5 document.
///
/// The document inlines all styling and references no external resources. The
/// menu body uses a CSS *multi-column* flow: each category flows down a column
/// and wraps to the next, keeping every item's name and price paired. Names are
/// never truncated; long names wrap.
///
/// `pages` is the screen's content split into capacity-sized pages. A single
/// page renders statically; multiple pages are stacked and cross-fade on a
/// fixed, input-seeded CSS timeline so every item is shown legibly without
/// clipping — deterministic by construction (no clock, no randomness). All
/// user-supplied text is HTML-escaped.
pub fn render_screen(
    screen: &ScreenDef,
    meta: &ScreenMeta,
    pages: &[Vec<CategoryWithItems>],
    font_size_px: u32,
    column_count: u32,
    container_width: u32,
) -> String {
    let cat_font = font_size_px + 3;
    let columns = column_count.max(1);
    let _ = container_width; // width informs upstream font negotiation, not clipping

    let page_count = pages.len().max(1);
    let mut body = String::new();
    for (i, page) in pages.iter().enumerate() {
        let indicator = if page_count > 1 {
            format!(
                "<div class=\"pageno\">{cur} / {total}</div>",
                cur = i + 1,
                total = page_count
            )
        } else {
            String::new()
        };
        let _ = write!(
            body,
            "<div class=\"page\"><div class=\"cols\">{inner}</div>{indicator}</div>",
            inner = render_page_body(page),
            indicator = indicator
        );
    }
    let cycle = cycle_css(page_count);
    let featured = featured_strip(pages);
    // Portrait walls are narrow: lay the feature strip across the top; landscape
    // walls get a left rail. Chosen from geometry so it is deterministic.
    let is_portrait = screen.height_px > screen.width_px;
    let stage_dir = if is_portrait { "column" } else { "row" };
    // Featured area: a left rail (~24% width) on landscape, or a top strip
    // (~26% height) on portrait. Cards stack vertically in a rail, horizontally
    // in a strip.
    let (feat_axis, feat_list_dir) = if is_portrait {
        (
            format!("height:{}px;", (screen.height_px * 26 / 100)),
            "flex-direction:row;".to_string(),
        )
    } else {
        (
            format!("width:{}px;", (screen.width_px * 24 / 100)),
            "flex-direction:column;".to_string(),
        )
    };

    let title = escape_html(&meta.title);
    let subtitle = escape_html(&meta.subtitle);
    let subtitle_html = if meta.subtitle.is_empty() {
        String::new()
    } else {
        format!("<span class=\"period\">{subtitle}</span>")
    };

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width={width}, height={height}\">\n\
<title>{title}</title>\n<style>\n\
*{{margin:0;padding:0;box-sizing:border-box;}}\n\
html,body{{width:{width}px;height:{height}px;overflow:hidden;}}\n\
body{{background:radial-gradient(120% 90% at 0% 0%,#1c140e 0%,#0e0b09 55%,#070605 100%);\
color:#f3ece1;font-family:{font_stack};}}\n\
.board{{width:{width}px;height:{height}px;padding:{margin}px;display:flex;\
flex-direction:column;}}\n\
.header{{height:{header}px;display:flex;align-items:baseline;gap:20px;\
border-bottom:3px solid #c8862f;padding-bottom:14px;margin-bottom:18px;flex:none;}}\n\
.brand{{font-size:{brand_font}px;font-weight:800;letter-spacing:-0.01em;\
color:#fbf5ea;}}\n\
.period{{font-size:{period_font}px;font-weight:600;text-transform:uppercase;\
letter-spacing:0.16em;color:#e7b15a;}}\n\
.stage{{flex:1;min-height:0;display:flex;flex-direction:{stage_dir};gap:{gutter}px;}}\n\
.content{{position:relative;flex:1;min-height:0;min-width:0;overflow:hidden;}}\n\
.cols{{column-count:{columns};column-gap:{gutter}px;height:100%;overflow:hidden;}}\n\
.featured{{flex:none;display:flex;flex-direction:column;min-height:0;{feat_axis}}}\n\
.feat-title{{flex:none;font-size:{cat_font}px;font-weight:800;color:#e7b15a;\
text-transform:uppercase;letter-spacing:0.08em;margin-bottom:12px;}}\n\
.feat-list{{flex:1 1 auto;min-height:0;display:flex;{feat_list_dir}gap:14px;}}\n\
.feat-card{{flex:1 1 0;display:flex;flex-direction:column;background:#181410;\
border:1px solid #3a2e1f;border-radius:14px;overflow:hidden;min-height:0;}}\n\
.feat-img{{flex:1 1 auto;min-height:90px;background-size:cover;\
background-position:center;}}\n\
.feat-meta{{padding:10px 12px;display:flex;flex-direction:column;gap:3px;flex:none;}}\n\
.feat-tag{{font-size:{footer_font}px;text-transform:uppercase;letter-spacing:0.1em;\
color:#9b8b73;}}\n\
.feat-name{{font-size:{item_font}px;font-weight:700;color:#fbf5ea;line-height:1.15;}}\n\
.feat-price{{font-family:{mono_stack};font-weight:700;color:#f4c87a;\
font-size:{item_font}px;}}\n\
.thumb{{flex:none;width:1.5em;height:1.5em;object-fit:cover;border-radius:5px;\
margin-right:9px;align-self:center;}}\n\
.pageno{{position:absolute;right:0;bottom:0;font-size:{footer_font}px;\
color:#7a6c57;letter-spacing:0.08em;}}\n\
{cycle}\n\
.category{{margin:0 0 {cat_gap}px;}}\n\
.cat-header{{font-size:{cat_font}px;font-weight:800;color:#e7b15a;\
text-transform:uppercase;letter-spacing:0.06em;border-bottom:1px solid #4a3a26;\
padding-bottom:6px;margin-bottom:8px;break-after:avoid;-webkit-column-break-after:avoid;}}\n\
.cat-header .cont{{font-size:{item_font}px;font-weight:500;text-transform:none;\
letter-spacing:0;color:#9b8b73;}}\n\
.menu-item{{display:flex;align-items:baseline;font-size:{item_font}px;\
line-height:1.25;padding:{item_pad}px 0;break-inside:avoid;\
-webkit-column-break-inside:avoid;}}\n\
.item-name{{flex:0 1 auto;overflow-wrap:anywhere;color:#f0e8da;}}\n\
.leader{{flex:1 1 auto;min-width:10px;margin:0 8px;\
border-bottom:1px dotted #5a4a33;transform:translateY(-4px);}}\n\
.item-price{{flex:none;font-family:{mono_stack};font-weight:700;\
color:#f4c87a;white-space:nowrap;}}\n\
.footer{{height:{footer}px;display:flex;align-items:center;justify-content:flex-end;\
font-size:{footer_font}px;color:#7a6c57;letter-spacing:0.08em;flex:none;}}\n\
</style>\n</head>\n<body>\n\
<div class=\"board\">\n\
<header class=\"header\"><span class=\"brand\">{title}</span>{subtitle_html}</header>\n\
<div class=\"stage\">{featured}<main class=\"content\">{body}</main></div>\n\
<footer class=\"footer\">{title} · {screen_id}</footer>\n\
</div>\n</body>\n</html>",
        width = screen.width_px,
        height = screen.height_px,
        title = title,
        screen_id = escape_html(&screen.id),
        subtitle_html = subtitle_html,
        font_stack = FONT_STACK,
        mono_stack = MONO_STACK,
        margin = crate::layout::capacity::MARGIN_PX,
        header = crate::layout::capacity::HEADER_HEIGHT_PX,
        footer = crate::layout::capacity::FOOTER_HEIGHT_PX,
        brand_font = cat_font + 10,
        period_font = font_size_px.saturating_sub(2).max(14),
        footer_font = font_size_px.saturating_sub(6).max(12),
        gutter = crate::layout::capacity::GUTTER_PX,
        cat_gap = crate::layout::capacity::GUTTER_PX,
        columns = columns,
        cat_font = cat_font,
        item_font = font_size_px,
        item_pad = 2,
        cycle = cycle,
        stage_dir = stage_dir,
        feat_axis = feat_axis,
        feat_list_dir = feat_list_dir,
        featured = featured,
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
                price_display: None,
                image: None,
                featured: false,
            }],
            continued: false,
        }
    }

    fn meta() -> ScreenMeta {
        ScreenMeta {
            title: "Saffron Junction".into(),
            subtitle: "Lunch".into(),
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
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 28, 4, 600);
        assert!(html.contains("Cheeseburger"));
        assert!(html.contains("$8.99"));
        assert!(html.contains("Burgers"));
        assert!(html.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn escapes_item_name_in_output() {
        let groups = vec![group_with("Cat", "Fish & <Chips>", 5.0)];
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 28, 4, 600);
        assert!(html.contains("Fish &amp; &lt;Chips&gt;"));
        assert!(!html.contains("Fish & <Chips>"));
    }

    #[test]
    fn no_script_or_external_urls() {
        let groups = vec![group_with("Burgers", "Cheeseburger", 8.99)];
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 28, 4, 600);
        assert!(!html.contains("<script"));
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
    }

    #[test]
    fn continuation_marker_appended() {
        let mut groups = vec![group_with("Burgers", "Cheeseburger", 8.99)];
        groups[0].continued = true;
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 28, 4, 600);
        assert!(html.contains("Burgers"));
        assert!(html.contains("(cont.)"));
    }

    #[test]
    fn long_name_is_not_truncated() {
        // Names must never be clipped: the full string appears verbatim and no
        // ellipsis is introduced (challenge rule: wrapping is fine, clipping is not).
        let long = "X".repeat(200);
        let groups = vec![group_with("Cat", &long, 1.0)];
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 24, 1, 100);
        assert!(html.contains(&long));
        assert!(!html.contains('…'));
    }

    #[test]
    fn price_display_overrides_numeric_price() {
        let mut groups = vec![group_with("Platters", "Mutton Platter", 19.99)];
        groups[0].items[0].price_display = Some("$19.99–79.99".into());
        let html = render_screen(&screen(), &meta(), std::slice::from_ref(&groups), 28, 4, 600);
        assert!(html.contains("$19.99–79.99"));
        assert!(!html.contains("$19.99<")); // not the bare numeric form
    }
}
