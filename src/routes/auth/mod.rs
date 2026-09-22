//! Authentication HTTP route group.

mod native;
mod web;

use axum::http::HeaderMap;
use axum::Router;

use crate::auth::security::{PeerAddr, RequestContext};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .merge(native::router())
        .merge(web::router())
}

fn context(state: &AppState, headers: &HeaderMap, peer: PeerAddr) -> RequestContext {
    RequestContext::from_headers(headers, peer.0, &state.config)
}
