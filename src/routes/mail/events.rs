use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use futures_util::stream::{self, Stream};

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/mail/events", get(mail_events))
}

async fn mail_events(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    principal.require_scope("mail:read")?;

    let user_id = principal.user_id.as_str().to_owned();
    let receiver = state.mail_realtime.subscribe();

    let stream = stream::unfold((receiver, user_id), |(mut receiver, user_id)| async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let visible = event
                        .user_id
                        .as_deref()
                        .map(|event_user_id| event_user_id == user_id)
                        .unwrap_or(true);
                    if !visible {
                        continue;
                    }

                    let event_type = event
                        .payload
                        .get("type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("mail.updated");

                    let item = Event::default()
                        .event(event_type)
                        .data(event.payload.to_string());
                    return Some((Ok(item), (receiver, user_id)));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text("keep-alive"),
    ))
}
