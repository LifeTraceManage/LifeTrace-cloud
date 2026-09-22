//! HTTP routes.

pub mod assistant;
pub mod auth;
pub mod beecount;

pub mod files;
pub mod health;
pub mod mail;
pub mod meta;
pub mod photo;
pub mod privacy;
pub mod sync;
pub mod web_auth;

use axum::Router;

use crate::state::AppState;

/// Assemble the public Cloud HTTP surface.
///
/// Finance is exposed through the BeeCount-compatible PostgreSQL-backed routes;
/// the retired in-memory Finance CRUD example is no longer part of the router.
pub fn router(state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(beecount::router(
            state.config.beecount_attachment_max_upload_bytes,
        ))
        .merge(web_auth::router())
        .merge(assistant::router())
        .merge(meta::router())
        .merge(files::router())
        .merge(photo::router())
        .merge(mail::router())
        .merge(privacy::router())
        .merge(sync::router())
}
