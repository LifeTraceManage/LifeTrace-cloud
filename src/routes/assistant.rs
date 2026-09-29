//! Authenticated LifeTrace Agent endpoints.
//!
//! Browser and native clients share the same authenticated runtime. Browser
//! mutations are CSRF-protected by the AuthenticatedPrincipal extractor.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::agent::{context, runtime, session};
use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

const MAX_PROMPT_CHARS: usize = 4_000;
const MAX_LEGACY_CONTEXT_BYTES: usize = 250_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantRequest {
    prompt: String,
    #[serde(default)]
    context: Value,
    #[serde(default)]
    session_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct Items<T> {
    items: Vec<T>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/web/assistant", post(assistant))
        .route("/api/v1/assistant", post(assistant))
        .route("/api/v1/assistant/sessions", get(list_sessions))
        .route(
            "/api/v1/assistant/sessions/{session_id}/messages",
            get(list_messages),
        )
}

async fn assistant(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<AssistantRequest>,
) -> Result<Json<runtime::AgentRunOutput>, ApiError> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "prompt must not be empty",
            StatusCode::BAD_REQUEST,
        ));
    }
    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            format!("prompt must not exceed {MAX_PROMPT_CHARS} characters"),
            StatusCode::BAD_REQUEST,
        ));
    }

    // Older web clients still send an eagerly-built context snapshot. The Agent
    // no longer trusts that client-side snapshot for answers: tools load
    // authorized server-side data instead. Keep accepting the field during the
    // migration so existing clients remain compatible, but retain the old size
    // guard to avoid unnecessarily large requests.
    let context_size = serde_json::to_vec(&request.context)
        .map(|value| value.len())
        .unwrap_or(MAX_LEGACY_CONTEXT_BYTES + 1);
    if context_size > MAX_LEGACY_CONTEXT_BYTES {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "assistant context is too large",
            StatusCode::PAYLOAD_TOO_LARGE,
        ));
    }

    runtime::run(&state, &principal, prompt, request.session_id)
        .await
        .map(Json)
        .map_err(map_runtime_error)
}

async fn list_sessions(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<ListQuery>,
) -> Result<Json<Items<session::AgentSession>>, ApiError> {
    let user_id = context::ensure_cloud_user(&state.pool, &principal.user_id)
        .await
        .map_err(map_database_error)?;
    let access = context::AgentAccessPartition::from_principal(&principal);
    let items = session::list_sessions(&state.pool, user_id, &access, query.limit.unwrap_or(30))
        .await
        .map_err(map_database_error)?;
    Ok(Json(Items { items }))
}

async fn list_messages(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(session_id): Path<Uuid>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Items<session::AgentMessage>>, ApiError> {
    let user_id = context::ensure_cloud_user(&state.pool, &principal.user_id)
        .await
        .map_err(map_database_error)?;
    let access = context::AgentAccessPartition::from_principal(&principal);
    let items =
        session::list_messages(&state.pool, user_id, &access, session_id, query.limit.unwrap_or(100))
            .await
            .map_err(map_database_error)?;
    Ok(Json(Items { items }))
}

fn map_runtime_error(error: runtime::AgentRuntimeError) -> ApiError {
    match error {
        runtime::AgentRuntimeError::Database(error) => map_database_error(error),
        runtime::AgentRuntimeError::Provider(message) => {
            tracing::warn!(error = %message, "agent provider failed");
            ApiError::new(
                ErrorCode::TemporarilyUnavailable,
                "assistant provider is temporarily unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            )
        }
    }
}

fn map_database_error(error: sqlx::Error) -> ApiError {
    if matches!(error, sqlx::Error::RowNotFound) {
        return ApiError::new(
            ErrorCode::InvalidRequest,
            "agent session was not found",
            StatusCode::NOT_FOUND,
        );
    }
    tracing::error!(error = %error, "agent database operation failed");
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "agent storage is temporarily unavailable",
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_context_limit_keeps_existing_client_payloads_bounded() {
        let value = serde_json::json!({"note.note": ["x".repeat(MAX_LEGACY_CONTEXT_BYTES)]});
        assert!(serde_json::to_vec(&value).unwrap().len() > MAX_LEGACY_CONTEXT_BYTES);
    }
}
