//! HTTP routes.

pub mod assistant;
pub mod auth;
pub mod beecount;

pub mod files;
pub mod health;
pub mod mail;
pub mod mail_attachment;
pub mod mail_list;
pub mod meta;
pub mod photo_challenge;
pub mod photo_challenge_desktop;
pub mod photo_staging;
pub mod privacy;
pub mod sync;
pub mod web_auth;

use axum::Router;

use crate::state::AppState;

/// Assemble all routes into one router.
///
/// Database-backed Cloud deployments expose finance only through the BeeCount
/// surfaces. LifeTrace Web reads the same PostgreSQL BeeCount-compatible entity
/// store used by the stock BeeCount client. The historical LifeTrace finance
/// CRUD routes remain mounted solely for the in-memory protocol test harness.
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
        .merge(photo_staging::router())
        .merge(photo_challenge::router())
        .merge(photo_challenge_desktop::router())
        .merge(mail::router())
        .merge(mail_attachment::router())
        .merge(mail_list::router())
        .merge(privacy::router())
        .merge(sync::router())
}
