use chrono::{DateTime, Utc};
use rig::completion::Message;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::agent::context::AgentAccessPartition;

const HISTORY_LIMIT: i64 = 24;

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub provider: Option<String>,
    pub metadata_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub async fn ensure_session(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    requested: Option<Uuid>,
    prompt: &str,
) -> Result<AgentSession, sqlx::Error> {
    if let Some(session_id) = requested {
        return sqlx::query_as::<_, AgentSession>(
            "SELECT id,title,status,created_at,updated_at,last_message_at \
             FROM agent_sessions WHERE id=$1 AND user_id=$2 AND app_id=$3 AND scopes_json=$4 AND status='active'",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(&access.app_id)
        .bind(&access.scopes_json)
        .fetch_one(pool)
        .await;
    }

    let id = Uuid::new_v4();
    let title = title_from_prompt(prompt);
    sqlx::query_as::<_, AgentSession>(
        "INSERT INTO agent_sessions (id,user_id,app_id,scopes_json,title,status) \
         VALUES ($1,$2,$3,$4,$5,'active') \
         RETURNING id,title,status,created_at,updated_at,last_message_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .bind(title)
    .fetch_one(pool)
    .await
}

pub async fn list_sessions(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    limit: i64,
) -> Result<Vec<AgentSession>, sqlx::Error> {
    sqlx::query_as::<_, AgentSession>(
        "SELECT id,title,status,created_at,updated_at,last_message_at \
         FROM agent_sessions WHERE user_id=$1 AND app_id=$2 AND scopes_json=$3 AND status='active' \
         ORDER BY COALESCE(last_message_at,created_at) DESC LIMIT $4",
    )
    .bind(user_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await
}

pub async fn list_messages(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    session_id: Uuid,
    limit: i64,
) -> Result<Vec<AgentMessage>, sqlx::Error> {
    let mut items = sqlx::query_as::<_, AgentMessage>(
        "SELECT m.id,m.session_id,m.run_id,m.role,m.content,m.provider,m.metadata_json,m.created_at \
         FROM agent_messages m JOIN agent_sessions s ON s.id=m.session_id \
         WHERE m.user_id=$1 AND m.session_id=$2 AND s.user_id=$1 \
           AND s.app_id=$3 AND s.scopes_json=$4 \
         ORDER BY m.created_at DESC,m.rowid DESC LIMIT $5",
    )
    .bind(user_id)
    .bind(session_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    items.reverse();
    Ok(items)
}

pub async fn load_history(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    session_id: Uuid,
) -> Result<Vec<Message>, sqlx::Error> {
    #[derive(FromRow)]
    struct HistoryRow {
        role: String,
        content: String,
    }

    let mut rows = sqlx::query_as::<_, HistoryRow>(
        "SELECT m.role,m.content FROM agent_messages m \
         JOIN agent_sessions s ON s.id=m.session_id \
         WHERE m.user_id=$1 AND m.session_id=$2 AND s.user_id=$1 \
           AND s.app_id=$3 AND s.scopes_json=$4 AND m.role IN ('user','assistant') \
         ORDER BY m.created_at DESC,m.rowid DESC LIMIT $5",
    )
    .bind(user_id)
    .bind(session_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .bind(HISTORY_LIMIT)
    .fetch_all(pool)
    .await?;
    rows.reverse();

    Ok(rows
        .into_iter()
        .filter_map(|row| match row.role.as_str() {
            "user" => Some(Message::user(row.content)),
            "assistant" => Some(Message::assistant(row.content)),
            _ => None,
        })
        .collect())
}

pub async fn start_run(
    pool: &SqlitePool,
    user_id: Uuid,
    session_id: Uuid,
    provider: &str,
    model: Option<&str>,
    prompt: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO agent_runs (id,session_id,user_id,status,provider,model,prompt) \
         VALUES ($1,$2,$3,'running',$4,$5,$6)",
    )
    .bind(id)
    .bind(session_id)
    .bind(user_id)
    .bind(provider)
    .bind(model)
    .bind(prompt)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn insert_message(
    pool: &SqlitePool,
    user_id: Uuid,
    session_id: Uuid,
    run_id: Option<Uuid>,
    role: &str,
    content: &str,
    provider: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO agent_messages (id,session_id,run_id,user_id,role,content,provider) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(id)
    .bind(session_id)
    .bind(run_id)
    .bind(user_id)
    .bind(role)
    .bind(content)
    .bind(provider)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE agent_sessions SET updated_at=CURRENT_TIMESTAMP,last_message_at=CURRENT_TIMESTAMP \
         WHERE id=$1 AND user_id=$2",
    )
    .bind(session_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn complete_run(
    pool: &SqlitePool,
    run_id: Uuid,
    user_id: Uuid,
    provider: &str,
    model: Option<&str>,
    fallback_error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_runs SET status='completed',provider=$3,model=$4,error_message=$5,finished_at=CURRENT_TIMESTAMP \
         WHERE id=$1 AND user_id=$2",
    )
    .bind(run_id)
    .bind(user_id)
    .bind(provider)
    .bind(model)
    .bind(fallback_error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn fail_run(
    pool: &SqlitePool,
    run_id: Uuid,
    user_id: Uuid,
    code: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE agent_runs SET status='failed',error_code=$3,error_message=$4,finished_at=CURRENT_TIMESTAMP \
         WHERE id=$1 AND user_id=$2",
    )
    .bind(run_id)
    .bind(user_id)
    .bind(code)
    .bind(message)
    .execute(pool)
    .await?;
    Ok(())
}

fn title_from_prompt(prompt: &str) -> String {
    let title: String = prompt
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(48)
        .collect();
    if title.is_empty() {
        "新对话".to_owned()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::title_from_prompt;

    #[test]
    fn session_title_is_short_and_non_empty() {
        assert_eq!(title_from_prompt("  帮我 总结 今天  "), "帮我 总结 今天");
        assert!(title_from_prompt(&"a".repeat(100)).chars().count() <= 48);
        assert_eq!(title_from_prompt("   "), "新对话");
    }
}
