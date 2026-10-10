//! EPIC-17 privacy export and account data-lifecycle endpoints.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use lifetrace_contracts::registry::{EntityOwnership, REGISTRY};
use lifetrace_contracts::ErrorCode;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::extract::authenticate_bearer_or_web_session;
use crate::auth::scope;
use crate::auth::security::{clear_session_cookie, cookie_value};
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::error::ApiError;
use crate::state::AppState;

const SUPPORTED_MODULES: &[&str] = &[
    "account",
    "devices",
    "sessions",
    "notes",
    "files",
    "english",
    "habits",
    "reviews",
    "workouts",
    "execution",
    "mail",
    "agent",
];

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/privacy/export", get(export_all))
        .route("/api/v1/privacy/export/{module}", get(export_module))
        .route("/api/v1/privacy/policy", get(policy))
        .route("/api/v1/privacy/account", delete(delete_account))
}

fn api_error(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> ApiError {
    ApiError::new(code, message, status)
}

fn db_error(error: sqlx::Error) -> ApiError {
    api_error(
        ErrorCode::TemporarilyUnavailable,
        format!("privacy database operation failed: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

async fn write_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization.is_some() {
        return state
            .auth
            .authenticate(AuthCredential::Bearer(authorization))
            .await;
    }

    let raw_session = cookie_value(headers, &state.config.auth_cookie_name).unwrap_or_default();
    let csrf = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    state
        .auth_service
        .verify_web_csrf(&raw_session, csrf, origin)
        .await
}

fn module_for_entity(entity_type: &str) -> Option<&'static str> {
    scope::required_entity_scope(entity_type, false)
        .and_then(|value| value.split_once(':').map(|(module, _)| module))
}

fn module_read_scope(module: &str) -> Option<&'static str> {
    match module {
        "account" => Some("account:read"),
        "devices" => Some("devices:read"),
        "sessions" => Some("sessions:read"),
        "notes" => Some("notes:read"),
        "files" => Some("files:read"),
        "english" => Some("english:read"),
        "habits" => Some("habits:read"),
        "reviews" => Some("reviews:read"),
        "workouts" => Some("workouts:read"),
        "execution" => Some("execution:read"),
        "mail" => Some("mail:read"),
        "agent" => Some("account:read"),
        _ => None,
    }
}

fn validate_module(module: &str) -> Result<(), ApiError> {
    if SUPPORTED_MODULES.contains(&module) {
        Ok(())
    } else {
        Err(api_error(
            ErrorCode::InvalidRequest,
            format!("unsupported export module: {module}"),
            StatusCode::BAD_REQUEST,
        ))
    }
}

fn database_user_id(principal: &AuthenticatedPrincipal) -> Option<Uuid> {
    Uuid::parse_str(principal.user_id.as_str()).ok()
}

async fn safe_account_profile(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<Value, ApiError> {
    let Some(user_id) = database_user_id(principal) else {
        return Ok(json!({"userId": principal.user_id.as_str()}));
    };
    let row = sqlx::query(
        "SELECT id,status,email,display_name,created_at,updated_at,email_verified_at,disabled_at,registration_source \
         FROM cloud_users WHERE id=$1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(json!({"userId": principal.user_id.as_str()}));
    };
    Ok(json!({
        "userId": row.try_get::<Uuid, _>("id").ok().map(|value| value.to_string()),
        "status": row.try_get::<String, _>("status").ok(),
        "email": row.try_get::<Option<String>, _>("email").ok().flatten(),
        "displayName": row.try_get::<Option<String>, _>("display_name").ok().flatten(),
        "createdAt": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
        "updatedAt": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok(),
        "emailVerifiedAt": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("email_verified_at").ok().flatten(),
        "disabledAt": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("disabled_at").ok().flatten(),
        "registrationSource": row.try_get::<Option<String>, _>("registration_source").ok().flatten()
    }))
}

async fn json_array(state: &AppState, sql: &str, user_id: Uuid) -> Result<Value, ApiError> {
    let raw = sqlx::query_scalar::<_, String>(sql)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
    serde_json::from_str(&raw).map_err(|error| {
        api_error(
            ErrorCode::InternalError,
            format!("failed to decode privacy export JSON: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })
}

async fn agent_json_array(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    sql: &str,
    user_id: Uuid,
) -> Result<Value, ApiError> {
    let scopes_json = serde_json::to_string(&principal.scopes).map_err(|error| {
        api_error(
            ErrorCode::InternalError,
            format!("failed to encode agent export scope partition: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let raw = sqlx::query_scalar::<_, String>(sql)
        .bind(user_id)
        .bind(principal.app_id.as_str())
        .bind(scopes_json)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
    serde_json::from_str(&raw).map_err(|error| {
        api_error(
            ErrorCode::InternalError,
            format!("failed to decode agent privacy export JSON: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })
}

async fn database_section(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    module: &str,
) -> Result<Option<Value>, ApiError> {
    let Some(user_id) = database_user_id(principal) else {
        return Ok(None);
    };
    match module {
        "devices" => Ok(Some(
            json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'appId',app_id,'platform',platform,'clientVersion',client_version,
                    'status',status,'deviceName',device_name,'firstSeenAt',first_seen_at,
                    'lastSeenAt',last_seen_at,'lastSyncAt',last_sync_at,'revokedAt',revoked_at
                )), '[]') FROM cloud_devices WHERE user_id=$1 ORDER BY first_seen_at",
                user_id,
            )
            .await?,
        )),
        "sessions" => Ok(Some(
            json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'deviceId',CASE WHEN typeof(device_id)='blob' THEN lower(hex(device_id)) ELSE device_id END,'appId',app_id,'scopes',json(scopes),
                    'sessionType',session_type,'status',status,'createdAt',created_at,
                    'lastSeenAt',last_seen_at,'idleExpiresAt',idle_expires_at,
                    'absoluteExpiresAt',absolute_expires_at,'revokedAt',revoked_at
                )), '[]') FROM auth_sessions WHERE user_id=$1 ORDER BY created_at",
                user_id,
            )
            .await?,
        )),
        "mail" => {
            let accounts = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'provider',provider,'emailAddress',email_address,'displayName',display_name,
                    'imapHost',imap_host,'imapPort',imap_port,'smtpHost',smtp_host,'smtpPort',smtp_port,
                    'status',status,'lastSyncAt',last_sync_at,'createdAt',created_at
                )), '[]') FROM mail_accounts WHERE user_id=$1",
                user_id,
            )
            .await?;
            let identities = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'accountId',CASE WHEN typeof(account_id)='blob' THEN lower(hex(account_id)) ELSE account_id END,'emailAddress',email_address,
                    'displayName',display_name,'replyTo',reply_to,'signature',signature_html,
                    'isDefault',is_default,'createdAt',created_at,'updatedAt',updated_at,
                    'deletedAt',deleted_at
                )), '[]') FROM mail_identities WHERE user_id=$1 ORDER BY created_at",
                user_id,
            )
            .await?;
            let messages = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'accountId',CASE WHEN typeof(account_id)='blob' THEN lower(hex(account_id)) ELSE account_id END,'threadId',CASE WHEN typeof(thread_id)='blob' THEN lower(hex(thread_id)) ELSE thread_id END,'subject',subject,
                    'from',json(from_json),'to',json(to_json),'receivedAt',received_at,
                    'isRead',is_read,'snippet',snippet,'hasAttachments',has_attachments
                )), '[]') FROM mail_messages WHERE user_id=$1 ORDER BY received_at",
                user_id,
            )
            .await?;
            let attachments = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'messageId',CASE WHEN typeof(message_id)='blob' THEN lower(hex(message_id)) ELSE message_id END,'filename',filename,'mimeType',mime_type,
                    'sizeBytes',size_bytes,'downloadState',download_state,'createdAt',created_at
                )), '[]') FROM mail_attachments WHERE user_id=$1 ORDER BY created_at",
                user_id,
            )
            .await?;
            let drafts = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'accountId',CASE WHEN typeof(account_id)='blob' THEN lower(hex(account_id)) ELSE account_id END,'identityId',CASE WHEN identity_id IS NULL THEN NULL WHEN typeof(identity_id)='blob' THEN lower(hex(identity_id)) ELSE identity_id END,'subject',subject,
                    'bodyText',body_text,'state',state,'createdAt',created_at,'updatedAt',updated_at
                )), '[]') FROM mail_drafts WHERE user_id=$1 ORDER BY created_at",
                user_id,
            )
            .await?;
            let draft_attachments = json_array(
                state,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'draftId',CASE WHEN typeof(draft_id)='blob' THEN lower(hex(draft_id)) ELSE draft_id END,'filename',filename,'mimeType',mime_type,
                    'sizeBytes',size_bytes,'createdAt',created_at
                )), '[]') FROM mail_draft_attachments WHERE user_id=$1 ORDER BY created_at",
                user_id,
            )
            .await?;
            Ok(Some(json!({
                "accounts": accounts,
                "identities": identities,
                "messages": messages,
                "attachments": attachments,
                "drafts": drafts,
                "draftAttachments": draft_attachments
            })))
        }
        "agent" => {
            let sessions = agent_json_array(
                state,
                principal,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(id)='blob' THEN lower(hex(id)) ELSE id END,'title',title,'status',status,'appId',app_id,'scopes',json(scopes_json),
                    'createdAt',created_at,'updatedAt',updated_at,'lastMessageAt',last_message_at
                )), '[]') FROM agent_sessions
                WHERE user_id=$1 AND app_id=$2 AND scopes_json=$3
                ORDER BY created_at",
                user_id,
            )
            .await?;
            let messages = agent_json_array(
                state,
                principal,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(m.id)='blob' THEN lower(hex(m.id)) ELSE m.id END,
                    'sessionId',CASE WHEN typeof(m.session_id)='blob' THEN lower(hex(m.session_id)) ELSE m.session_id END,
                    'runId',CASE WHEN m.run_id IS NULL THEN NULL WHEN typeof(m.run_id)='blob' THEN lower(hex(m.run_id)) ELSE m.run_id END,'role',m.role,'content',m.content,
                    'provider',m.provider,'metadata',json(m.metadata_json),'createdAt',m.created_at
                )), '[]') FROM agent_messages m
                JOIN agent_sessions s ON s.id=m.session_id
                WHERE m.user_id=$1 AND s.app_id=$2 AND s.scopes_json=$3
                ORDER BY m.created_at,m.rowid",
                user_id,
            )
            .await?;
            let runs = agent_json_array(
                state,
                principal,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(r.id)='blob' THEN lower(hex(r.id)) ELSE r.id END,
                    'sessionId',CASE WHEN typeof(r.session_id)='blob' THEN lower(hex(r.session_id)) ELSE r.session_id END,'status',r.status,'provider',r.provider,'model',r.model,
                    'prompt',r.prompt,'errorCode',r.error_code,'errorMessage',r.error_message,
                    'createdAt',r.created_at,'startedAt',r.started_at,'finishedAt',r.finished_at
                )), '[]') FROM agent_runs r
                JOIN agent_sessions s ON s.id=r.session_id
                WHERE r.user_id=$1 AND s.app_id=$2 AND s.scopes_json=$3
                ORDER BY r.created_at,r.rowid",
                user_id,
            )
            .await?;
            let tool_calls = agent_json_array(
                state,
                principal,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(t.id)='blob' THEN lower(hex(t.id)) ELSE t.id END,
                    'runId',CASE WHEN typeof(t.run_id)='blob' THEN lower(hex(t.run_id)) ELSE t.run_id END,
                    'sessionId',CASE WHEN typeof(t.session_id)='blob' THEN lower(hex(t.session_id)) ELSE t.session_id END,'toolName',t.tool_name,
                    'arguments',json(t.arguments_json),'status',t.status,'result',json(t.result_json),
                    'errorMessage',t.error_message,'requiresApproval',t.requires_approval,
                    'startedAt',t.started_at,'finishedAt',t.finished_at
                )), '[]') FROM agent_tool_calls t
                JOIN agent_sessions s ON s.id=t.session_id
                WHERE t.user_id=$1 AND s.app_id=$2 AND s.scopes_json=$3
                ORDER BY t.started_at,t.rowid",
                user_id,
            )
            .await?;
            let approvals = agent_json_array(
                state,
                principal,
                "SELECT COALESCE(json_group_array(json_object(
                    'id',CASE WHEN typeof(a.id)='blob' THEN lower(hex(a.id)) ELSE a.id END,
                    'runId',CASE WHEN typeof(a.run_id)='blob' THEN lower(hex(a.run_id)) ELSE a.run_id END,
                    'sessionId',CASE WHEN typeof(a.session_id)='blob' THEN lower(hex(a.session_id)) ELSE a.session_id END,
                    'toolCallId',CASE WHEN a.tool_call_id IS NULL THEN NULL WHEN typeof(a.tool_call_id)='blob' THEN lower(hex(a.tool_call_id)) ELSE a.tool_call_id END,
                    'actionName',a.action_name,'action',json(a.action_json),'status',a.status,
                    'requestedAt',a.requested_at,'decidedAt',a.decided_at,'expiresAt',a.expires_at,
                    'supersededByApprovalId',CASE WHEN a.superseded_by_approval_id IS NULL THEN NULL WHEN typeof(a.superseded_by_approval_id)='blob' THEN lower(hex(a.superseded_by_approval_id)) ELSE a.superseded_by_approval_id END,
                    'cancellationReason',a.cancellation_reason
                )), '[]') FROM agent_approvals a
                JOIN agent_sessions s ON s.id=a.session_id
                WHERE a.user_id=$1 AND s.app_id=$2 AND s.scopes_json=$3
                ORDER BY a.requested_at,a.rowid",
                user_id,
            )
            .await?;
            Ok(Some(json!({
                "sessions": sessions,
                "messages": messages,
                "runs": runs,
                "toolCalls": tool_calls,
                "approvals": approvals
            })))
        }
        _ => Ok(None),
    }
}

async fn build_export(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    requested_module: Option<&str>,
) -> Result<Value, ApiError> {
    if let Some(module) = requested_module {
        validate_module(module)?;
        if let Some(required) = module_read_scope(module) {
            principal.require_scope(required)?;
        }
    }

    let mut sections = BTreeMap::<String, Value>::new();
    for descriptor in REGISTRY {
        if descriptor.ownership != EntityOwnership::UserOwned {
            continue;
        }
        let Some(module) = module_for_entity(descriptor.entity_type) else {
            continue;
        };
        if requested_module.is_some_and(|requested| requested != module) {
            continue;
        }
        let Some(required) = module_read_scope(module) else {
            continue;
        };
        if !principal.scopes.contains(required) {
            continue;
        }
        let entities = state
            .store
            .list_entities(&principal.user_id, descriptor.entity_type)
            .await?;
        if entities.is_empty() {
            continue;
        }
        let value = serde_json::to_value(entities).map_err(|error| {
            api_error(
                ErrorCode::InternalError,
                format!("failed to serialize privacy export: {error}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let section = sections
            .entry(module.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let (Value::Array(target), Value::Array(mut items)) = (section, value) {
            target.append(&mut items);
        }
    }

    let modules: Vec<&str> = if let Some(module) = requested_module {
        vec![module]
    } else {
        SUPPORTED_MODULES
            .iter()
            .copied()
            .filter(|module| {
                module_read_scope(module)
                    .is_some_and(|required| principal.scopes.contains(required))
            })
            .collect()
    };

    for module in modules {
        if module == "account" {
            sections.insert(
                "account".to_owned(),
                safe_account_profile(state, principal).await?,
            );
        }
        if let Some(value) = database_section(state, principal, module).await? {
            sections.insert(module.to_owned(), value);
        }
    }

    Ok(json!({
        "format": "lifetrace-privacy-export-v1",
        "exportedAt": chrono::Utc::now(),
        "userId": principal.user_id.as_str(),
        "requestedModule": requested_module,
        "sections": sections
    }))
}

async fn export_all(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_bearer_or_web_session(&state, &headers).await?;
    principal.require_scope("account:read")?;
    build_export(&state, &principal, None).await.map(Json)
}

async fn export_module(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(module): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_bearer_or_web_session(&state, &headers).await?;
    build_export(&state, &principal, Some(module.as_str()))
        .await
        .map(Json)
}

async fn policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = authenticate_bearer_or_web_session(&state, &headers).await?;
    principal.require_scope("account:read")?;
    Ok(Json(json!({
        "policyVersion": 1,
        "onlinePrimaryData": "retained while the account is active; deleted by the account deletion workflow",
        "sessionsAndTokens": "revoked or deleted when the owning account is deleted",
        "mailRawContent": "retained while mail aggregation is enabled and deleted with the owning account",
        "importFiles": "raw import uploads are device-local unless a domain explicitly opts into cloud storage",
        "fileObjects": "the current cloud service stores metadata only; any non-null external storage reference blocks account deletion until an object cleanup provider is configured",
        "backupDeletion": "logical deletion is immediate; encrypted backup copies age out under the deployment backup-retention window and must be re-deleted if restored",
        "diagnosticLogs": "authentication secrets must be redacted before structured diagnostic metadata is written",
        "agentHistory": "agent sessions, messages, runs and tool-call audit records are retained with the account, exported only within the current app/scope partition, and deleted with the owning account",
        "environment": state.config.environment
    })))
}

async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let principal = write_principal(&state, &headers).await?;
    principal.require_scope("account:write")?;

    let user_id = database_user_id(&principal).ok_or_else(|| {
        api_error(
            ErrorCode::InvalidRequest,
            "account id is not a persistent cloud UUID",
            StatusCode::BAD_REQUEST,
        )
    })?;

    // The current server has no object-byte store. If a mail attachment already
    // points at external storage, fail closed instead of claiming the object was
    // erased when no cleanup provider is available.
    let has_external_objects: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM mail_attachments WHERE user_id=$1 AND storage_ref IS NOT NULL)",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;
    if has_external_objects {
        return Err(api_error(
            ErrorCode::TemporarilyUnavailable,
            "external file objects must be deleted before account deletion can complete",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }

    let mut tx = state.pool.begin().await.map_err(db_error)?;
    sqlx::query(
        "UPDATE auth_sessions SET status='revoked',revoked_at=CURRENT_TIMESTAMP,revoked_reason='account_deleted' \
         WHERE user_id=$1 AND status='active'",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;

    let deleted = sqlx::query("DELETE FROM cloud_users WHERE id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    if deleted.rows_affected() != 1 {
        return Err(api_error(
            ErrorCode::InvalidRequest,
            "account no longer exists",
            StatusCode::NOT_FOUND,
        ));
    }
    tx.commit().await.map_err(db_error)?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie(&state.config));
    Ok(response)
}
