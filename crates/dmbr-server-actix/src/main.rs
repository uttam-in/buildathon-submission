//! Actix Web HTTP server that serves the menu boards as live webpages.
//!
//! Same routes and behavior as the Axum server — they share all logic via the
//! `dmbr-web` crate:
//! - `GET /`                                  — picker: choose a wall config
//! - `GET /config/{config}`                   — choose a day-state for a config
//! - `GET /board/{config}/{state}`            — gallery of that wall's screens
//! - `GET /screen/{config}/{state}/{screen}`  — one full-resolution screen page
//!
//! No database: the `Resources/` JSON files are read fresh on every request.

use std::env;

use actix_web::{web, App, HttpResponse, HttpServer};
use dmbr_web::{config_page, find_entry, gallery_page, picker_page, Resources, WebError};

/// Default Resources directory, relative to the workspace root.
const DEFAULT_RESOURCES: &str = "../Resources";
/// Default listen port.
const DEFAULT_PORT: u16 = 8081;

/// Maps a [`WebError`] to an HTTP response.
fn err_response(e: WebError) -> HttpResponse {
    match e {
        WebError::NotFound(m) => HttpResponse::NotFound().body(format!("404 — {m}")),
        WebError::Internal(m) => HttpResponse::InternalServerError().body(format!("500 — {m}")),
    }
}

/// `GET /` — the config picker.
async fn home(res: web::Data<Resources>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(picker_page(&res.catalog().await))
}

/// `GET /config/{config}` — choose a state for the given config.
async fn config(res: web::Data<Resources>, path: web::Path<String>) -> HttpResponse {
    let config = path.into_inner();
    let catalog = res.catalog().await;
    match find_entry(&catalog.configs, &config) {
        Some(cfg) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(config_page(cfg, &catalog)),
        None => err_response(WebError::NotFound(format!("config '{config}'"))),
    }
}

/// `GET /board/{config}/{state}` — gallery of the rendered wall's screens.
async fn board(res: web::Data<Resources>, path: web::Path<(String, String)>) -> HttpResponse {
    let (config, state) = path.into_inner();
    let catalog = res.catalog().await;
    let (Some(cfg), Some(st)) = (
        find_entry(&catalog.configs, &config),
        find_entry(&catalog.states, &state),
    ) else {
        return err_response(WebError::NotFound(format!("{config}/{state}")));
    };
    match res.render(&config, &state).await {
        Ok(output) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(gallery_page(cfg, st, &output)),
        Err(e) => err_response(e),
    }
}

/// `GET /screen/{config}/{state}/{screen}` — one screen at native resolution.
async fn screen(
    res: web::Data<Resources>,
    path: web::Path<(String, String, String)>,
) -> HttpResponse {
    let (config, state, screen) = path.into_inner();
    match res.render_screen(&config, &state, &screen).await {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(e) => err_response(e),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let root = env::var("RESOURCES_DIR").unwrap_or_else(|_| DEFAULT_RESOURCES.to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let resources = web::Data::new(Resources::new(&root));
    println!("dmbr-server-actix serving Resources at '{root}' on http://localhost:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(resources.clone())
            .route("/", web::get().to(home))
            .route("/config/{config}", web::get().to(config))
            .route("/board/{config}/{state}", web::get().to(board))
            .route("/screen/{config}/{state}/{screen}", web::get().to(screen))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
