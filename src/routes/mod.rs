//! HTTP routes.

pub mod assistant;
pub mod auth;

pub mod files;
pub mod health;
pub mod mail;
pub mod meta;
pub mod photo;
pub mod privacy;
pub mod sync;

use axum::Router;

use crate::state::AppState;

/// Assemble the public Cloud HTTP surface.
///
pub fn router(state: AppState) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(assistant::router())
        .merge(meta::router())
        .merge(files::router())
        .merge(photo::router())
        .merge(mail::router())
        .merge(privacy::router())
        .merge(sync::router())
}
