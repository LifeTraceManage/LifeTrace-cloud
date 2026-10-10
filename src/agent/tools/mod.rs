use chrono::{DateTime, Offset, Utc};
use chrono_tz::Tz;
use lifetrace_contracts::registry::{EntityOwnership, REGISTRY};
use rig::tool::{MissingToolContext, Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{QueryBuilder, Row, Sqlite};
use uuid::Uuid;

use crate::agent::context::AgentInvocationContext;
use crate::auth::scope;

const MAX_TOOL_RESULT_CHARS: usize = 48_000;

#[derive(Debug, thiserror::Error)]
pub enum AgentToolError {
    #[error("missing agent invocation context: {0}")]
    Context(#[from] MissingToolContext),
    #[error("tool storage operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("tool permission denied: {0}")]
    Permission(String),
    #[error("invalid tool input: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverviewArgs {}
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTimeArgs {
    #[serde(default)]
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecordsArgs {
    #[serde(default)]
    pub entity_types: Option<Vec<String>>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMailArgs {
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub unread_only: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
}

pub struct CurrentTimeTool;
pub struct LifeTraceOverviewTool;
pub struct SearchRecordsTool;
pub struct SearchMailTool;
pub struct ListMailAccountsTool;
pub struct ListPendingApprovalsTool;

impl Tool for CurrentTimeTool {
    const NAME: &'static str = "lifetrace_get_current_time";
    type Args = CurrentTimeArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "获取真实的当前日期和时间。涉及“现在、今天、明天、刚才、今晚、当前几点、距截止还有多久”等相对时间时优先调用，不要向用户询问当前时间。timeZone 使用 IANA 时区；优先使用页面上下文中的 timeZone。只读，无需审批。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "timeZone":{
                    "type":"string",
                    "description":"可选 IANA 时区，例如 Asia/Shanghai。优先使用当前 pageContext.timeZone；省略时返回 UTC。"
                }
            },
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = current_time_value(Utc::now(), args.time_zone.as_deref());
        finish_audited(&ctx, call_id, result).await
    }
}

impl Tool for LifeTraceOverviewTool {
    const NAME: &'static str = "lifetrace_overview";
    type Args = OverviewArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "读取当前用户 LifeTrace 各业务实体的数量概览。适合在不知道该先查哪类记录时使用。只读。"
            .to_owned()
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = load_overview(&ctx).await;
        finish_audited(&ctx, call_id, result).await
    }
}

impl Tool for SearchRecordsTool {
    const NAME: &'static str = "lifetrace_search_records";
    type Args = SearchRecordsArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "按业务实体类型和关键词搜索当前用户已经同步到 LifeTrace 的记录，例如任务、日程、笔记、习惯、复盘、训练和账单。只返回当前授权范围内的数据，只读。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "entityTypes":{
                    "type":"array",
                    "items":{"type":"string"},
                    "description":"可选。LifeTrace entityType，例如 execution.task、note.note"
                },
                "entityId":{"type":"string","description":"可选。精确读取一个当前用户可访问的 Sync 实体；来自 selectedEntity 时优先使用它"},
                "query":{"type":"string","description":"可选。对 JSON 记录做不区分大小写的关键词匹配"},
                "limit":{"type":"integer","minimum":1,"maximum":20,"default":10}
            },
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = search_records(&ctx, args).await;
        finish_audited(&ctx, call_id, result).await
    }
}

impl Tool for SearchMailTool {
    const NAME: &'static str = "lifetrace_search_mail";
    type Args = SearchMailArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "搜索当前用户近 30 天已同步邮件，并返回主题、发收件人、Reply-To、正文预览和邮件 ID。回复邮件前必须先用它定位准确邮件；只读。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "messageId":{"type":"string","description":"可选。精确读取一封当前用户的邮件；当前页面提供 selectedEntity.entityId 时优先使用"},
                "query":{"type":"string","description":"可选。匹配主题、摘要、正文或发件人"},
                "unreadOnly":{"type":"boolean","default":false},
                "limit":{"type":"integer","minimum":1,"maximum":20,"default":10}
            },
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = search_mail(&ctx, args).await;
        finish_audited(&ctx, call_id, result).await
    }
}

impl Tool for ListMailAccountsTool {
    const NAME: &'static str = "lifetrace_list_mail_accounts";
    type Args = OverviewArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "读取当前用户可用于发信的邮箱账号与发件身份。起草新邮件前，用它确认 accountId / identityId；只读。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = list_mail_accounts(&ctx).await;
        finish_audited(&ctx, call_id, result).await
    }
}

impl Tool for ListPendingApprovalsTool {
    const NAME: &'static str = "lifetrace_list_pending_approvals";
    type Args = OverviewArgs;
    type Output = Value;
    type Error = AgentToolError;

    fn description(&self) -> String {
        "列出当前 Agent 会话仍可审批的 pending 写操作。用户要求修改、纠正或替换尚未批准的提案时，必须先调用此工具确认旧 approvalId。只读。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        let call_id = audit_start(&ctx, Self::NAME, &args).await?;
        let result = list_pending_approvals(&ctx).await;
        finish_audited(&ctx, call_id, result).await
    }
}

fn current_time_value(
    now: DateTime<Utc>,
    time_zone: Option<&str>,
) -> Result<Value, AgentToolError> {
    let time_zone = time_zone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("UTC");
    let tz: Tz = time_zone
        .parse()
        .map_err(|_| AgentToolError::Invalid(format!("unsupported IANA time zone: {time_zone}")))?;
    let local = now.with_timezone(&tz);
    let offset_seconds = local.offset().fix().local_minus_utc();

    Ok(json!({
        "timeZone": time_zone,
        "localIso": local.to_rfc3339(),
        "localDate": local.format("%Y-%m-%d").to_string(),
        "localTime": local.format("%H:%M:%S").to_string(),
        "weekday": local.format("%A").to_string(),
        "utcOffsetSeconds": offset_seconds,
        "utcIso": now.to_rfc3339(),
        "unixTimestamp": now.timestamp()
    }))
}

pub async fn list_pending_approvals(ctx: &AgentInvocationContext) -> Result<Value, AgentToolError> {
    let rows = sqlx::query(
        "SELECT id,action_name,action_json,requested_at,expires_at \
         FROM agent_approvals \
         WHERE user_id=$1 AND session_id=$2 AND status='pending' \
           AND (expires_at IS NULL OR expires_at>CURRENT_TIMESTAMP) \
         ORDER BY requested_at DESC,rowid DESC LIMIT 20",
    )
    .bind(ctx.user_id)
    .bind(ctx.session_id)
    .fetch_all(&ctx.pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(json!({
            "approvalId": row.try_get::<Uuid,_>("id")?,
            "actionName": row.try_get::<String,_>("action_name")?,
            "action": compact_value(row.try_get::<Value,_>("action_json")?),
            "requestedAt": row.try_get::<DateTime<Utc>,_>("requested_at")?,
            "expiresAt": row.try_get::<Option<DateTime<Utc>>,_>("expires_at")?
        }));
    }
    Ok(json!({"items":items,"count":items.len()}))
}

pub async fn load_overview(ctx: &AgentInvocationContext) -> Result<Value, AgentToolError> {
    let rows = sqlx::query(
        "SELECT entity_type,COUNT(*) AS item_count FROM sync_entities \
         WHERE user_id=$1 AND is_deleted=0 GROUP BY entity_type ORDER BY entity_type",
    )
    .bind(ctx.user_id)
    .fetch_all(&ctx.pool)
    .await?;

    let mut counts = serde_json::Map::new();
    for row in rows {
        let entity_type: String = row.try_get("entity_type")?;
        if can_read_entity(ctx, &entity_type) {
            let count: i64 = row.try_get("item_count")?;
            counts.insert(entity_type, json!(count));
        }
    }

    let unread_mail = if ctx.scopes.contains("mail:read") {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mail_messages WHERE user_id=$1 AND is_read=0 \
             AND received_at >= datetime('now','-30 days')",
        )
        .bind(ctx.user_id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap_or(0)
    } else {
        0
    };

    Ok(json!({
        "entityCounts": counts,
        "unreadMailLast30Days": unread_mail
    }))
}

async fn search_records(
    ctx: &AgentInvocationContext,
    args: SearchRecordsArgs,
) -> Result<Value, AgentToolError> {
    let allowed = allowed_entity_types(ctx);
    let selected = match args.entity_types {
        Some(requested) if !requested.is_empty() => {
            let mut selected = Vec::new();
            for entity_type in requested {
                if !allowed
                    .iter()
                    .any(|allowed_type| allowed_type == &entity_type)
                {
                    return Err(AgentToolError::Permission(entity_type));
                }
                if !selected.contains(&entity_type) {
                    selected.push(entity_type);
                }
            }
            selected
        }
        _ => allowed,
    };

    if selected.is_empty() {
        return Ok(json!({"items":[],"reason":"no readable entity types"}));
    }

    let limit = args.limit.unwrap_or(10).clamp(1, 20);
    let entity_id = args
        .entity_id
        .map(|value| value.trim().chars().take(240).collect::<String>())
        .filter(|value| !value.is_empty());
    let query = args
        .query
        .map(|value| value.trim().chars().take(120).collect::<String>())
        .filter(|value| !value.is_empty());

    let mut builder = QueryBuilder::<Sqlite>::new(
        "SELECT entity_type,entity_id,payload,server_modified_at FROM sync_entities \
         WHERE user_id=",
    );
    builder.push_bind(ctx.user_id);
    builder.push(" AND is_deleted=0 AND entity_type IN (");
    {
        let mut separated = builder.separated(", ");
        for entity_type in &selected {
            separated.push_bind(entity_type);
        }
    }
    builder.push(")");
    if let Some(entity_id) = &entity_id {
        builder.push(" AND entity_id=");
        builder.push_bind(entity_id);
    }
    if let Some(query) = &query {
        builder.push(" AND lower(CAST(payload AS TEXT)) LIKE ");
        builder.push_bind(format!("%{}%", query.to_ascii_lowercase()));
    }
    builder.push(" ORDER BY server_modified_at DESC LIMIT ");
    builder.push_bind(limit);

    let rows = builder.build().fetch_all(&ctx.pool).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let payload: Value = row.try_get("payload")?;
        items.push(json!({
            "entityType": row.try_get::<String,_>("entity_type")?,
            "entityId": row.try_get::<String,_>("entity_id")?,
            "serverModifiedAt": row.try_get::<DateTime<Utc>,_>("server_modified_at")?,
            "payload": compact_value(payload)
        }));
    }
    Ok(json!({"items":items,"count":items.len()}))
}

async fn search_mail(
    ctx: &AgentInvocationContext,
    args: SearchMailArgs,
) -> Result<Value, AgentToolError> {
    if !ctx.scopes.contains("mail:read") {
        return Err(AgentToolError::Permission("mail:read".to_owned()));
    }
    let message_id = args
        .message_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Uuid::parse_str(value)
                .map_err(|_| AgentToolError::Invalid("messageId must be a UUID".to_owned()))
        })
        .transpose()?;
    let query = args
        .query
        .unwrap_or_default()
        .trim()
        .chars()
        .take(120)
        .collect::<String>();
    let pattern = format!("%{}%", query.to_ascii_lowercase());
    let unread_only = args.unread_only.unwrap_or(false);
    let limit = args.limit.unwrap_or(10).clamp(1, 20);

    let rows = if let Some(message_id) = message_id {
        sqlx::query(
            "SELECT id,account_id,subject,from_json,to_json,cc_json,reply_to_json,received_at,is_read,snippet,body_text \
             FROM mail_messages WHERE user_id=$1 AND id=$2 LIMIT 1",
        )
        .bind(ctx.user_id)
        .bind(message_id)
        .fetch_all(&ctx.pool)
        .await?
    } else {
        sqlx::query(
            "SELECT id,account_id,subject,from_json,to_json,cc_json,reply_to_json,received_at,is_read,snippet,body_text \
             FROM mail_messages WHERE user_id=$1 AND received_at >= datetime('now','-30 days') \
             AND ($2='' OR lower(subject) LIKE $3 OR lower(coalesce(snippet,'')) LIKE $3 \
                  OR lower(coalesce(body_text,'')) LIKE $3 OR lower(CAST(from_json AS TEXT)) LIKE $3) \
             AND ($4=0 OR is_read=0) ORDER BY received_at DESC LIMIT $5",
        )
        .bind(ctx.user_id)
        .bind(&query)
        .bind(pattern)
        .bind(unread_only)
        .bind(limit)
        .fetch_all(&ctx.pool)
        .await?
    };

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let body = row
            .try_get::<Option<String>, _>("body_text")?
            .map(|value| value.chars().take(4_000).collect::<String>());
        items.push(json!({
            "id": row.try_get::<Uuid,_>("id")?,
            "accountId": row.try_get::<Uuid,_>("account_id")?,
            "subject": row.try_get::<String,_>("subject")?,
            "from": row.try_get::<Value,_>("from_json")?,
            "to": row.try_get::<Value,_>("to_json")?,
            "cc": row.try_get::<Value,_>("cc_json")?,
            "replyTo": row.try_get::<Value,_>("reply_to_json")?,
            "receivedAt": row.try_get::<DateTime<Utc>,_>("received_at")?,
            "isRead": row.try_get::<bool,_>("is_read")?,
            "snippet": row.try_get::<Option<String>,_>("snippet")?,
            "bodyText": body
        }));
    }
    Ok(json!({"items":items,"count":items.len()}))
}

async fn list_mail_accounts(ctx: &AgentInvocationContext) -> Result<Value, AgentToolError> {
    if !ctx.scopes.contains("mail:read") {
        return Err(AgentToolError::Permission("mail:read".to_owned()));
    }

    let accounts = sqlx::query(
        "SELECT id,email_address,display_name,provider,status FROM mail_accounts \
         WHERE user_id=$1 AND deleted_at IS NULL ORDER BY created_at ASC",
    )
    .bind(ctx.user_id)
    .fetch_all(&ctx.pool)
    .await?;

    let identities = sqlx::query(
        "SELECT id,account_id,email_address,display_name,is_default FROM mail_identities \
         WHERE user_id=$1 AND deleted_at IS NULL ORDER BY is_default DESC,created_at ASC",
    )
    .bind(ctx.user_id)
    .fetch_all(&ctx.pool)
    .await?;

    let mut items = Vec::with_capacity(accounts.len());
    for account in accounts {
        let account_id = account.try_get::<Uuid, _>("id")?;
        let account_id_for_match = account_id;
        let identity_items = identities
            .iter()
            .filter_map(|row| {
                let identity_account_id = row.try_get::<Uuid, _>("account_id").ok()?;
                if identity_account_id != account_id_for_match {
                    return None;
                }
                Some(json!({
                    "id": row.try_get::<Uuid,_>("id").ok()?,
                    "email": row.try_get::<String,_>("email_address").ok()?,
                    "displayName": row.try_get::<Option<String>,_>("display_name").ok()?,
                    "isDefault": row.try_get::<bool,_>("is_default").ok()?
                }))
            })
            .collect::<Vec<_>>();
        items.push(json!({
            "accountId": account_id,
            "email": account.try_get::<String,_>("email_address")?,
            "displayName": account.try_get::<Option<String>,_>("display_name")?,
            "provider": account.try_get::<String,_>("provider")?,
            "status": account.try_get::<String,_>("status")?,
            "identities": identity_items
        }));
    }
    Ok(json!({"items":items,"count":items.len()}))
}

fn allowed_entity_types(ctx: &AgentInvocationContext) -> Vec<String> {
    REGISTRY
        .iter()
        .filter(|descriptor| descriptor.ownership == EntityOwnership::UserOwned)
        .filter_map(|descriptor| {
            let required = scope::required_entity_scope(descriptor.entity_type, false)?;
            ctx.scopes
                .contains(required)
                .then(|| descriptor.entity_type.to_owned())
        })
        .collect()
}

fn can_read_entity(ctx: &AgentInvocationContext, entity_type: &str) -> bool {
    scope::required_entity_scope(entity_type, false)
        .is_some_and(|required| ctx.scopes.contains(required))
}

fn compact_value(value: Value) -> Value {
    let encoded = serde_json::to_string(&value).unwrap_or_default();
    if encoded.chars().count() <= 8_000 {
        value
    } else {
        json!({
            "truncated": true,
            "preview": encoded.chars().take(8_000).collect::<String>()
        })
    }
}

async fn audit_start<T: Serialize>(
    ctx: &AgentInvocationContext,
    tool_name: &str,
    args: &T,
) -> Result<Uuid, AgentToolError> {
    let id = Uuid::new_v4();
    let arguments = serde_json::to_string(args).unwrap_or_else(|_| "{}".to_owned());
    sqlx::query(
        "INSERT INTO agent_tool_calls \
         (id,run_id,session_id,user_id,tool_name,arguments_json,status,requires_approval) \
         VALUES ($1,$2,$3,$4,$5,$6,'running',0)",
    )
    .bind(id)
    .bind(ctx.run_id)
    .bind(ctx.session_id)
    .bind(ctx.user_id)
    .bind(tool_name)
    .bind(arguments)
    .execute(&ctx.pool)
    .await?;
    Ok(id)
}

pub async fn fail_open_tool_calls(
    ctx: &AgentInvocationContext,
    message: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE agent_tool_calls SET status='failed',error_message=$3,finished_at=CURRENT_TIMESTAMP \
         WHERE run_id=$1 AND user_id=$2 AND status='running'",
    )
    .bind(ctx.run_id)
    .bind(ctx.user_id)
    .bind(message.chars().take(500).collect::<String>())
    .execute(&ctx.pool)
    .await?;
    Ok(result.rows_affected())
}

async fn finish_audited(
    ctx: &AgentInvocationContext,
    call_id: Uuid,
    result: Result<Value, AgentToolError>,
) -> Result<Value, AgentToolError> {
    match result {
        Ok(value) => {
            let encoded = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_owned());
            let encoded: String = encoded.chars().take(MAX_TOOL_RESULT_CHARS).collect();
            sqlx::query(
                "UPDATE agent_tool_calls SET status='completed',result_json=$3,finished_at=CURRENT_TIMESTAMP \
                 WHERE id=$1 AND user_id=$2",
            )
            .bind(call_id)
            .bind(ctx.user_id)
            .bind(encoded)
            .execute(&ctx.pool)
            .await?;
            tracing::info!(
                run_id = %ctx.run_id,
                session_id = %ctx.session_id,
                tool_call_id = %call_id,
                "agent tool completed"
            );
            Ok(value)
        }
        Err(error) => {
            let message: String = error.to_string().chars().take(500).collect();
            let _ = sqlx::query(
                "UPDATE agent_tool_calls SET status='failed',error_message=$3,finished_at=CURRENT_TIMESTAMP \
                 WHERE id=$1 AND user_id=$2",
            )
            .bind(call_id)
            .bind(ctx.user_id)
            .bind(message)
            .execute(&ctx.pool)
            .await;
            tracing::warn!(
                run_id = %ctx.run_id,
                session_id = %ctx.session_id,
                tool_call_id = %call_id,
                error = %error,
                "agent tool failed"
            );
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_value_bounds_large_records() {
        let value = json!({"body":"x".repeat(10_000)});
        assert_eq!(compact_value(value)["truncated"], true);
    }

    #[test]
    fn current_time_uses_requested_iana_timezone() {
        let now = DateTime::parse_from_rfc3339("2026-09-30T06:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let value = current_time_value(now, Some("Asia/Shanghai")).unwrap();
        assert_eq!(value["timeZone"], "Asia/Shanghai");
        assert_eq!(value["localDate"], "2026-09-30");
        assert_eq!(value["localTime"], "14:30:00");
        assert_eq!(value["utcOffsetSeconds"], 28_800);
    }

    #[test]
    fn current_time_rejects_unknown_timezone() {
        let error = current_time_value(Utc::now(), Some("Mars/Olympus")).unwrap_err();
        assert!(error.to_string().contains("unsupported IANA time zone"));
    }
}
