//! Photo staging and challenge HTTP route group.

pub mod challenge;
mod challenge_desktop;
pub mod staging;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(staging::router())
        .merge(challenge::router())
        .merge(challenge_desktop::router())
}
