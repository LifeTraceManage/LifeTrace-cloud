//! Authentication HTTP route group.

mod native;
mod web;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(native::router())
        .merge(web::router())
}
