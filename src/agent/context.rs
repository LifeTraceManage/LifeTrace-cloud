use std::collections::BTreeSet;
use std::sync::Arc;

use lifetrace_contracts::UserId;
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentInvocationContext {
    pub pool: SqlitePool,
    pub user_id: Uuid,
    pub scopes: Arc<BTreeSet<String>>,
    pub run_id: Uuid,
    pub session_id: Uuid,
}

pub fn database_user_id(user_id: &UserId) -> Uuid {
    Uuid::parse_str(user_id.as_str()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("https://lifetrace.local/users/{}", user_id.as_str()).as_bytes(),
        )
    })
}

pub async fn ensure_cloud_user(pool: &SqlitePool, user_id: &UserId) -> Result<Uuid, sqlx::Error> {
    let user_id = database_user_id(user_id);
    sqlx::query(
        "INSERT INTO cloud_users (id,status) VALUES ($1,'active') \
         ON CONFLICT(id) DO UPDATE SET updated_at=CURRENT_TIMESTAMP",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(user_id)
}
