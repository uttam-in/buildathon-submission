//! Axum HTTP server: serves the menu boards as live webpages, plus a
//! Postgres-backed admin UI for managing stores and their screen monitors.
//!
//! Public routes (no DB needed for the demo renderer):
//! - `GET /`                                  — picker: choose a wall config
//! - `GET /config/{config}`                   — choose a day-state for a config
//! - `GET /board/{config}/{state}`            — gallery of that wall's screens
//! - `GET /screen/{config}/{state}/{screen}`  — one full-resolution screen page
//!
//! Store routes (DB-backed; a store's monitors define its wall):
//! - `GET /store/{slug}`                      — gallery of a store's screens
//! - `GET /store/{slug}/{screen_id}`          — one store screen at native res
//!
//! Admin routes (session-cookie auth):
//! - `GET/POST /admin/login`, `POST /admin/logout`
//! - `GET /admin/stores`, `POST /admin/stores`
//! - `GET /admin/stores/{id}`, `POST /admin/stores/{id}/update|delete`
//! - `POST /admin/stores/{id}/screens`
//! - `POST /admin/screens/{id}/update|delete`
//!
//! No menu database: the `Resources/` JSON (menu + day-states) is read fresh on
//! each request. Only stores/monitors/admins live in Postgres.

mod auth;

use std::env;
use std::sync::Arc;

use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use dmbr_web::admin_pages::{login_page, store_edit_page, stores_page};
use dmbr_web::{config_page, db, find_entry, gallery_page, picker_page, Entry, Resources, WebError};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::SessionKey;

const DEFAULT_RESOURCES: &str = "../Resources";
const DEFAULT_PORT: u16 = 8080;

/// Shared application state.
#[derive(Clone)]
struct AppState {
    resources: Arc<Resources>,
    pool: PgPool,
    session: SessionKey,
}

#[tokio::main]
async fn main() {
    let root = env::var("RESOURCES_DIR").unwrap_or_else(|_| DEFAULT_RESOURCES.to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let secret = env::var("SESSION_SECRET").unwrap_or_else(|_| "dev-insecure-secret".into());

    let pool = db::connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    let state = AppState {
        resources: Arc::new(Resources::new(&root)),
        pool,
        session: SessionKey::new(secret.as_bytes()),
    };

    let app = Router::new()
        // public renderer
        .route("/", get(home))
        .route("/config/:config", get(config))
        .route("/board/:config/:state", get(board))
        .route("/screen/:config/:state/:screen", get(screen))
        // store walls (DB-backed)
        .route("/store/:slug", get(store_wall))
        .route("/store/:slug/:screen_id", get(store_screen))
        // admin
        .route("/admin", get(|| async { Redirect::to("/admin/stores") }))
        .route("/admin/login", get(login_form).post(login_submit))
        .route("/admin/logout", post(logout))
        .route("/admin/stores", get(admin_stores).post(admin_create_store))
        .route("/admin/stores/:id", get(admin_store))
        .route("/admin/stores/:id/update", post(admin_update_store))
        .route("/admin/stores/:id/delete", post(admin_delete_store))
        .route("/admin/stores/:id/screens", post(admin_create_screen))
        .route("/admin/screens/:id/update", post(admin_update_screen))
        .route("/admin/screens/:id/delete", post(admin_delete_screen))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));
    println!("dmbr-server-axum: Resources='{root}' on http://localhost:{port} (admin at /admin)");
    axum::serve(listener, app).await.expect("server error");
}

/// Maps a [`WebError`] to an HTTP response.
fn err_response(e: WebError) -> Response {
    match e {
        WebError::NotFound(m) => (StatusCode::NOT_FOUND, format!("404 — {m}")).into_response(),
        WebError::Internal(m) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("500 — {m}")).into_response()
        }
    }
}

// ---- Public renderer routes ------------------------------------------------

async fn home(State(s): State<AppState>) -> Html<String> {
    Html(picker_page(&s.resources.catalog()))
}

async fn config(State(s): State<AppState>, Path(config): Path<String>) -> Response {
    let catalog = s.resources.catalog();
    match find_entry(&catalog.configs, &config) {
        Some(cfg) => Html(config_page(cfg, &catalog)).into_response(),
        None => err_response(WebError::NotFound(format!("config '{config}'"))),
    }
}

async fn board(
    State(s): State<AppState>,
    Path((config, state)): Path<(String, String)>,
) -> Response {
    let catalog = s.resources.catalog();
    let (Some(cfg), Some(st)) = (
        find_entry(&catalog.configs, &config),
        find_entry(&catalog.states, &state),
    ) else {
        return err_response(WebError::NotFound(format!("{config}/{state}")));
    };
    match s.resources.render(&config, &state) {
        Ok(output) => Html(gallery_page(cfg, st, &output)).into_response(),
        Err(e) => err_response(e),
    }
}

async fn screen(
    State(s): State<AppState>,
    Path((config, state, screen)): Path<(String, String, String)>,
) -> Response {
    match s.resources.render_screen(&config, &state, &screen) {
        Ok(html) => Html(html).into_response(),
        Err(e) => err_response(e),
    }
}

// ---- Store wall routes (DB-backed) -----------------------------------------

/// Loads a store by slug and its screens as `(screen_id, orientation, w, h)`.
/// The screen id is the monitor's UUID, so it is stable and unique per wall.
async fn load_store_screens(
    pool: &PgPool,
    slug: &str,
) -> Result<(db::Store, Vec<(String, String, u32, u32)>), WebError> {
    let store = sqlx::query_as::<_, db::Store>(
        "SELECT id, slug, name, timezone, state_key FROM menuboard.stores WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| WebError::Internal(e.to_string()))?
    .ok_or_else(|| WebError::NotFound(format!("store '{slug}'")))?;

    let screens = db::list_screens(pool, store.id)
        .await
        .map_err(|e| WebError::Internal(e.to_string()))?
        .into_iter()
        .map(|sc| {
            (
                sc.id.to_string(),
                sc.orientation,
                sc.width_px as u32,
                sc.height_px as u32,
            )
        })
        .collect();
    Ok((store, screens))
}

async fn store_wall(State(s): State<AppState>, Path(slug): Path<String>) -> Response {
    let (store, screens) = match load_store_screens(&s.pool, &slug).await {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    if screens.is_empty() {
        return (
            StatusCode::OK,
            Html(format!(
                "<body style='font-family:system-ui;background:#0e0b09;color:#f3ece1;padding:48px'>\
<h1>{}</h1><p>No screen monitors configured for this store yet. \
Add some in <a style='color:#f4c87a' href='/admin/stores'>admin</a>.</p></body>",
                store.name
            )),
        )
            .into_response();
    }
    match s.resources.render_store_screens(&store.state_key, &screens) {
        Ok(output) => {
            let cfg = Entry { key: slug.clone(), name: store.name.clone() };
            let st = Entry { key: store.state_key.clone(), name: store.state_key.clone() };
            Html(store_gallery(&cfg, &st, &output)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn store_screen(
    State(s): State<AppState>,
    Path((slug, screen_id)): Path<(String, String)>,
) -> Response {
    let (store, screens) = match load_store_screens(&s.pool, &slug).await {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    match s.resources.render_store_screens(&store.state_key, &screens) {
        Ok(output) => match output.screens.into_iter().find(|sc| sc.screen_id == screen_id) {
            Some(sc) => Html(sc.html_content).into_response(),
            None => err_response(WebError::NotFound(format!("screen '{screen_id}'"))),
        },
        Err(e) => err_response(e),
    }
}

/// Gallery for a store wall (screens link to `/store/{slug}/{id}`).
fn store_gallery(
    config: &Entry,
    state: &Entry,
    output: &dmbr_core::models::LayoutOutput,
) -> String {
    let mut cards = String::new();
    for sc in &output.screens {
        cards.push_str(&format!(
            "<li style='list-style:none'><a target='_blank' \
href='/store/{slug}/{sid}' style='display:flex;flex-direction:column;gap:6px;\
background:#181410;border:1px solid #3a2e1f;border-radius:14px;padding:20px;\
text-decoration:none;color:#f4c87a'><span style='font-size:19px;font-weight:700;\
color:#fbf5ea'>{sid}</span><span style='font-size:13px;color:#9a8f7d'>{n} items</span></a></li>",
            slug = config.key,
            sid = sc.screen_id,
            n = sc.item_count,
        ));
    }
    format!(
        "<!DOCTYPE html><html><head><meta charset='utf-8'><title>{name}</title></head>\
<body style='font-family:system-ui;background:radial-gradient(120% 90% at 0% 0%,#1c140e,#070605);\
color:#f3ece1;padding:48px;min-height:100vh'>\
<h1 style='color:#fbf5ea'>{name}</h1>\
<p style='color:#9a8f7d'>{n} screen(s) · meal period: {period} · render_hash {hash}</p>\
<ul style='display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:14px;\
padding:0;margin-top:18px'>{cards}</ul></body></html>",
        name = config.name,
        n = output.screens.len(),
        period = state.name,
        hash = output.render_hash,
        cards = cards,
    )
}

// ---- Admin: auth -----------------------------------------------------------

/// Returns the admin username if the request carries a valid session cookie.
fn current_admin(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    auth::session_user(&state.session, cookie)
}

/// Guards an admin handler: returns `Ok(user)` or a redirect to the login page.
fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<String, Response> {
    current_admin(state, headers).ok_or_else(|| Redirect::to("/admin/login").into_response())
}

async fn login_form() -> Html<String> {
    Html(login_page(None))
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_submit(State(s): State<AppState>, Form(form): Form<LoginForm>) -> Response {
    if db::verify_admin(&s.pool, &form.username, &form.password).await {
        let cookie = auth::make_cookie(&s.session, &form.username);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::SET_COOKIE, cookie.parse().unwrap());
        (headers, Redirect::to("/admin/stores")).into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Html(login_page(Some("Invalid username or password."))),
        )
            .into_response()
    }
}

async fn logout() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        auth::clear_cookie().parse().unwrap(),
    );
    (headers, Redirect::to("/admin/login")).into_response()
}

// ---- Admin: stores ---------------------------------------------------------

async fn admin_stores(State(s): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    match db::list_stores(&s.pool).await {
        Ok(stores) => Html(stores_page(&stores)).into_response(),
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

#[derive(Deserialize)]
struct StoreForm {
    slug: String,
    name: String,
    timezone: String,
    state_key: String,
}

async fn admin_create_store(
    State(s): State<AppState>,
    headers: HeaderMap,
    Form(f): Form<StoreForm>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    match db::create_store(&s.pool, &f.slug, &f.name, &f.timezone, &f.state_key).await {
        Ok(_) => Redirect::to("/admin/stores").into_response(),
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

async fn admin_store(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    let store = match db::get_store(&s.pool, id).await {
        Ok(Some(st)) => st,
        Ok(None) => return err_response(WebError::NotFound(format!("store {id}"))),
        Err(e) => return err_response(WebError::Internal(e.to_string())),
    };
    let screens = db::list_screens(&s.pool, id).await.unwrap_or_default();
    let states = s.resources.catalog().states;
    Html(store_edit_page(&store, &screens, &states)).into_response()
}

async fn admin_update_store(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Form(f): Form<StoreForm>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    match db::update_store(&s.pool, id, &f.slug, &f.name, &f.timezone, &f.state_key).await {
        Ok(_) => Redirect::to(&format!("/admin/stores/{id}")).into_response(),
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

async fn admin_delete_store(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    match db::delete_store(&s.pool, id).await {
        Ok(_) => Redirect::to("/admin/stores").into_response(),
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

// ---- Admin: screens --------------------------------------------------------

#[derive(Deserialize)]
struct ScreenForm {
    label: String,
    orientation: String,
    width_px: i32,
    height_px: i32,
    #[serde(default)]
    position: i32,
}

async fn admin_create_screen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(store_id): Path<Uuid>,
    Form(f): Form<ScreenForm>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    match db::create_screen(
        &s.pool, store_id, &f.label, &f.orientation, f.width_px, f.height_px, f.position,
    )
    .await
    {
        Ok(_) => Redirect::to(&format!("/admin/stores/{store_id}")).into_response(),
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

async fn admin_update_screen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Form(f): Form<ScreenForm>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    let store_id: Option<Uuid> =
        sqlx::query_scalar("SELECT store_id FROM menuboard.screens WHERE id = $1")
            .bind(id)
            .fetch_optional(&s.pool)
            .await
            .ok()
            .flatten();
    match db::update_screen(
        &s.pool, id, &f.label, &f.orientation, f.width_px, f.height_px, f.position,
    )
    .await
    {
        Ok(_) => match store_id {
            Some(sid) => Redirect::to(&format!("/admin/stores/{sid}")).into_response(),
            None => Redirect::to("/admin/stores").into_response(),
        },
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}

async fn admin_delete_screen(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(r) = require_admin(&s, &headers) {
        return r;
    }
    let store_id: Option<Uuid> =
        sqlx::query_scalar("SELECT store_id FROM menuboard.screens WHERE id = $1")
            .bind(id)
            .fetch_optional(&s.pool)
            .await
            .ok()
            .flatten();
    match db::delete_screen(&s.pool, id).await {
        Ok(_) => match store_id {
            Some(sid) => Redirect::to(&format!("/admin/stores/{sid}")).into_response(),
            None => Redirect::to("/admin/stores").into_response(),
        },
        Err(e) => err_response(WebError::Internal(e.to_string())),
    }
}
