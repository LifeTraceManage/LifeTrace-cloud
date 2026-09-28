use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/mail/events", get(mail_events))
}

async fn mail_events(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    principal.require_scope("mail:read")?;

    let user_id = principal.user_id.as_str().to_owned();
    let receiver = state.mail_realtime.subscribe();

    Ok(ws.on_upgrade(move |socket| stream_events(socket, receiver, user_id)))
}

async fn stream_events(
    mut socket: WebSocket,
    mut receiver: tokio::sync::broadcast::Receiver<crate::mail::realtime::MailRealtimeEvent>,
    user_id: String,
) {
    loop {
        match receiver.recv().await {
            Ok(event) => {
                let visible = event
                    .user_id
                    .as_deref()
                    .map(|event_user_id| event_user_id == user_id.as_str())
                    .unwrap_or(true);
                if !visible {
                    continue;
                }

                if socket
                    .send(Message::Text(event.payload.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
}
