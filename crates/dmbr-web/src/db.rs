//! Postgres data layer for the admin app: stores, screen monitors, and admin
//! auth. Uses sqlx with a runtime connection pool (no compile-time DB needed).

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

use dmbr_convert::challenge::ChallengeMenu;

/// A restaurant location.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Store {
    /// Primary key.
    pub id: Uuid,
    /// URL slug, e.g. `store-042`.
    pub slug: String,
    /// Display name.
    pub name: String,
    /// IANA timezone.
    pub timezone: String,
    /// Day-state key this store renders (a `states/*.json` stem).
    pub state_key: String,
}

/// A physical screen / TV monitor at a store.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Screen {
    /// Primary key.
    pub id: Uuid,
    /// Owning store.
    pub store_id: Uuid,
    /// Human label, e.g. "Counter Left".
    pub label: String,
    /// `landscape` or `portrait`.
    pub orientation: String,
    /// Width in pixels.
    pub width_px: i32,
    /// Height in pixels.
    pub height_px: i32,
    /// Ordering within the store's wall.
    pub position: i32,
}

/// Opens a Postgres connection pool.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

// ---- Admin auth ------------------------------------------------------------

/// Verifies a username/password against `menuboard.admin_users`. Returns true
/// on a match. Argon2 verification is constant-time per the library.
pub async fn verify_admin(pool: &PgPool, username: &str, password: &str) -> bool {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT password_hash FROM menuboard.admin_users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    let Some((hash,)) = row else {
        return false;
    };
    let Ok(parsed) = PasswordHash::new(&hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// ---- Stores ----------------------------------------------------------------

/// Lists all stores, ordered by slug.
pub async fn list_stores(pool: &PgPool) -> Result<Vec<Store>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, slug, name, timezone, state_key FROM menuboard.stores ORDER BY slug",
    )
    .fetch_all(pool)
    .await
}

/// Fetches one store by id.
pub async fn get_store(pool: &PgPool, id: Uuid) -> Result<Option<Store>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, slug, name, timezone, state_key FROM menuboard.stores WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Creates a store and returns its id.
pub async fn create_store(
    pool: &PgPool,
    slug: &str,
    name: &str,
    timezone: &str,
    state_key: &str,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO menuboard.stores (slug, name, timezone, state_key)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(slug)
    .bind(name)
    .bind(timezone)
    .bind(state_key)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Updates a store's editable fields.
pub async fn update_store(
    pool: &PgPool,
    id: Uuid,
    slug: &str,
    name: &str,
    timezone: &str,
    state_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE menuboard.stores
         SET slug = $2, name = $3, timezone = $4, state_key = $5, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(slug)
    .bind(name)
    .bind(timezone)
    .bind(state_key)
    .execute(pool)
    .await?;
    Ok(())
}

/// Deletes a store (cascades to its screens).
pub async fn delete_store(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM menuboard.stores WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Screens ---------------------------------------------------------------

/// Lists a store's screens in wall order.
pub async fn list_screens(pool: &PgPool, store_id: Uuid) -> Result<Vec<Screen>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, store_id, label, orientation, width_px, height_px, position
         FROM menuboard.screens WHERE store_id = $1 ORDER BY position, label",
    )
    .bind(store_id)
    .fetch_all(pool)
    .await
}

/// Adds a screen to a store.
pub async fn create_screen(
    pool: &PgPool,
    store_id: Uuid,
    label: &str,
    orientation: &str,
    width_px: i32,
    height_px: i32,
    position: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO menuboard.screens
         (store_id, label, orientation, width_px, height_px, position)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(store_id)
    .bind(label)
    .bind(orientation)
    .bind(width_px)
    .bind(height_px)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

/// Updates a screen.
pub async fn update_screen(
    pool: &PgPool,
    id: Uuid,
    label: &str,
    orientation: &str,
    width_px: i32,
    height_px: i32,
    position: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE menuboard.screens
         SET label = $2, orientation = $3, width_px = $4, height_px = $5,
             position = $6, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(label)
    .bind(orientation)
    .bind(width_px)
    .bind(height_px)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

/// Deletes a screen.
pub async fn delete_screen(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM menuboard.screens WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Menu: categories ------------------------------------------------------

/// A menu category row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MenuCategoryRow {
    /// Primary key.
    pub id: Uuid,
    /// Stable slug, e.g. `biryani-s`.
    pub slug: String,
    /// Display name.
    pub name: String,
    /// Display order.
    pub position: i32,
    /// Availability window start `HH:MM`, or None.
    pub avail_from: Option<String>,
    /// Availability window end `HH:MM`, or None.
    pub avail_to: Option<String>,
    /// Weekday codes the category is available; empty = any day.
    pub avail_days: Vec<String>,
}

/// A menu item row. Prices are read as f64 (via `::float8` casts).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MenuItemRow {
    /// Primary key.
    pub id: Uuid,
    /// Owning category.
    pub category_id: Uuid,
    /// Stable slug.
    pub slug: String,
    /// Display name.
    pub name: String,
    /// Single price, or the low end of a range.
    pub price_min: f64,
    /// High end of a range, or None for a single price.
    pub price_max: Option<f64>,
    /// Photo URL, or None.
    pub image: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the item is currently sellable (false = 86'd).
    pub in_stock: bool,
    /// Order within its category.
    pub position: i32,
}

/// Column list for menu items with prices cast to float8 for f64 decoding.
const ITEM_COLS: &str = "id, category_id, slug, name, \
price_min::float8 AS price_min, price_max::float8 AS price_max, \
image, description, in_stock, position";

/// Lists all categories in display order.
pub async fn list_categories(pool: &PgPool) -> Result<Vec<MenuCategoryRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, slug, name, position, avail_from, avail_to, avail_days
         FROM menuboard.menu_categories ORDER BY position, name",
    )
    .fetch_all(pool)
    .await
}

/// Fetches one category by id.
pub async fn get_category(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<MenuCategoryRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, slug, name, position, avail_from, avail_to, avail_days
         FROM menuboard.menu_categories WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Creates a category, returning its id.
pub async fn create_category(
    pool: &PgPool,
    slug: &str,
    name: &str,
    position: i32,
    avail_from: Option<&str>,
    avail_to: Option<&str>,
    avail_days: &[String],
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO menuboard.menu_categories
         (slug, name, position, avail_from, avail_to, avail_days)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(slug)
    .bind(name)
    .bind(position)
    .bind(avail_from)
    .bind(avail_to)
    .bind(avail_days)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Updates a category.
#[allow(clippy::too_many_arguments)]
pub async fn update_category(
    pool: &PgPool,
    id: Uuid,
    slug: &str,
    name: &str,
    position: i32,
    avail_from: Option<&str>,
    avail_to: Option<&str>,
    avail_days: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE menuboard.menu_categories
         SET slug=$2, name=$3, position=$4, avail_from=$5, avail_to=$6,
             avail_days=$7, updated_at=now()
         WHERE id=$1",
    )
    .bind(id)
    .bind(slug)
    .bind(name)
    .bind(position)
    .bind(avail_from)
    .bind(avail_to)
    .bind(avail_days)
    .execute(pool)
    .await?;
    Ok(())
}

/// Deletes a category (cascades to its items).
pub async fn delete_category(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM menuboard.menu_categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Menu: items -----------------------------------------------------------

/// Lists items in a category, in order.
pub async fn list_items(
    pool: &PgPool,
    category_id: Uuid,
) -> Result<Vec<MenuItemRow>, sqlx::Error> {
    sqlx::query_as(&format!(
        "SELECT {ITEM_COLS} FROM menuboard.menu_items
         WHERE category_id = $1 ORDER BY position, name"
    ))
    .bind(category_id)
    .fetch_all(pool)
    .await
}

/// Creates an item.
#[allow(clippy::too_many_arguments)]
pub async fn create_item(
    pool: &PgPool,
    category_id: Uuid,
    slug: &str,
    name: &str,
    price_min: f64,
    price_max: Option<f64>,
    image: Option<&str>,
    description: Option<&str>,
    in_stock: bool,
    position: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO menuboard.menu_items
         (category_id, slug, name, price_min, price_max, image, description, in_stock, position)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(category_id)
    .bind(slug)
    .bind(name)
    .bind(price_min)
    .bind(price_max)
    .bind(image)
    .bind(description)
    .bind(in_stock)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

/// Updates an item.
#[allow(clippy::too_many_arguments)]
pub async fn update_item(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    price_min: f64,
    price_max: Option<f64>,
    image: Option<&str>,
    description: Option<&str>,
    in_stock: bool,
    position: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE menuboard.menu_items
         SET name=$2, price_min=$3, price_max=$4, image=$5, description=$6,
             in_stock=$7, position=$8, updated_at=now()
         WHERE id=$1",
    )
    .bind(id)
    .bind(name)
    .bind(price_min)
    .bind(price_max)
    .bind(image)
    .bind(description)
    .bind(in_stock)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(())
}

/// Toggles an item's in-stock flag.
pub async fn set_item_stock(pool: &PgPool, id: Uuid, in_stock: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE menuboard.menu_items SET in_stock=$2, updated_at=now() WHERE id=$1")
        .bind(id)
        .bind(in_stock)
        .execute(pool)
        .await?;
    Ok(())
}

/// Looks up the owning category id of an item (for redirects).
pub async fn item_category(pool: &PgPool, id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar("SELECT category_id FROM menuboard.menu_items WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// Deletes an item.
pub async fn delete_item(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM menuboard.menu_items WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- Menu: seeding & full-menu build --------------------------------------

/// Returns the number of menu items currently in the DB.
pub async fn menu_item_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT count(*) FROM menuboard.menu_items")
        .fetch_one(pool)
        .await
}

/// Seeds the menu tables from a parsed challenge menu, if the DB is empty.
/// Idempotent: does nothing when items already exist.
pub async fn seed_menu_from_json(pool: &PgPool, menu: &ChallengeMenu) -> Result<bool, sqlx::Error> {
    if menu_item_count(pool).await? > 0 {
        return Ok(false);
    }
    for (ci, cat) in menu.categories.iter().enumerate() {
        let (from, to, days) = match &cat.availability {
            Some(a) => (a.from.clone(), a.to.clone(), a.days.clone()),
            None => (None, None, Vec::new()),
        };
        let cat_id = create_category(
            pool,
            &cat.id,
            &cat.name,
            ci as i32,
            from.as_deref(),
            to.as_deref(),
            &days,
        )
        .await?;
        for (ii, item) in cat.items.iter().enumerate() {
            let (pmin, pmax) = match (item.price, &item.price_range) {
                (Some(p), _) => (p, None),
                (None, Some(r)) => (r.min, Some(r.max)),
                (None, None) => (0.0, None),
            };
            create_item(
                pool,
                cat_id,
                &item.id,
                &item.name,
                pmin,
                pmax,
                item.image.as_deref(),
                item.description.as_deref(),
                true,
                ii as i32,
            )
            .await?;
        }
    }
    Ok(true)
}

/// Parses `HH:MM` into minutes since midnight; None on malformed input.
fn hhmm_to_min(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// Whether `now` is within `[start, end)`, treating `end <= start` as overnight.
fn in_window(now: u32, start: u32, end: u32) -> bool {
    if start < end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

/// Resolves whether a DB category is visible for the given day + time.
fn category_visible(cat: &MenuCategoryRow, day: &str, now: u32) -> bool {
    if !cat.avail_days.is_empty() && !cat.avail_days.iter().any(|d| d.eq_ignore_ascii_case(day)) {
        return false;
    }
    match (cat.avail_from.as_deref(), cat.avail_to.as_deref()) {
        (Some(f), Some(t)) => match (hhmm_to_min(f), hhmm_to_min(t)) {
            (Some(s), Some(e)) => in_window(now, s, e),
            _ => true,
        },
        (Some(f), None) => hhmm_to_min(f).map(|s| now >= s).unwrap_or(true),
        (None, Some(t)) => hhmm_to_min(t).map(|e| now < e).unwrap_or(true),
        (None, None) => true,
    }
}

/// Builds the renderer's `FullMenu` from the DB for a given `day` (e.g. "wed")
/// and `time` ("HH:MM"). Category day/time availability is resolved here (out-of
/// -window categories are dropped); out-of-stock items are dropped too. The
/// result feeds straight into `dmbr_core::render`. Items keep their `image` and
/// price-range display string.
pub async fn build_full_menu(
    pool: &PgPool,
    restaurant_id: &str,
    day: &str,
    time: &str,
) -> Result<dmbr_core::models::FullMenu, sqlx::Error> {
    use dmbr_core::models::{FullMenu, MenuCategory, MenuItem};

    let now = hhmm_to_min(time).unwrap_or(0);
    let cats = list_categories(pool).await?;
    let mut categories = Vec::new();
    let mut items = Vec::new();

    for cat in &cats {
        if !category_visible(cat, day, now) {
            continue; // out-of-window category disappears entirely
        }
        categories.push(MenuCategory {
            id: cat.slug.clone(),
            name: cat.name.clone(),
            display_order: cat.position as i64,
        });
        for it in list_items(pool, cat.id).await? {
            let price_display = it
                .price_max
                .map(|max| format!("${:.2}\u{2013}${:.2}", it.price_min, max));
            items.push(MenuItem {
                id: it.slug.clone(),
                name: it.name.clone(),
                price: it.price_min,
                category: cat.slug.clone(),
                available: it.in_stock, // false (86'd) is filtered by the engine
                display_order: it.position as i64,
                description: it.description.clone(),
                price_display,
                image: it.image.clone(),
            });
        }
    }

    Ok(FullMenu {
        restaurant_id: restaurant_id.to_string(),
        version: "db".into(),
        categories,
        items,
        meal_period_rules: Vec::new(),
    })
}
