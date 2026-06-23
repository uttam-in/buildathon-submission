//! One-shot migration + admin-seed runner.
//!
//! Connects to `DATABASE_URL`, applies the `menuboard` schema, and seeds an
//! admin user from `ADMIN_USER` / `ADMIN_PASSWORD` (defaults `admin` / `admin`
//! for local dev). Idempotent: re-running is safe (CREATE ... IF NOT EXISTS,
//! and the admin upsert refreshes the password hash).
//!
//! Run: `DATABASE_URL=... ADMIN_USER=... ADMIN_PASSWORD=... cargo run -p dmbr-web --bin dmbr-migrate`

use std::env;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use sqlx::postgres::PgPoolOptions;

/// The schema DDL, embedded so the binary is self-contained.
const SCHEMA_SQL: &str = include_str!("../../../../migrations/0001_menuboard.sql");
/// The menu-catalog DDL.
const MENU_SQL: &str = include_str!("../../../../migrations/0002_menu.sql");
/// The featured-flag DDL.
const FEATURED_SQL: &str = include_str!("../../../../migrations/0003_featured.sql");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set")?;
    let admin_user = env::var("ADMIN_USER").unwrap_or_else(|_| "admin".into());
    let admin_pass = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into());
    if env::var("ADMIN_PASSWORD").is_err() {
        eprintln!(
            "WARNING: ADMIN_PASSWORD not set — seeding admin with the insecure default \
             'admin'. Set ADMIN_USER/ADMIN_PASSWORD before running outside local dev."
        );
    }

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;

    println!("connected; applying schema…");
    // Execute the whole DDL script. `sqlx::raw_sql` runs multiple statements.
    sqlx::raw_sql(SCHEMA_SQL).execute(&pool).await?;
    sqlx::raw_sql(MENU_SQL).execute(&pool).await?;
    sqlx::raw_sql(FEATURED_SQL).execute(&pool).await?;
    println!("schema applied (stores, screens, admin_users, menu_categories, menu_items)");

    // Seed the menu from Resources/menu.json (idempotent — only if empty).
    let menu_path = env::var("MENU_JSON").unwrap_or_else(|_| "../Resources/menu.json".into());
    match std::fs::read_to_string(&menu_path) {
        Ok(text) => {
            let menu: dmbr_convert::challenge::ChallengeMenu = serde_json::from_str(&text)?;
            let seeded = dmbr_web::db::seed_menu_from_json(&pool, &menu).await?;
            if seeded {
                let n = dmbr_web::db::menu_item_count(&pool).await?;
                println!("seeded menu from '{menu_path}' ({n} items)");
            } else {
                println!("menu already populated — skipped seeding");
            }
        }
        Err(e) => println!("note: could not read '{menu_path}' ({e}); skipped menu seed"),
    }

    // Hash the admin password with Argon2.
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(admin_pass.as_bytes(), &salt)
        .map_err(|e| format!("hash failed: {e}"))?
        .to_string();

    // Upsert the admin user (refresh hash on conflict).
    sqlx::query(
        "INSERT INTO menuboard.admin_users (username, password_hash)
         VALUES ($1, $2)
         ON CONFLICT (username) DO UPDATE SET password_hash = EXCLUDED.password_hash",
    )
    .bind(&admin_user)
    .bind(&hash)
    .execute(&pool)
    .await?;
    println!("seeded admin user '{admin_user}'");

    let store_count: i64 = sqlx::query_scalar("SELECT count(*) FROM menuboard.stores")
        .fetch_one(&pool)
        .await?;
    println!("done. stores in DB: {store_count}");
    Ok(())
}
