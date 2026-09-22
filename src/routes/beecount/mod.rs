//! BeeCount HTTP route group.

pub mod account;
pub mod attachments;
pub mod compat;
pub mod stats;
pub mod web;
pub mod ws;

use axum::Router;
use crate::state::AppState;

pub fn router(attachment_max_upload_bytes: usize) -> Router<AppState> {
    Router::<AppState>::new()
        .merge(web::router())
        .merge(account::router())
        .merge(attachments::router(attachment_max_upload_bytes))
        .merge(compat::router())
        .merge(stats::router())
        .merge(ws::router())
}
