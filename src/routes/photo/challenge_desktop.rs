use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use lifetrace_contracts::ErrorCode;
use serde::Serialize;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

use super::challenge::{challenge_owner, load_stats, ChallengeStats};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeEntry {
    id: String,
    file_name: Option<String>,
    captured_at: Option<DateTime<Utc>>,
    score: i64,
    qualified: bool,
    breakdown: Value,
    feedback: String,
    model: String,
    thumbnail_data_url: Option<String>,
    scored_at: DateTime<Utc>,
    staging_pending: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminResponse {
    stats: ChallengeStats,
    entries: Vec<ChallengeEntry>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/photo-challenge/desktop-admin", get(desktop_admin))
}

async fn desktop_admin(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<AdminResponse>, ApiError> {
    if !state.database_enabled {
        return Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "摄影挑战云端模式需要 PostgreSQL",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }

    let owner = challenge_owner(&state).await?;
    if principal.user_id != owner {
        return Err(ApiError::new(
            ErrorCode::AuthScopeDenied,
            "只有摄影挑战所属账号可以查看详情",
            StatusCode::FORBIDDEN,
        ));
    }

    let stats = load_stats(&state, &owner).await?;
    let owner_id = Uuid::parse_str(owner.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "摄影挑战所属账号编号无效",
            StatusCode::BAD_REQUEST,
        )
    })?;

    let rows = sqlx::query(
        "SELECT id,file_name,captured_at,score,qualified,breakdown,feedback,model,thumbnail_data_url,scored_at,staging_id \
         FROM photo_challenge_scores WHERE user_id=$1 ORDER BY scored_at DESC LIMIT 1000",
    )
    .bind(owner_id)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let entries = rows
        .iter()
        .map(|row| {
            Ok(ChallengeEntry {
                id: row
                    .try_get::<Uuid, _>("id")
                    .map_err(database_error)?
                    .to_string(),
                file_name: row.try_get("file_name").map_err(database_error)?,
                captured_at: row.try_get("captured_at").map_err(database_error)?,
                score: i64::from(row.try_get::<i32, _>("score").map_err(database_error)?),
                qualified: row.try_get("qualified").map_err(database_error)?,
                breakdown: row.try_get("breakdown").map_err(database_error)?,
                feedback: row.try_get("feedback").map_err(database_error)?,
                model: row.try_get("model").map_err(database_error)?,
                thumbnail_data_url: row.try_get("thumbnail_data_url").map_err(database_error)?,
                scored_at: row.try_get("scored_at").map_err(database_error)?,
                staging_pending: row
                    .try_get::<Option<Uuid>, _>("staging_id")
                    .map_err(database_error)?
                    .is_some(),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(AdminResponse { stats, entries }))
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("摄影挑战数据库操作失败: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}
