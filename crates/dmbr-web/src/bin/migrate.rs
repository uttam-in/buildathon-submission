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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set")?;
    let admin_user = env::var("ADMIN_USER").unwrap_or_else(|_| "admin".into());
    let admin_pass = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into());

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;

    println!("connected; applying schema…");
    // Execute the whole DDL script. `sqlx::raw_sql` runs multiple statements.
    sqlx::raw_sql(SCHEMA_SQL).execute(&pool).await?;
    println!("schema applied (menuboard.stores, screens, admin_users)");

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
