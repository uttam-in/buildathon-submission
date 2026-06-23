//! Postgres data layer for the admin app: stores, screen monitors, and admin
//! auth. Uses sqlx with a runtime connection pool (no compile-time DB needed).

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

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
