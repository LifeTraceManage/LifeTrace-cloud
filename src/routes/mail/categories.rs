use std::collections::BTreeSet;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use lifetrace_contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(
            "/api/v1/mail/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/api/v1/mail/categories/{id}",
            patch(update_category).delete(delete_category),
        )
        .route(
            "/api/v1/mail/messages/{id}/categories",
            get(list_message_categories).put(set_message_categories),
        )
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct MailCategory {
    id: Uuid,
    name: String,
    message_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryInput {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CategoryAssignmentInput {
    #[serde(default)]
    category_ids: Vec<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct MessageRef {
    account_id: Uuid,
    content_hash: String,
}

fn invalid(message: &'static str) -> ApiError {
    ApiError::new(
        ErrorCode::InvalidRequest,
        message,
        StatusCode::BAD_REQUEST,
    )
}

fn not_found(message: &'static str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::NOT_FOUND)
}

fn database_error() -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        "mail storage operation failed",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

fn user_uuid(principal: &AuthenticatedPrincipal) -> Result<Uuid, ApiError> {
    Uuid::parse_str(principal.user_id.as_str())
        .map_err(|_| invalid("invalid authenticated user id"))
}

fn normalized_name(value: &str) -> Result<String, ApiError> {
    let name = value.trim();
    let count = name.chars().count();
    if count == 0 || count > 40 {
        return Err(invalid("category name must contain 1 to 40 characters"));
    }
    Ok(name.to_owned())
}

async fn message_ref(
    state: &AppState,
    user_id: Uuid,
    message_id: Uuid,
) -> Result<MessageRef, ApiError> {
    sqlx::query_as::<_, MessageRef>(
        "SELECT account_id,content_hash FROM mail_messages WHERE user_id=$1 AND id=$2",
    )
    .bind(user_id)
    .bind(message_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| database_error())?
    .ok_or_else(|| not_found("mail message not found"))
}

async fn list_categories(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let user_id = user_uuid(&principal)?;
    let items = sqlx::query_as::<_, MailCategory>(
        r#"
        SELECT c.id,c.name,
               (
                   SELECT count(*)
                   FROM mail_message_categories mc
                   WHERE mc.user_id=c.user_id
                     AND mc.category_id=c.id
                     AND EXISTS (
                         SELECT 1
                         FROM mail_messages m
                         WHERE m.user_id=mc.user_id
                           AND m.account_id=mc.account_id
                           AND m.content_hash=mc.content_hash
                     )
               ) AS message_count,
               c.created_at,c.updated_at
        FROM mail_categories c
        WHERE c.user_id=$1
        ORDER BY lower(c.name),c.created_at
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| database_error())?;
    Ok(Json(json!({ "items": items })))
}

async fn create_category(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<CategoryInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    principal.require_scope("mail:write")?;
    let user_id = user_uuid(&principal)?;
    let name = normalized_name(&input.name)?;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM mail_categories WHERE user_id=$1 AND lower(name)=lower($2))",
    )
    .bind(user_id)
    .bind(&name)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| database_error())?;
    if exists {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "mail category already exists",
            StatusCode::CONFLICT,
        ));
    }

    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO mail_categories (id,user_id,name) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(user_id)
        .bind(&name)
        .execute(&state.pool)
        .await
        .map_err(|_| database_error())?;

    let item = sqlx::query_as::<_, MailCategory>(
        "SELECT id,name,0 AS message_count,created_at,updated_at FROM mail_categories WHERE user_id=$1 AND id=$2",
    )
    .bind(user_id)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| database_error())?;
    Ok((StatusCode::CREATED, Json(json!(item))))
}

async fn update_category(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<CategoryInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let user_id = user_uuid(&principal)?;
    let name = normalized_name(&input.name)?;

    let duplicate: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM mail_categories WHERE user_id=$1 AND id<>$2 AND lower(name)=lower($3))",
    )
    .bind(user_id)
    .bind(id)
    .bind(&name)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| database_error())?;
    if duplicate {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "mail category already exists",
            StatusCode::CONFLICT,
        ));
    }

    let result = sqlx::query(
        "UPDATE mail_categories SET name=$3,updated_at=CURRENT_TIMESTAMP WHERE user_id=$1 AND id=$2",
    )
    .bind(user_id)
    .bind(id)
    .bind(&name)
    .execute(&state.pool)
    .await
    .map_err(|_| database_error())?;
    if result.rows_affected() == 0 {
        return Err(not_found("mail category not found"));
    }

    let item = sqlx::query_as::<_, MailCategory>(
        r#"
        SELECT c.id,c.name,
               (
                   SELECT count(*)
                   FROM mail_message_categories mc
                   WHERE mc.user_id=c.user_id
                     AND mc.category_id=c.id
                     AND EXISTS (
                         SELECT 1 FROM mail_messages m
                         WHERE m.user_id=mc.user_id
                           AND m.account_id=mc.account_id
                           AND m.content_hash=mc.content_hash
                     )
               ) AS message_count,
               c.created_at,c.updated_at
        FROM mail_categories c
        WHERE c.user_id=$1 AND c.id=$2
        "#,
    )
    .bind(user_id)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| database_error())?;
    Ok(Json(json!(item)))
}

async fn delete_category(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let user_id = user_uuid(&principal)?;
    let result = sqlx::query("DELETE FROM mail_categories WHERE user_id=$1 AND id=$2")
        .bind(user_id)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| database_error())?;
    if result.rows_affected() == 0 {
        return Err(not_found("mail category not found"));
    }
    Ok(Json(json!({ "ok": true })))
}

async fn list_message_categories(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let user_id = user_uuid(&principal)?;
    let message = message_ref(&state, user_id, id).await?;
    let items = sqlx::query_as::<_, MailCategory>(
        r#"
        SELECT c.id,c.name,0 AS message_count,c.created_at,c.updated_at
        FROM mail_categories c
        JOIN mail_message_categories mc
          ON mc.user_id=c.user_id AND mc.category_id=c.id
        WHERE c.user_id=$1
          AND mc.account_id=$2
          AND mc.content_hash=$3
        ORDER BY lower(c.name),c.created_at
        "#,
    )
    .bind(user_id)
    .bind(message.account_id)
    .bind(&message.content_hash)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| database_error())?;
    Ok(Json(json!({ "items": items })))
}

async fn set_message_categories(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<CategoryAssignmentInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let user_id = user_uuid(&principal)?;
    let message = message_ref(&state, user_id, id).await?;
    let category_ids: BTreeSet<Uuid> = input.category_ids.into_iter().collect();

    let mut tx = state.pool.begin().await.map_err(|_| database_error())?;
    for category_id in &category_ids {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM mail_categories WHERE user_id=$1 AND id=$2)",
        )
        .bind(user_id)
        .bind(category_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| database_error())?;
        if !exists {
            return Err(invalid("mail category does not belong to the current user"));
        }
    }

    sqlx::query(
        "DELETE FROM mail_message_categories WHERE user_id=$1 AND account_id=$2 AND content_hash=$3",
    )
    .bind(user_id)
    .bind(message.account_id)
    .bind(&message.content_hash)
    .execute(&mut *tx)
    .await
    .map_err(|_| database_error())?;

    for category_id in &category_ids {
        sqlx::query(
            r#"
            INSERT INTO mail_message_categories
                (user_id,account_id,content_hash,category_id)
            VALUES ($1,$2,$3,$4)
            "#,
        )
        .bind(user_id)
        .bind(message.account_id)
        .bind(&message.content_hash)
        .bind(category_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| database_error())?;
    }
    tx.commit().await.map_err(|_| database_error())?;

    state.mail_realtime.publish_account_updated(
        principal.user_id.as_str(),
        message.account_id,
        0,
        "category",
    );

    Ok(Json(json!({
        "ok": true,
        "categoryIds": category_ids.into_iter().collect::<Vec<_>>()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_name_is_trimmed_and_bounded() {
        assert_eq!(normalized_name("  工作  ").unwrap(), "工作");
        assert!(normalized_name("   ").is_err());
        assert!(normalized_name(&"a".repeat(41)).is_err());
    }
}
