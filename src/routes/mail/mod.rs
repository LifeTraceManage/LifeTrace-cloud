//! Mail HTTP route group.

mod api;
mod attachment;
mod categories;
mod events;
mod list;
mod workspace;

use crate::state::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(api::router())
        .merge(events::router())
        .merge(list::router())
        .merge(attachment::router())
        .merge(categories::router())
        .merge(workspace::router())
}
