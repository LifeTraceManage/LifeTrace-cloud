use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use lifetrace_contracts::ErrorCode;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::mail::domain::{MailDraftInput, MailIdentityInput};
use crate::mail::{MailService, MailServiceError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(
            "/api/v1/mail/identities",
            get(list_identities).post(create_identity),
        )
        .route(
            "/api/v1/mail/identities/{id}",
            patch(update_identity).delete(delete_identity),
        )
        .route("/api/v1/mail/drafts", get(list_drafts).post(create_draft))
        .route(
            "/api/v1/mail/drafts/{id}",
            patch(update_draft).delete(delete_draft),
        )
        .route("/api/v1/mail/drafts/{id}/send", post(send_draft))
        .route(
            "/api/v1/mail/drafts/{id}/attachments",
            get(list_draft_attachments).post(add_draft_attachment),
        )
        .route(
            "/api/v1/mail/drafts/{id}/attachments/{attachment_id}",
            delete(delete_draft_attachment),
        )
        .layer(DefaultBodyLimit::max(19 * 1024 * 1024))
}

fn service(state: &AppState) -> MailService {
    MailService::new(state.pool.clone(), state.database_enabled)
}

fn map_error(error: MailServiceError) -> ApiError {
    let (status, message) = match error {
        MailServiceError::DatabaseRequired => (
            StatusCode::SERVICE_UNAVAILABLE,
            "mail storage requires PostgreSQL",
        ),
        MailServiceError::InvalidUser | MailServiceError::InvalidAccount => {
            (StatusCode::BAD_REQUEST, "invalid mail request")
        }
        MailServiceError::AccountNotFound => (StatusCode::NOT_FOUND, "mail account not found"),
        MailServiceError::IdentityNotFound => (StatusCode::NOT_FOUND, "mail identity not found"),
        MailServiceError::DraftNotFound => (StatusCode::NOT_FOUND, "mail draft not found"),
        MailServiceError::MessageNotFound => (StatusCode::NOT_FOUND, "mail message not found"),
        MailServiceError::ThreadNotFound => (StatusCode::NOT_FOUND, "mail thread not found"),
        MailServiceError::ArchiveUnavailable => {
            (StatusCode::CONFLICT, "archive folder is unavailable")
        }
        MailServiceError::DestinationUnavailable => {
            (StatusCode::CONFLICT, "destination mail folder is unavailable")
        }
        MailServiceError::Credential => (
            StatusCode::SERVICE_UNAVAILABLE,
            "mail credential store is unavailable",
        ),
        MailServiceError::Protocol => (StatusCode::BAD_GATEWAY, "mail provider operation failed"),
        MailServiceError::Database => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "mail storage operation failed",
        ),
        MailServiceError::Parse => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "mail message could not be parsed",
        ),
        MailServiceError::SendInProgress => {
            (StatusCode::CONFLICT, "mail send is already in progress")
        }
    };
    let code = if status.is_server_error() {
        ErrorCode::TemporarilyUnavailable
    } else {
        ErrorCode::InvalidRequest
    };
    ApiError::new(code, message, status)
}

async fn list_identities(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_identities(&principal.user_id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn create_identity(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<MailIdentityInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    principal.require_scope("mail:write")?;
    let item = service(&state)
        .create_identity(&principal.user_id, input)
        .await
        .map_err(map_error)?;
    Ok((StatusCode::CREATED, Json(json!(item))))
}

async fn update_identity(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<MailIdentityInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let item = service(&state)
        .update_identity(&principal.user_id, id, input)
        .await
        .map_err(map_error)?;
    Ok(Json(json!(item)))
}

async fn delete_identity(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .delete_identity(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true })))
}

async fn list_drafts(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_drafts(&principal.user_id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn create_draft(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<MailDraftInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    principal.require_scope("mail:write")?;
    let item = service(&state)
        .create_draft(&principal.user_id, input)
        .await
        .map_err(map_error)?;
    Ok((StatusCode::CREATED, Json(json!(item))))
}

async fn update_draft(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<MailDraftInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let item = service(&state)
        .update_draft(&principal.user_id, id, input)
        .await
        .map_err(map_error)?;
    Ok(Json(json!(item)))
}

async fn delete_draft(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .delete_draft(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true })))
}

async fn send_draft(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let message_id = service(&state)
        .send_draft(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true, "messageId": message_id })))
}


async fn list_draft_attachments(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_draft_attachments(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn add_draft_attachment(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    principal.require_scope("mail:write")?;
    let mut file = None;
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid attachment upload",
            StatusCode::BAD_REQUEST,
        )
    })? {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("attachment").to_owned();
        let mime_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_owned();
        let bytes = field.bytes().await.map_err(|_| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "invalid attachment payload",
                StatusCode::BAD_REQUEST,
            )
        })?;
        file = Some((filename, mime_type, bytes.to_vec()));
        break;
    }
    let (filename, mime_type, content) = file.ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "attachment file is required",
            StatusCode::BAD_REQUEST,
        )
    })?;
    let item = service(&state)
        .add_draft_attachment(&principal.user_id, id, filename, mime_type, content)
        .await
        .map_err(map_error)?;
    Ok((StatusCode::CREATED, Json(json!(item))))
}

async fn delete_draft_attachment(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path((id, attachment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .delete_draft_attachment(&principal.user_id, id, attachment_id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true })))
}
