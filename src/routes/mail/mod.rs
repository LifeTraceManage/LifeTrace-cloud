//! Mail HTTP route group.

mod api;
mod attachment;
mod list;
mod workspace;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(api::router())
        .merge(list::router())
        .merge(attachment::router())
        .merge(workspace::router())
}
