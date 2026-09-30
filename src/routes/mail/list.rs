use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
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
    Router::<AppState>::new().route("/api/v1/mail/messages", get(list_messages))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageListQuery {
    account_id: Option<Uuid>,
    folder_id: Option<Uuid>,
    role: Option<String>,
    q: Option<String>,
    unread_only: Option<bool>,
    starred_only: Option<bool>,
    category_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct MailMessageSummary {
    id: Uuid,
    account_id: Uuid,
    folder_id: Uuid,
    thread_id: Uuid,
    subject: String,
    from_json: Value,
    to_json: Value,
    sent_at: Option<DateTime<Utc>>,
    received_at: DateTime<Utc>,
    is_read: bool,
    is_archived: bool,
    is_starred: bool,
    snippet: Option<String>,
    has_attachments: bool,
    category_ids_json: Value,
}

fn normalize_role(value: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_ascii_lowercase();
    if matches!(
        value.as_str(),
        "inbox" | "sent" | "drafts" | "trash" | "spam" | "archive" | "other"
    ) {
        Ok(Some(value))
    } else {
        Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid mailbox role",
            StatusCode::BAD_REQUEST,
        ))
    }
}

async fn list_messages(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<MessageListQuery>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;

    let user_id = Uuid::parse_str(principal.user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid authenticated user id",
            StatusCode::BAD_REQUEST,
        )
    })?;
    let role = normalize_role(query.role)?;
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);

    let primary = sqlx::query_as::<_, MailMessageSummary>(
        r#"
        SELECT m.id,m.account_id,m.folder_id,m.thread_id,m.subject,m.from_json,m.to_json,
               m.sent_at,m.received_at,m.is_read,m.is_archived,
               CASE WHEN lower(m.flags_json) LIKE '%flagged%' THEN 1 ELSE 0 END AS is_starred,
               m.snippet,m.has_attachments,
               COALESCE((
                   SELECT json_group_array(
                       CASE
                           WHEN typeof(mc.category_id)='blob' AND length(mc.category_id)=16 THEN
                               lower(
                                   substr(hex(mc.category_id),1,8) || '-' ||
                                   substr(hex(mc.category_id),9,4) || '-' ||
                                   substr(hex(mc.category_id),13,4) || '-' ||
                                   substr(hex(mc.category_id),17,4) || '-' ||
                                   substr(hex(mc.category_id),21,12)
                               )
                           ELSE CAST(mc.category_id AS TEXT)
                       END
                   )
                   FROM mail_message_categories mc
                   WHERE mc.user_id=m.user_id
                     AND mc.account_id=m.account_id
                     AND mc.content_hash=m.content_hash
               ), '[]') AS category_ids_json
        FROM mail_messages m
        JOIN mail_folders f ON f.id=m.folder_id
        WHERE m.user_id=$1
          AND ($2 IS NULL OR m.account_id=$2)
          AND ($3 IS NULL OR m.folder_id=$3)
          AND (
                $3 IS NOT NULL
                OR $4 IS NOT NULL
                OR $7=1
                OR $8 IS NOT NULL
                OR (f.normalized_role='inbox' AND m.is_archived=0)
              )
          AND (
                $4 IS NULL
                OR f.normalized_role=$4
                OR ($4='archive' AND m.is_archived=1)
              )
          AND m.received_at >= datetime('now','-30 days')
          AND ($5 IS NULL
               OR lower(m.subject) LIKE '%' || lower($5) || '%'
               OR lower(coalesce(m.snippet,'')) LIKE '%' || lower($5) || '%'
               OR lower(coalesce(m.body_text,'')) LIKE '%' || lower($5) || '%'
               OR lower(m.from_json) LIKE '%' || lower($5) || '%'
               OR lower(m.to_json) LIKE '%' || lower($5) || '%')
          AND ($6 IS NULL OR ($6=1 AND m.is_read=0) OR $6=0)
          AND ($7 IS NULL OR $7=0 OR lower(m.flags_json) LIKE '%flagged%')
          AND (
                $8 IS NULL
                OR (
                    f.normalized_role NOT IN ('trash','spam')
                    AND EXISTS (
                        SELECT 1
                        FROM mail_message_categories mc
                        WHERE mc.user_id=m.user_id
                          AND mc.account_id=m.account_id
                          AND mc.content_hash=m.content_hash
                          AND mc.category_id=$8
                    )
                )
              )
        ORDER BY m.received_at DESC
        LIMIT $9 OFFSET $10
        "#,
    )
    .bind(user_id)
    .bind(query.account_id)
    .bind(query.folder_id)
    .bind(role.as_deref())
    .bind(q)
    .bind(query.unread_only)
    .bind(query.starred_only)
    .bind(query.category_id)
    .bind(limit + 1)
    .bind(offset)
    .fetch_all(&state.pool)
    .await;

    let mut items = match primary {
        Ok(items) => items,
        Err(error) => {
            tracing::warn!(
                error = %error,
                user_id = %user_id,
                category_filter = query.category_id.is_some(),
                "mail message list query failed"
            );

            let error_text = error.to_string().to_ascii_lowercase();
            let category_metadata_unavailable = error_text.contains("mail_message_categories")
                || error_text.contains("json_group_array")
                || error_text.contains("json cannot hold blob")
                || error_text.contains("category_ids_json");

            if query.category_id.is_some() || !category_metadata_unavailable {
                return Err(ApiError::new(
                    ErrorCode::TemporarilyUnavailable,
                    "mail storage operation failed",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }

            tracing::warn!(
                user_id = %user_id,
                "mail category storage unavailable; serving message list without category metadata"
            );

            sqlx::query_as::<_, MailMessageSummary>(
                r#"
                SELECT m.id,m.account_id,m.folder_id,m.thread_id,m.subject,m.from_json,m.to_json,
                       m.sent_at,m.received_at,m.is_read,m.is_archived,
                       CASE WHEN lower(m.flags_json) LIKE '%flagged%' THEN 1 ELSE 0 END AS is_starred,
                       m.snippet,m.has_attachments,
                       '[]' AS category_ids_json
                FROM mail_messages m
                JOIN mail_folders f ON f.id=m.folder_id
                WHERE m.user_id=$1
                  AND ($2 IS NULL OR m.account_id=$2)
                  AND ($3 IS NULL OR m.folder_id=$3)
                  AND (
                        $3 IS NOT NULL
                        OR $4 IS NOT NULL
                        OR $7=1
                        OR (f.normalized_role='inbox' AND m.is_archived=0)
                      )
                  AND (
                        $4 IS NULL
                        OR f.normalized_role=$4
                        OR ($4='archive' AND m.is_archived=1)
                      )
                  AND m.received_at >= datetime('now','-30 days')
                  AND ($5 IS NULL
                       OR lower(m.subject) LIKE '%' || lower($5) || '%'
                       OR lower(coalesce(m.snippet,'')) LIKE '%' || lower($5) || '%'
                       OR lower(coalesce(m.body_text,'')) LIKE '%' || lower($5) || '%'
                       OR lower(m.from_json) LIKE '%' || lower($5) || '%'
                       OR lower(m.to_json) LIKE '%' || lower($5) || '%')
                  AND ($6 IS NULL OR ($6=1 AND m.is_read=0) OR $6=0)
                  AND ($7 IS NULL OR $7=0 OR lower(m.flags_json) LIKE '%flagged%')
                ORDER BY m.received_at DESC
                LIMIT $8 OFFSET $9
                "#,
            )
            .bind(user_id)
            .bind(query.account_id)
            .bind(query.folder_id)
            .bind(role.as_deref())
            .bind(q)
            .bind(query.unread_only)
            .bind(query.starred_only)
            .bind(limit + 1)
            .bind(offset)
            .fetch_all(&state.pool)
            .await
            .map_err(|fallback_error| {
                tracing::warn!(
                    error = %fallback_error,
                    user_id = %user_id,
                    "mail message list fallback query failed"
                );
                ApiError::new(
                    ErrorCode::TemporarilyUnavailable,
                    "mail storage operation failed",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?
        }
    };

    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }

    Ok(Json(json!({
        "items": items,
        "hasMore": has_more,
        "nextOffset": offset + items.len() as i64
    })))
}


#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn category_uuid_blob_is_safe_for_json_aggregation() {
        let pool = sqlx::SqlitePool::connect(":memory:").await.expect("sqlite");
        sqlx::query("CREATE TABLE t (category_id TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("table");

        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        sqlx::query("INSERT INTO t(category_id) VALUES ($1)")
            .bind(id)
            .execute(&pool)
            .await
            .expect("insert");

        let row = sqlx::query(
            r#"
            SELECT json_group_array(
                CASE
                    WHEN typeof(category_id)='blob' AND length(category_id)=16 THEN
                        lower(
                            substr(hex(category_id),1,8) || '-' ||
                            substr(hex(category_id),9,4) || '-' ||
                            substr(hex(category_id),13,4) || '-' ||
                            substr(hex(category_id),17,4) || '-' ||
                            substr(hex(category_id),21,12)
                        )
                    ELSE CAST(category_id AS TEXT)
                END
            ) AS ids
            FROM t
            "#,
        )
        .fetch_one(&pool)
        .await
        .expect("aggregate");

        let ids: String = row.try_get("ids").expect("ids");
        assert_eq!(ids, r#"["550e8400-e29b-41d4-a716-446655440000"]"#);
    }
}
