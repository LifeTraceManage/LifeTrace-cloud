//! LifeTrace cloud backend.
//!
//! SQLite is the single embedded persistence path. Sync wire compatibility is
//! defined by `lifetrace-contracts`; domain-specific HTTP adapters are grouped
//! behind explicit modules.

pub mod auth;
pub mod beecount;

pub mod config;
pub mod error;
pub mod http;
pub mod mail;
pub mod object_storage;
pub mod repository;
pub mod routes;
pub mod state;
pub mod sync;

pub use config::Config;
pub use error::ApiError;
pub use state::{AppState, StartupError};

use axum::http::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    HeaderName, Method,
};
use axum::{middleware, Router};
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};

const TAURI_DESKTOP_ORIGINS: &[&str] = &["http://tauri.localhost", "https://tauri.localhost"];

fn cors_origins(config: &Config) -> Vec<axum::http::HeaderValue> {
    let mut values = config.cors_allowed_origins.clone();
    for origin in TAURI_DESKTOP_ORIGINS {
        if !values.iter().any(|value| value == origin) {
            values.push((*origin).to_owned());
        }
    }
    values
        .iter()
        .filter_map(|value| value.parse().ok())
        .collect()
}

/// Build the full application router.
pub fn app(state: AppState) -> Router {
    let origins = cors_origins(&state.config);
    let cors = if origins.is_empty() {
        CorsLayer::new()
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                ACCEPT,
                AUTHORIZATION,
                CONTENT_TYPE,
                HeaderName::from_static("x-csrf-token"),
                HeaderName::from_static("x-request-id"),
                HeaderName::from_static("x-photo-challenge-key"),
            ])
    };

    let production = state.config.is_production();
    let rate_limiter = http::rate_limit::ApiRateLimiter::from_config(&state.config);
    let mut router = routes::router(state.clone())
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            http::rate_limit::middleware,
        ))
        .with_state(state)
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors);

    for layer in http::security::response_security_layers() {
        router = router.layer(layer);
    }
    if production {
        router = router.layer(http::security::hsts_layer());
    }

    let web_root = std::env::var("LIFETRACE_WEB_ROOT").unwrap_or_else(|_| "/app/web".to_owned());
    let photo_root = std::env::var("LIFETRACE_PHOTO_WEB_ROOT")
        .unwrap_or_else(|_| "/app/photo-challenge".to_owned());

    if std::path::Path::new(&photo_root).exists() {
        let index = format!("{photo_root}/index.html");
        router = router.nest_service(
            "/photo-challenge-upload",
            ServeDir::new(photo_root).not_found_service(ServeFile::new(index)),
        );
    }

    if std::path::Path::new(&web_root).exists() {
        let index = format!("{web_root}/index.html");
        router = router.fallback_service(
            ServeDir::new(web_root).not_found_service(ServeFile::new(index)),
        );
    }

    router
}
