//! Server-rendered HTML for the admin UI: login, store list, and the
//! per-store editor (store fields + screen monitors). Pure functions of their
//! inputs; the server wires them to routes and the DB.

use crate::db::{MenuCategoryRow, MenuItemRow, Screen, Store};
use crate::{esc, page, Entry};

/// The login page. `error` shows a message above the form when present.
pub fn login_page(error: Option<&str>) -> String {
    let err = error
        .map(|e| format!("<p class=\"err\">{}</p>", esc(e)))
        .unwrap_or_default();
    let body = format!(
        "<div class=\"auth\"><h1>Admin sign in</h1>{err}\
<form method=\"post\" action=\"/admin/login\">\
<label>Username<input name=\"username\" autofocus required></label>\
<label>Password<input name=\"password\" type=\"password\" required></label>\
<button type=\"submit\">Sign in</button></form>\
<p class=\"sub\"><a href=\"/\">← back to boards</a></p></div>\
<style>.auth{{max-width:360px;margin:6vh auto;}}\
.auth form{{display:flex;flex-direction:column;gap:14px;margin-top:18px;}}\
.auth label{{display:flex;flex-direction:column;gap:6px;font-size:14px;color:#c9bca6;}}\
.auth input{{padding:11px 13px;border-radius:10px;border:1px solid #3a2e1f;\
background:#120e0a;color:#f3ece1;font-size:15px;}}\
.auth button{{padding:12px;border:0;border-radius:10px;background:#c8862f;\
color:#1a1206;font-weight:700;font-size:15px;cursor:pointer;}}\
.err{{background:#3a1414;border:1px solid #7a2a2a;color:#f3c9c9;\
padding:10px 12px;border-radius:10px;margin-top:12px;}}</style>",
        err = err
    );
    page("Admin sign in", &body)
}

/// Shared admin chrome: top bar with nav + a logout button, wrapping `inner`.
fn admin_shell(title: &str, inner: &str) -> String {
    let body = format!(
        "<div class=\"bar\"><span class=\"logo\">Saffron Junction · Admin</span>\
<span class=\"nav\"><a href=\"/admin/stores\">Stores</a><a href=\"/admin/menu\">Menu</a></span>\
<form method=\"post\" action=\"/admin/logout\"><button>Sign out</button></form></div>\
{inner}\
<style>.bar{{display:flex;justify-content:space-between;align-items:center;\
margin-bottom:28px;padding-bottom:14px;border-bottom:1px solid #3a2e1f;}}\
.logo{{font-weight:800;color:#fbf5ea;font-size:18px;}}\
.nav{{display:flex;gap:18px;margin-left:auto;margin-right:18px;}}\
.nav a{{color:#c9bca6;text-decoration:none;font-size:14px;}}\
.nav a:hover{{color:#f4c87a;}}\
.bar button{{background:none;border:1px solid #4a3a26;color:#c9bca6;\
padding:7px 14px;border-radius:9px;cursor:pointer;}}\
table{{width:100%;border-collapse:collapse;margin-top:8px;}}\
th,td{{text-align:left;padding:10px 12px;border-bottom:1px solid #2a2118;font-size:14px;}}\
th{{color:#9a8f7d;text-transform:uppercase;font-size:12px;letter-spacing:0.08em;}}\
.btn{{display:inline-block;padding:9px 15px;border-radius:9px;background:#c8862f;\
color:#1a1206;font-weight:700;border:0;cursor:pointer;text-decoration:none;font-size:14px;}}\
.btn.sec{{background:#221a11;color:#e7b15a;border:1px solid #4a3a26;}}\
.btn.danger{{background:#3a1414;color:#f3c9c9;border:1px solid #7a2a2a;}}\
form.inline{{display:inline;}}\
.field{{display:flex;flex-direction:column;gap:6px;font-size:13px;color:#c9bca6;}}\
input,select{{padding:9px 11px;border-radius:9px;border:1px solid #3a2e1f;\
background:#120e0a;color:#f3ece1;font-size:14px;}}\
.row{{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));\
gap:12px;align-items:end;margin-top:8px;}}\
.actions{{display:flex;gap:8px;align-items:center;}}\
.card2{{background:#181410;border:1px solid #3a2e1f;border-radius:14px;\
padding:22px;margin-top:18px;}}</style>",
        inner = inner
    );
    page(title, &body)
}

/// The store list page.
pub fn stores_page(stores: &[Store]) -> String {
    let mut rows = String::new();
    if stores.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"4\" style=\"color:#9a8f7d\">No stores yet — add one below.</td></tr>",
        );
    }
    for s in stores {
        rows.push_str(&format!(
            "<tr><td><b>{name}</b></td><td><code>{slug}</code></td>\
<td>{tz}</td><td class=\"actions\">\
<a class=\"btn sec\" href=\"/admin/stores/{id}\">Manage</a>\
<a class=\"btn sec\" target=\"_blank\" href=\"/store/{slug}\">View wall ↗</a>\
<form class=\"inline\" method=\"post\" action=\"/admin/stores/{id}/delete\" \
onsubmit=\"return confirm('Delete store {name}?')\">\
<button class=\"btn danger\">Delete</button></form></td></tr>",
            name = esc(&s.name),
            slug = esc(&s.slug),
            tz = esc(&s.timezone),
            id = s.id,
        ));
    }
    let inner = format!(
        "<h1>Stores</h1>\
<table><thead><tr><th>Name</th><th>Slug</th><th>Timezone</th><th>Actions</th></tr></thead>\
<tbody>{rows}</tbody></table>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Add a store</h2>\
<form method=\"post\" action=\"/admin/stores\"><div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" required></label>\
<label class=\"field\">Slug<input name=\"slug\" placeholder=\"store-042\" required></label>\
<label class=\"field\">Timezone<input name=\"timezone\" value=\"America/Chicago\"></label>\
<label class=\"field\">State key<input name=\"state_key\" value=\"weekday-lunch-rush\"></label>\
<button class=\"btn\" type=\"submit\">Add store</button></div></form></div>",
        rows = rows
    );
    admin_shell("Stores · Admin", &inner)
}

/// Renders an `<option>` list of state keys, marking `selected` as chosen.
fn state_options(states: &[Entry], selected: &str) -> String {
    let mut out = String::new();
    for s in states {
        let sel = if s.key == selected { " selected" } else { "" };
        out.push_str(&format!(
            "<option value=\"{k}\"{sel}>{name}</option>",
            k = esc(&s.key),
            sel = sel,
            name = esc(&s.name),
        ));
    }
    out
}

/// The per-store editor: store fields + the screens table and add form.
pub fn store_edit_page(store: &Store, screens: &[Screen], states: &[Entry]) -> String {
    let mut screen_rows = String::new();
    if screens.is_empty() {
        screen_rows.push_str(
            "<tr><td colspan=\"4\" style=\"color:#9a8f7d\">No monitors yet — add one below.</td></tr>",
        );
    }
    for sc in screens {
        let land = if sc.orientation == "landscape" { " selected" } else { "" };
        let port = if sc.orientation == "portrait" { " selected" } else { "" };
        screen_rows.push_str(&format!(
            "<tr><td colspan=\"4\"><form method=\"post\" \
action=\"/admin/screens/{id}/update\"><div class=\"row\">\
<label class=\"field\">Label<input name=\"label\" value=\"{label}\" required></label>\
<label class=\"field\">Orientation<select name=\"orientation\">\
<option value=\"landscape\"{land}>landscape</option>\
<option value=\"portrait\"{port}>portrait</option></select></label>\
<label class=\"field\">Width<input name=\"width_px\" type=\"number\" value=\"{w}\" required></label>\
<label class=\"field\">Height<input name=\"height_px\" type=\"number\" value=\"{h}\" required></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"{pos}\"></label>\
<div class=\"actions\"><button class=\"btn\" type=\"submit\">Save</button></div></div></form>\
<form class=\"inline\" method=\"post\" action=\"/admin/screens/{id}/delete\">\
<button class=\"btn danger\">Delete monitor</button></form></td></tr>",
            id = sc.id,
            label = esc(&sc.label),
            land = land,
            port = port,
            w = sc.width_px,
            h = sc.height_px,
            pos = sc.position,
        ));
    }

    let inner = format!(
        "<div class=\"sub\" style=\"margin-bottom:8px\"><a href=\"/admin/stores\">← all stores</a></div>\
<h1>{name}</h1>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Store details</h2>\
<form method=\"post\" action=\"/admin/stores/{id}/update\"><div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" value=\"{name}\" required></label>\
<label class=\"field\">Slug<input name=\"slug\" value=\"{slug}\" required></label>\
<label class=\"field\">Timezone<input name=\"timezone\" value=\"{tz}\"></label>\
<label class=\"field\">Day state<select name=\"state_key\">{state_opts}</select></label>\
<button class=\"btn\" type=\"submit\">Save store</button></div></form>\
<p class=\"sub\" style=\"margin-top:10px\">\
<a class=\"btn sec\" target=\"_blank\" href=\"/store/{slug}\">View this store's wall ↗</a></p></div>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Screen monitors</h2>\
<table><tbody>{screen_rows}</tbody></table></div>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Add a monitor</h2>\
<form method=\"post\" action=\"/admin/stores/{id}/screens\"><div class=\"row\">\
<label class=\"field\">Label<input name=\"label\" placeholder=\"Counter Left\" required></label>\
<label class=\"field\">Orientation<select name=\"orientation\">\
<option value=\"landscape\">landscape</option>\
<option value=\"portrait\">portrait</option></select></label>\
<label class=\"field\">Width<input name=\"width_px\" type=\"number\" value=\"1920\" required></label>\
<label class=\"field\">Height<input name=\"height_px\" type=\"number\" value=\"1080\" required></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"0\"></label>\
<button class=\"btn\" type=\"submit\">Add monitor</button></div></form></div>",
        id = store.id,
        name = esc(&store.name),
        slug = esc(&store.slug),
        tz = esc(&store.timezone),
        state_opts = state_options(states, &store.state_key),
        screen_rows = screen_rows,
    );
    admin_shell(&format!("{} · Admin", store.name), &inner)
}

// ---- Menu editor -----------------------------------------------------------

/// The menu page: list of categories + an add-category form.
pub fn menu_page(categories: &[(MenuCategoryRow, i64)]) -> String {
    let mut rows = String::new();
    if categories.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"4\" style=\"color:#9a8f7d\">No categories yet — add one below.</td></tr>",
        );
    }
    for (c, item_count) in categories {
        let avail = match (c.avail_from.as_deref(), c.avail_to.as_deref()) {
            (Some(f), Some(t)) => format!("{f}–{t}"),
            _ => "—".to_string(),
        };
        let days = if c.avail_days.is_empty() {
            "every day".to_string()
        } else {
            esc(&c.avail_days.join(", "))
        };
        rows.push_str(&format!(
            "<tr><td><b>{name}</b></td><td><code>{slug}</code></td>\
<td>{count} items</td><td>{avail} · {days}</td>\
<td class=\"actions\"><a class=\"btn sec\" href=\"/admin/menu/{id}\">Edit</a>\
<form class=\"inline\" method=\"post\" action=\"/admin/menu/{id}/delete\" \
onsubmit=\"return confirm('Delete category {name} and all its items?')\">\
<button class=\"btn danger\">Delete</button></form></td></tr>",
            name = esc(&c.name),
            slug = esc(&c.slug),
            count = item_count,
            avail = avail,
            days = days,
            id = c.id,
        ));
    }
    let inner = format!(
        "<h1>Menu</h1>\
<p class=\"sub\">Categories and items are stored in the database and drive every \
store's wall live. Editing here changes what the screens show on next refresh.</p>\
<table><thead><tr><th>Category</th><th>Slug</th><th>Items</th>\
<th>Availability</th><th>Actions</th></tr></thead><tbody>{rows}</tbody></table>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Add a category</h2>\
<form method=\"post\" action=\"/admin/menu\"><div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" required></label>\
<label class=\"field\">Slug<input name=\"slug\" placeholder=\"desserts\" required></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"0\"></label>\
<label class=\"field\">Avail from<input name=\"avail_from\" placeholder=\"06:00\"></label>\
<label class=\"field\">Avail to<input name=\"avail_to\" placeholder=\"11:00\"></label>\
<label class=\"field\">Days (csv)<input name=\"avail_days\" placeholder=\"sat,sun\"></label>\
<button class=\"btn\" type=\"submit\">Add category</button></div></form></div>",
        rows = rows
    );
    admin_shell("Menu · Admin", &inner)
}

/// The per-category editor: category fields + its items table + add-item form.
pub fn category_edit_page(cat: &MenuCategoryRow, items: &[MenuItemRow]) -> String {
    let mut item_rows = String::new();
    if items.is_empty() {
        item_rows.push_str(
            "<tr><td colspan=\"4\" style=\"color:#9a8f7d\">No items yet — add one below.</td></tr>",
        );
    }
    for it in items {
        let stock = if it.in_stock { " checked" } else { "" };
        let pmax = it.price_max.map(|m| format!("{m:.2}")).unwrap_or_default();
        let img = it.image.clone().unwrap_or_default();
        let desc = it.description.clone().unwrap_or_default();
        item_rows.push_str(&format!(
            "<tr><td colspan=\"4\"><form method=\"post\" action=\"/admin/items/{id}/update\">\
<div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" value=\"{name}\" required></label>\
<label class=\"field\">Price<input name=\"price_min\" type=\"number\" step=\"0.01\" value=\"{pmin:.2}\" required></label>\
<label class=\"field\">Max (range)<input name=\"price_max\" type=\"number\" step=\"0.01\" value=\"{pmax}\"></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"{pos}\"></label>\
<label class=\"field\" style=\"flex-direction:row;align-items:center;gap:8px\">\
<input name=\"in_stock\" type=\"checkbox\" value=\"true\"{stock}>In stock</label></div>\
<div class=\"row\">\
<label class=\"field\">Image URL<input name=\"image\" value=\"{img}\"></label>\
<label class=\"field\">Description<input name=\"description\" value=\"{desc}\"></label>\
<div class=\"actions\"><button class=\"btn\" type=\"submit\">Save</button></div></div></form>\
<form class=\"inline\" method=\"post\" action=\"/admin/items/{id}/delete\">\
<button class=\"btn danger\">Delete item</button></form></td></tr>",
            id = it.id,
            name = esc(&it.name),
            pmin = it.price_min,
            pmax = pmax,
            pos = it.position,
            stock = stock,
            img = esc(&img),
            desc = esc(&desc),
        ));
    }

    let days_csv = esc(&cat.avail_days.join(","));
    let inner = format!(
        "<div class=\"sub\" style=\"margin-bottom:8px\"><a href=\"/admin/menu\">← all categories</a></div>\
<h1>{name}</h1>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Category details</h2>\
<form method=\"post\" action=\"/admin/menu/{id}/update\"><div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" value=\"{name}\" required></label>\
<label class=\"field\">Slug<input name=\"slug\" value=\"{slug}\" required></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"{pos}\"></label>\
<label class=\"field\">Avail from<input name=\"avail_from\" value=\"{from}\" placeholder=\"06:00\"></label>\
<label class=\"field\">Avail to<input name=\"avail_to\" value=\"{to}\" placeholder=\"11:00\"></label>\
<label class=\"field\">Days (csv)<input name=\"avail_days\" value=\"{days}\" placeholder=\"sat,sun\"></label>\
<button class=\"btn\" type=\"submit\">Save category</button></div></form></div>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Items</h2>\
<table><tbody>{item_rows}</tbody></table></div>\
<div class=\"card2\"><h2 style=\"margin:0 0 6px;color:#e7b15a;font-size:14px;\
text-transform:uppercase;letter-spacing:0.1em;\">Add an item</h2>\
<form method=\"post\" action=\"/admin/menu/{id}/items\"><div class=\"row\">\
<label class=\"field\">Name<input name=\"name\" required></label>\
<label class=\"field\">Slug<input name=\"slug\" placeholder=\"mango-lassi\" required></label>\
<label class=\"field\">Price<input name=\"price_min\" type=\"number\" step=\"0.01\" value=\"0.00\" required></label>\
<label class=\"field\">Max (range)<input name=\"price_max\" type=\"number\" step=\"0.01\"></label>\
<label class=\"field\">Position<input name=\"position\" type=\"number\" value=\"0\"></label></div>\
<div class=\"row\">\
<label class=\"field\">Image URL<input name=\"image\"></label>\
<label class=\"field\">Description<input name=\"description\"></label>\
<button class=\"btn\" type=\"submit\">Add item</button></div></form></div>",
        id = cat.id,
        name = esc(&cat.name),
        slug = esc(&cat.slug),
        pos = cat.position,
        from = esc(cat.avail_from.as_deref().unwrap_or("")),
        to = esc(cat.avail_to.as_deref().unwrap_or("")),
        days = days_csv,
        item_rows = item_rows,
    );
    admin_shell(&format!("{} · Menu · Admin", cat.name), &inner)
}
