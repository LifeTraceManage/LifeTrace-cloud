use chrono::{DateTime, NaiveDate, Utc};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::sync::v1::{
    ChangeOperation, ClientPlatform, PushChangeResultV1, PushRequestV1, SyncChangeV1,
    SyncClientInfo,
};
use lifetrace_contracts::{ChangeId, DeviceId, EntityId, EntityType, RequestId, ServerVersion};
use rig::tool::{MissingToolContext, Tool, ToolContext};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

use crate::agent::context::{AgentAccessPartition, AgentInvocationContext};
use crate::auth::AuthenticatedPrincipal;
use crate::mail::domain::SendMailInput;
use crate::mail::MailService;
use crate::state::AppState;

const APPROVAL_TTL_MINUTES: i64 = 15;

#[derive(Debug, thiserror::Error)]
pub enum ApprovalError {
    #[error("missing agent invocation context: {0}")]
    Context(#[from] MissingToolContext),
    #[error("approval storage operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("approval permission denied: {0}")]
    Permission(String),
    #[error("invalid approval action: {0}")]
    Invalid(String),
    #[error("approval was not found")]
    NotFound,
    #[error("approval has expired")]
    Expired,
    #[error("approval conflict: {0}")]
    Conflict(String),
    #[error("approval execution failed: {0}")]
    Execution(String),
}

#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentApproval {
    pub id: Uuid,
    pub run_id: Uuid,
    pub session_id: Uuid,
    pub tool_call_id: Option<Uuid>,
    pub action_name: String,
    pub action_json: Value,
    pub status: String,
    pub requested_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub superseded_by_approval_id: Option<Uuid>,
    pub cancellation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionOutput {
    pub approval: AgentApproval,
    pub result: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskArgs {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub scheduled_start_at: Option<String>,
    #[serde(default)]
    pub scheduled_end_at: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub leave_unscheduled: bool,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskArgs {
    pub task_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub clear_due_at: bool,
    #[serde(default)]
    pub scheduled_start_at: Option<String>,
    #[serde(default)]
    pub scheduled_end_at: Option<String>,
    #[serde(default)]
    pub clear_schedule: bool,
    #[serde(default)]
    pub estimated_minutes: Option<i64>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCalendarEventArgs {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_all_day: bool,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub end_at: Option<String>,
    #[serde(default)]
    pub start_local_date: Option<String>,
    #[serde(default)]
    pub end_local_date: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectArgs {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectArgs {
    pub project_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub clear_description: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHabitArgs {
    pub name: String,
    #[serde(default)]
    pub activity_type: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub minimum_target: Option<f64>,
    #[serde(default)]
    pub normal_target: Option<f64>,
    #[serde(default)]
    pub target_days: Vec<u8>,
    #[serde(default)]
    pub schedule_type: Option<String>,
    pub start_date: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHabitArgs {
    pub habit_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub minimum_target: Option<f64>,
    #[serde(default)]
    pub clear_minimum_target: bool,
    #[serde(default)]
    pub normal_target: Option<f64>,
    #[serde(default)]
    pub target_days: Option<Vec<u8>>,
    #[serde(default)]
    pub schedule_type: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub clear_description: bool,
    #[serde(default)]
    pub is_archived: Option<bool>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWaitingItemArgs {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub waiting_for: String,
    #[serde(default)]
    pub expected_at: Option<String>,
    #[serde(default)]
    pub follow_up_at: Option<String>,
    #[serde(default)]
    pub source_task_id: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWaitingItemArgs {
    pub waiting_item_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub clear_description: bool,
    #[serde(default)]
    pub waiting_for: Option<String>,
    #[serde(default)]
    pub expected_at: Option<String>,
    #[serde(default)]
    pub clear_expected_at: bool,
    #[serde(default)]
    pub follow_up_at: Option<String>,
    #[serde(default)]
    pub clear_follow_up_at: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub resolution_summary: Option<String>,
    #[serde(default)]
    pub clear_resolution_summary: bool,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReminderArgs {
    pub subject_type: String,
    pub subject_id: String,
    pub trigger_at: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReminderArgs {
    pub reminder_id: String,
    #[serde(default)]
    pub trigger_at: Option<String>,
    #[serde(default)]
    pub snoozed_until: Option<String>,
    #[serde(default)]
    pub clear_snooze: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub clear_title: bool,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub clear_body: bool,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteArgs {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content_markdown: String,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub note_type: Option<String>,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMailArgs {
    pub account_id: String,
    #[serde(default)]
    pub identity_id: Option<String>,
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
    pub subject: String,
    pub body_text: String,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyMailArgs {
    pub message_id: String,
    #[serde(default)]
    pub identity_id: Option<String>,
    pub body_text: String,
    #[serde(default)]
    pub reply_all: bool,
    #[serde(default)]
    pub supersedes_approval_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNoteAction {
    entity_id: String,
    title: Option<String>,
    content_markdown: String,
    folder_id: Option<String>,
    note_type: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendMailAction {
    account_id: Uuid,
    identity_id: Option<Uuid>,
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    subject: String,
    body_text: String,
    source_message_id: Option<Uuid>,
    in_reply_to_header: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskAction {
    entity_id: String,
    title: String,
    description: Option<String>,
    project_id: Option<String>,
    priority: String,
    due_at: Option<String>,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    timezone: String,
    context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTaskAction {
    task_id: String,
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    due_at: Option<String>,
    clear_due_at: bool,
    scheduled_start_at: Option<String>,
    scheduled_end_at: Option<String>,
    #[serde(default)]
    clear_schedule: bool,
    estimated_minutes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCalendarEventAction {
    entity_id: String,
    title: String,
    description: Option<String>,
    is_all_day: bool,
    start_at: Option<String>,
    end_at: Option<String>,
    start_local_date: Option<String>,
    end_local_date: Option<String>,
    timezone: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectAction {
    entity_id: String,
    name: String,
    description: Option<String>,
    color: String,
    icon: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProjectAction {
    project_id: String,
    name: Option<String>,
    description: Option<String>,
    clear_description: bool,
    status: Option<String>,
    color: Option<String>,
    icon: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateHabitAction {
    entity_id: String,
    name: String,
    activity_type: String,
    unit: String,
    minimum_target: Option<f64>,
    normal_target: f64,
    target_days: Vec<u8>,
    schedule_type: String,
    start_date: String,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateHabitAction {
    habit_id: String,
    name: Option<String>,
    minimum_target: Option<f64>,
    clear_minimum_target: bool,
    normal_target: Option<f64>,
    target_days: Option<Vec<u8>>,
    schedule_type: Option<String>,
    start_date: Option<String>,
    description: Option<String>,
    clear_description: bool,
    is_archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateWaitingItemAction {
    entity_id: String,
    title: String,
    description: Option<String>,
    waiting_for: String,
    expected_at: Option<String>,
    follow_up_at: Option<String>,
    source_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateWaitingItemAction {
    waiting_item_id: String,
    title: Option<String>,
    description: Option<String>,
    clear_description: bool,
    waiting_for: Option<String>,
    expected_at: Option<String>,
    clear_expected_at: bool,
    follow_up_at: Option<String>,
    clear_follow_up_at: bool,
    status: Option<String>,
    resolution_summary: Option<String>,
    clear_resolution_summary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateReminderAction {
    entity_id: String,
    subject_type: String,
    subject_id: String,
    trigger_at: String,
    title: Option<String>,
    body: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateReminderAction {
    reminder_id: String,
    trigger_at: Option<String>,
    snoozed_until: Option<String>,
    clear_snooze: bool,
    status: Option<String>,
    title: Option<String>,
    clear_title: bool,
    body: Option<String>,
    clear_body: bool,
}

pub struct ProposeCreateTaskTool;
pub struct ProposeUpdateTaskTool;
pub struct ProposeCreateCalendarEventTool;
pub struct ProposeCreateProjectTool;
pub struct ProposeUpdateProjectTool;
pub struct ProposeCreateHabitTool;
pub struct ProposeUpdateHabitTool;
pub struct ProposeCreateWaitingItemTool;
pub struct ProposeUpdateWaitingItemTool;
pub struct ProposeCreateReminderTool;
pub struct ProposeUpdateReminderTool;
pub struct ProposeCreateNoteTool;
pub struct ProposeSendMailTool;
pub struct ProposeReplyMailTool;

impl Tool for ProposeCreateTaskTool {
    const NAME: &'static str = "lifetrace_propose_create_task";
    type Args = CreateTaskArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace 任务的写操作。带截止时间的任务默认必须同时给出 Planner 执行时间段；只有用户明确要求先收集不排期时才使用 leaveUnscheduled=true。该工具不会直接写入数据，只生成审批请求。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "title":{"type":"string"},
                "description":{"type":"string"},
                "projectId":{"type":"string"},
                "priority":{"type":"string","enum":["low","normal","high","urgent"]},
                "dueAt":{"type":"string","description":"RFC3339 timestamp"},
                "scheduledStartAt":{"type":"string","description":"RFC3339 timestamp"},
                "scheduledEndAt":{"type":"string","description":"RFC3339 timestamp"},
                "timezone":{"type":"string","description":"IANA timezone, for example Asia/Shanghai"},
                "context":{"type":"string"},
                "leaveUnscheduled":{"type":"boolean","default":false,"description":"仅当用户明确要求先收集、不安排 Planner 时间时使用"}
            },
            "required":["title"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let title = bounded_required(&args.title, "title", 300)?;
        let priority = args.priority.unwrap_or_else(|| "normal".to_owned());
        validate_one_of(&priority, "priority", &["low", "normal", "high", "urgent"])?;
        validate_optional_timestamp(args.due_at.as_deref(), "dueAt")?;
        validate_optional_timestamp(args.scheduled_start_at.as_deref(), "scheduledStartAt")?;
        validate_optional_timestamp(args.scheduled_end_at.as_deref(), "scheduledEndAt")?;
        if let (Some(start), Some(end)) = (
            args.scheduled_start_at.as_deref(),
            args.scheduled_end_at.as_deref(),
        ) {
            validate_range(start, end, "scheduledStartAt", "scheduledEndAt")?;
        }
        if args.scheduled_start_at.is_some() ^ args.scheduled_end_at.is_some() {
            return Err(ApprovalError::Invalid(
                "scheduledStartAt and scheduledEndAt must be provided together".to_owned(),
            ));
        }
        if args.due_at.is_some() && args.scheduled_start_at.is_none() && !args.leave_unscheduled {
            return Err(ApprovalError::Invalid(
                "deadline tasks require a Planner schedule; inspect existing tasks/calendar and retry with scheduledStartAt/scheduledEndAt, or set leaveUnscheduled=true only when the user explicitly asked to leave it unplanned".to_owned(),
            ));
        }

        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": title,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "projectId": bounded_optional(args.project_id.as_deref(), 200),
            "priority": priority,
            "dueAt": args.due_at,
            "scheduledStartAt": args.scheduled_start_at,
            "scheduledEndAt": args.scheduled_end_at,
            "timezone": bounded_optional(args.timezone.as_deref(), 100).unwrap_or_else(|| "UTC".to_owned()),
            "context": bounded_optional(args.context.as_deref(), 200)
        });
        propose(
            &ctx,
            Self::NAME,
            "create_task",
            arguments_json,
            action,
            json!({"title": title, "priority": priority}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeUpdateTaskTool {
    const NAME: &'static str = "lifetrace_propose_update_task";
    type Args = UpdateTaskArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出修改已有 LifeTrace 任务的写操作。支持标题、状态、优先级、截止时间和 Planner 执行时间段；不会直接执行，必须由用户显式批准。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "taskId":{"type":"string"},
                "title":{"type":"string"},
                "status":{"type":"string","enum":["todo","in_progress","waiting","done","cancelled"]},
                "priority":{"type":"string","enum":["low","normal","high","urgent"]},
                "dueAt":{"type":"string","description":"RFC3339 timestamp"},
                "clearDueAt":{"type":"boolean","default":false},
                "scheduledStartAt":{"type":"string","description":"RFC3339 Planner start timestamp"},
                "scheduledEndAt":{"type":"string","description":"RFC3339 Planner end timestamp"},
                "clearSchedule":{"type":"boolean","default":false},
                "estimatedMinutes":{"type":"integer","minimum":15,"maximum":720}
            },
            "required":["taskId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let task_id = bounded_required(&args.task_id, "taskId", 200)?;
        let title = match args.title.as_deref() {
            Some(value) => Some(bounded_required(value, "title", 300)?),
            None => None,
        };
        if let Some(status) = args.status.as_deref() {
            validate_one_of(
                status,
                "status",
                &["todo", "in_progress", "waiting", "done", "cancelled"],
            )?;
        }
        if let Some(priority) = args.priority.as_deref() {
            validate_one_of(priority, "priority", &["low", "normal", "high", "urgent"])?;
        }
        validate_optional_timestamp(args.due_at.as_deref(), "dueAt")?;
        validate_optional_timestamp(args.scheduled_start_at.as_deref(), "scheduledStartAt")?;
        validate_optional_timestamp(args.scheduled_end_at.as_deref(), "scheduledEndAt")?;
        if let (Some(start), Some(end)) = (
            args.scheduled_start_at.as_deref(),
            args.scheduled_end_at.as_deref(),
        ) {
            validate_range(start, end, "scheduledStartAt", "scheduledEndAt")?;
        }
        if args.scheduled_start_at.is_some() ^ args.scheduled_end_at.is_some() {
            return Err(ApprovalError::Invalid(
                "scheduledStartAt and scheduledEndAt must be provided together".to_owned(),
            ));
        }
        if args.clear_schedule
            && (args.scheduled_start_at.is_some() || args.scheduled_end_at.is_some())
        {
            return Err(ApprovalError::Invalid(
                "scheduled time fields and clearSchedule cannot be used together".to_owned(),
            ));
        }
        if let Some(estimated_minutes) = args.estimated_minutes {
            if !(15..=720).contains(&estimated_minutes) {
                return Err(ApprovalError::Invalid(
                    "estimatedMinutes must be between 15 and 720".to_owned(),
                ));
            }
        }
        if args.due_at.is_some() && args.clear_due_at {
            return Err(ApprovalError::Invalid(
                "dueAt and clearDueAt cannot be used together".to_owned(),
            ));
        }
        if title.is_none()
            && args.status.is_none()
            && args.priority.is_none()
            && args.due_at.is_none()
            && !args.clear_due_at
            && args.scheduled_start_at.is_none()
            && args.scheduled_end_at.is_none()
            && !args.clear_schedule
            && args.estimated_minutes.is_none()
        {
            return Err(ApprovalError::Invalid(
                "at least one task field must be changed".to_owned(),
            ));
        }

        let action = json!({
            "taskId": task_id,
            "title": title,
            "status": args.status,
            "priority": args.priority,
            "dueAt": args.due_at,
            "clearDueAt": args.clear_due_at,
            "scheduledStartAt": args.scheduled_start_at,
            "scheduledEndAt": args.scheduled_end_at,
            "clearSchedule": args.clear_schedule,
            "estimatedMinutes": args.estimated_minutes
        });
        propose(
            &ctx,
            Self::NAME,
            "update_task",
            arguments_json,
            action,
            json!({"taskId": task_id}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateCalendarEventTool {
    const NAME: &'static str = "lifetrace_propose_create_calendar_event";
    type Args = CreateCalendarEventArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace 日程的写操作。不会直接创建，必须由用户显式批准。日期、时间或时区不明确时先询问用户。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "title":{"type":"string"},
                "description":{"type":"string"},
                "isAllDay":{"type":"boolean","default":false},
                "startAt":{"type":"string","description":"RFC3339 timestamp for timed event"},
                "endAt":{"type":"string","description":"RFC3339 timestamp for timed event"},
                "startLocalDate":{"type":"string","description":"YYYY-MM-DD for all-day event"},
                "endLocalDate":{"type":"string","description":"YYYY-MM-DD for all-day event"},
                "timezone":{"type":"string","description":"IANA timezone"}
            },
            "required":["title"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let title = bounded_required(&args.title, "title", 300)?;
        validate_optional_timestamp(args.start_at.as_deref(), "startAt")?;
        validate_optional_timestamp(args.end_at.as_deref(), "endAt")?;
        validate_optional_date(args.start_local_date.as_deref(), "startLocalDate")?;
        validate_optional_date(args.end_local_date.as_deref(), "endLocalDate")?;

        if args.is_all_day {
            if args.start_local_date.is_none() {
                return Err(ApprovalError::Invalid(
                    "startLocalDate is required for all-day events".to_owned(),
                ));
            }
        } else if args.start_at.is_none() {
            return Err(ApprovalError::Invalid(
                "startAt is required for timed events".to_owned(),
            ));
        }
        if let (Some(start), Some(end)) = (args.start_at.as_deref(), args.end_at.as_deref()) {
            validate_range(start, end, "startAt", "endAt")?;
        }

        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": title,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "isAllDay": args.is_all_day,
            "startAt": args.start_at,
            "endAt": args.end_at,
            "startLocalDate": args.start_local_date,
            "endLocalDate": args.end_local_date,
            "timezone": bounded_optional(args.timezone.as_deref(), 100).unwrap_or_else(|| "UTC".to_owned())
        });
        propose(
            &ctx,
            Self::NAME,
            "create_calendar_event",
            arguments_json,
            action,
            json!({"title": title, "isAllDay": args.is_all_day}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateProjectTool {
    const NAME: &'static str = "lifetrace_propose_create_project";
    type Args = CreateProjectArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace Project 的写操作。不会直接执行，必须由用户显式批准。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "name":{"type":"string"},
                "description":{"type":"string"},
                "color":{"type":"string"},
                "icon":{"type":"string"}
            },
            "required":["name"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["sync:write", "execution:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let name = bounded_required(&args.name, "name", 300)?;
        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "name": name,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "color": bounded_optional(args.color.as_deref(), 100).unwrap_or_else(|| "#49715d".to_owned()),
            "icon": bounded_optional(args.icon.as_deref(), 100).unwrap_or_else(|| "target".to_owned())
        });
        propose(
            &ctx,
            Self::NAME,
            "create_project",
            arguments_json,
            action,
            json!({"name": name}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeUpdateProjectTool {
    const NAME: &'static str = "lifetrace_propose_update_project";
    type Args = UpdateProjectArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出修改已有 LifeTrace Project 的写操作。可修改名称、说明、状态、颜色和图标；不会直接执行。修改前先搜索确认 projectId。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "projectId":{"type":"string"},
                "name":{"type":"string"},
                "description":{"type":"string"},
                "clearDescription":{"type":"boolean","default":false},
                "status":{"type":"string","enum":["active","archived"]},
                "color":{"type":"string"},
                "icon":{"type":"string"}
            },
            "required":["projectId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["sync:write", "execution:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let project_id = bounded_required(&args.project_id, "projectId", 200)?;
        let name = match args.name.as_deref() {
            Some(value) => Some(bounded_required(value, "name", 300)?),
            None => None,
        };
        if args.description.is_some() && args.clear_description {
            return Err(ApprovalError::Invalid(
                "description and clearDescription cannot be used together".to_owned(),
            ));
        }
        if let Some(status) = args.status.as_deref() {
            validate_one_of(status, "status", &["active", "archived"])?;
        }
        if name.is_none()
            && args.description.is_none()
            && !args.clear_description
            && args.status.is_none()
            && args.color.is_none()
            && args.icon.is_none()
        {
            return Err(ApprovalError::Invalid(
                "at least one project field must be changed".to_owned(),
            ));
        }

        let action = json!({
            "projectId": project_id,
            "name": name,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "clearDescription": args.clear_description,
            "status": args.status,
            "color": bounded_optional(args.color.as_deref(), 100),
            "icon": bounded_optional(args.icon.as_deref(), 100)
        });
        propose(
            &ctx,
            Self::NAME,
            "update_project",
            arguments_json,
            action,
            json!({"projectId": project_id}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateHabitTool {
    const NAME: &'static str = "lifetrace_propose_create_habit";
    type Args = CreateHabitArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace 习惯的写操作。支持完成、计数、时长三类习惯；必须提供开始日期，不会直接执行。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "name":{"type":"string"},
                "activityType":{"type":"string","enum":["completion","count","duration"]},
                "unit":{"type":"string"},
                "minimumTarget":{"type":"number","minimum":0},
                "normalTarget":{"type":"number","exclusiveMinimum":0},
                "targetDays":{"type":"array","items":{"type":"integer","minimum":1,"maximum":7}},
                "scheduleType":{"type":"string","enum":["daily","custom"]},
                "startDate":{"type":"string","description":"YYYY-MM-DD"},
                "description":{"type":"string"}
            },
            "required":["name","startDate"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["sync:write", "habits:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let name = bounded_required(&args.name, "name", 300)?;
        let activity_type = args
            .activity_type
            .unwrap_or_else(|| "completion".to_owned());
        validate_one_of(
            &activity_type,
            "activityType",
            &["completion", "count", "duration"],
        )?;
        let schedule_type = args.schedule_type.unwrap_or_else(|| "daily".to_owned());
        validate_one_of(&schedule_type, "scheduleType", &["daily", "custom"])?;
        validate_optional_date(Some(args.start_date.as_str()), "startDate")?;
        let target_days = normalize_target_days(args.target_days)?;
        if schedule_type == "custom" && target_days.is_empty() {
            return Err(ApprovalError::Invalid(
                "targetDays must contain at least one weekday for custom schedules".to_owned(),
            ));
        }
        let target_days = if schedule_type == "daily" {
            Vec::new()
        } else {
            target_days
        };
        let normal_target = args.normal_target.unwrap_or(1.0);
        validate_habit_targets(args.minimum_target, normal_target)?;
        let unit = bounded_optional(args.unit.as_deref(), 100).unwrap_or_else(|| {
            if activity_type == "duration" {
                "分钟".to_owned()
            } else {
                "次".to_owned()
            }
        });

        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "name": name,
            "activityType": activity_type,
            "unit": unit,
            "minimumTarget": args.minimum_target,
            "normalTarget": normal_target,
            "targetDays": target_days,
            "scheduleType": schedule_type,
            "startDate": args.start_date,
            "description": bounded_optional(args.description.as_deref(), 4_000)
        });
        propose(
            &ctx,
            Self::NAME,
            "create_habit",
            arguments_json,
            action,
            json!({"name": name, "scheduleType": schedule_type}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeUpdateHabitTool {
    const NAME: &'static str = "lifetrace_propose_update_habit";
    type Args = UpdateHabitArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出修改已有 LifeTrace 习惯的写操作。支持名称、目标、执行日、开始日期、说明和归档状态；不会直接执行。修改前先搜索确认 habitId。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "habitId":{"type":"string"},
                "name":{"type":"string"},
                "minimumTarget":{"type":"number","minimum":0},
                "clearMinimumTarget":{"type":"boolean","default":false},
                "normalTarget":{"type":"number","exclusiveMinimum":0},
                "targetDays":{"type":"array","items":{"type":"integer","minimum":1,"maximum":7}},
                "scheduleType":{"type":"string","enum":["daily","custom"]},
                "startDate":{"type":"string","description":"YYYY-MM-DD"},
                "description":{"type":"string"},
                "clearDescription":{"type":"boolean","default":false},
                "isArchived":{"type":"boolean"}
            },
            "required":["habitId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["sync:write", "habits:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let habit_id = bounded_required(&args.habit_id, "habitId", 200)?;
        let name = match args.name.as_deref() {
            Some(value) => Some(bounded_required(value, "name", 300)?),
            None => None,
        };
        if args.minimum_target.is_some() && args.clear_minimum_target {
            return Err(ApprovalError::Invalid(
                "minimumTarget and clearMinimumTarget cannot be used together".to_owned(),
            ));
        }
        if args.description.is_some() && args.clear_description {
            return Err(ApprovalError::Invalid(
                "description and clearDescription cannot be used together".to_owned(),
            ));
        }
        if let Some(normal_target) = args.normal_target {
            if !normal_target.is_finite() || normal_target <= 0.0 {
                return Err(ApprovalError::Invalid(
                    "normalTarget must be a finite value greater than zero".to_owned(),
                ));
            }
        }
        if let Some(minimum_target) = args.minimum_target {
            if !minimum_target.is_finite() || minimum_target < 0.0 {
                return Err(ApprovalError::Invalid(
                    "minimumTarget must be a finite non-negative value".to_owned(),
                ));
            }
        }
        if let Some(schedule_type) = args.schedule_type.as_deref() {
            validate_one_of(schedule_type, "scheduleType", &["daily", "custom"])?;
        }
        let target_days = match args.target_days {
            Some(values) => Some(normalize_target_days(values)?),
            None => None,
        };
        if args.schedule_type.as_deref() == Some("custom")
            && target_days.as_ref().is_some_and(Vec::is_empty)
        {
            return Err(ApprovalError::Invalid(
                "targetDays must contain at least one weekday for custom schedules".to_owned(),
            ));
        }
        validate_optional_date(args.start_date.as_deref(), "startDate")?;
        if name.is_none()
            && args.minimum_target.is_none()
            && !args.clear_minimum_target
            && args.normal_target.is_none()
            && target_days.is_none()
            && args.schedule_type.is_none()
            && args.start_date.is_none()
            && args.description.is_none()
            && !args.clear_description
            && args.is_archived.is_none()
        {
            return Err(ApprovalError::Invalid(
                "at least one habit field must be changed".to_owned(),
            ));
        }

        let action = json!({
            "habitId": habit_id,
            "name": name,
            "minimumTarget": args.minimum_target,
            "clearMinimumTarget": args.clear_minimum_target,
            "normalTarget": args.normal_target,
            "targetDays": target_days,
            "scheduleType": args.schedule_type,
            "startDate": args.start_date,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "clearDescription": args.clear_description,
            "isArchived": args.is_archived
        });
        propose(
            &ctx,
            Self::NAME,
            "update_habit",
            arguments_json,
            action,
            json!({"habitId": habit_id}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateWaitingItemTool {
    const NAME: &'static str = "lifetrace_propose_create_waiting_item";
    type Args = CreateWaitingItemArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace Waiting Item 的写操作，用于记录正在等待某人或某结果的事项。时间不明确时先询问，不要猜测。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "title":{"type":"string"},
                "description":{"type":"string"},
                "waitingFor":{"type":"string"},
                "expectedAt":{"type":"string","description":"RFC3339 timestamp"},
                "followUpAt":{"type":"string","description":"RFC3339 timestamp"},
                "sourceTaskId":{"type":"string"}
            },
            "required":["title","waitingFor"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();
        let title = bounded_required(&args.title, "title", 300)?;
        let waiting_for = bounded_required(&args.waiting_for, "waitingFor", 300)?;
        validate_optional_timestamp(args.expected_at.as_deref(), "expectedAt")?;
        validate_optional_timestamp(args.follow_up_at.as_deref(), "followUpAt")?;
        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": title,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "waitingFor": waiting_for,
            "expectedAt": args.expected_at,
            "followUpAt": args.follow_up_at,
            "sourceTaskId": bounded_optional(args.source_task_id.as_deref(), 200)
        });
        propose(
            &ctx,
            Self::NAME,
            "create_waiting_item",
            arguments_json,
            action,
            json!({"title": title, "waitingFor": waiting_for}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeUpdateWaitingItemTool {
    const NAME: &'static str = "lifetrace_propose_update_waiting_item";
    type Args = UpdateWaitingItemArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出修改 Waiting Item 的写操作。可调整内容、等待对象、预期/跟进时间，并可标记 resolved 或重新打开；修改前先查询 waitingItemId。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "waitingItemId":{"type":"string"},
                "title":{"type":"string"},
                "description":{"type":"string"},
                "clearDescription":{"type":"boolean","default":false},
                "waitingFor":{"type":"string"},
                "expectedAt":{"type":"string","description":"RFC3339 timestamp"},
                "clearExpectedAt":{"type":"boolean","default":false},
                "followUpAt":{"type":"string","description":"RFC3339 timestamp"},
                "clearFollowUpAt":{"type":"boolean","default":false},
                "status":{"type":"string","enum":["open","resolved"]},
                "resolutionSummary":{"type":"string"},
                "clearResolutionSummary":{"type":"boolean","default":false}
            },
            "required":["waitingItemId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();
        let waiting_item_id = bounded_required(&args.waiting_item_id, "waitingItemId", 200)?;
        let title = match args.title.as_deref() {
            Some(value) => Some(bounded_required(value, "title", 300)?),
            None => None,
        };
        let waiting_for = match args.waiting_for.as_deref() {
            Some(value) => Some(bounded_required(value, "waitingFor", 300)?),
            None => None,
        };
        if args.description.is_some() && args.clear_description {
            return Err(ApprovalError::Invalid(
                "description and clearDescription cannot be used together".to_owned(),
            ));
        }
        if args.expected_at.is_some() && args.clear_expected_at {
            return Err(ApprovalError::Invalid(
                "expectedAt and clearExpectedAt cannot be used together".to_owned(),
            ));
        }
        if args.follow_up_at.is_some() && args.clear_follow_up_at {
            return Err(ApprovalError::Invalid(
                "followUpAt and clearFollowUpAt cannot be used together".to_owned(),
            ));
        }
        if args.resolution_summary.is_some() && args.clear_resolution_summary {
            return Err(ApprovalError::Invalid(
                "resolutionSummary and clearResolutionSummary cannot be used together".to_owned(),
            ));
        }
        validate_optional_timestamp(args.expected_at.as_deref(), "expectedAt")?;
        validate_optional_timestamp(args.follow_up_at.as_deref(), "followUpAt")?;
        if let Some(status) = args.status.as_deref() {
            validate_one_of(status, "status", &["open", "resolved"])?;
        }
        if title.is_none()
            && waiting_for.is_none()
            && args.description.is_none()
            && !args.clear_description
            && args.expected_at.is_none()
            && !args.clear_expected_at
            && args.follow_up_at.is_none()
            && !args.clear_follow_up_at
            && args.status.is_none()
            && args.resolution_summary.is_none()
            && !args.clear_resolution_summary
        {
            return Err(ApprovalError::Invalid(
                "at least one waiting item field must be changed".to_owned(),
            ));
        }
        let action = json!({
            "waitingItemId": waiting_item_id,
            "title": title,
            "description": bounded_optional(args.description.as_deref(), 4_000),
            "clearDescription": args.clear_description,
            "waitingFor": waiting_for,
            "expectedAt": args.expected_at,
            "clearExpectedAt": args.clear_expected_at,
            "followUpAt": args.follow_up_at,
            "clearFollowUpAt": args.clear_follow_up_at,
            "status": args.status,
            "resolutionSummary": bounded_optional(args.resolution_summary.as_deref(), 4_000),
            "clearResolutionSummary": args.clear_resolution_summary
        });
        propose(
            &ctx,
            Self::NAME,
            "update_waiting_item",
            arguments_json,
            action,
            json!({"waitingItemId": waiting_item_id}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateReminderTool {
    const NAME: &'static str = "lifetrace_propose_create_reminder";
    type Args = CreateReminderArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出为已有任务、日程或 Waiting Item 创建提醒。必须先查询确认 subjectId；提醒时间不明确时先询问用户。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "subjectType":{"type":"string","enum":["task","calendar_event","waiting_item"]},
                "subjectId":{"type":"string"},
                "triggerAt":{"type":"string","description":"RFC3339 timestamp"},
                "title":{"type":"string"},
                "body":{"type":"string"}
            },
            "required":["subjectType","subjectId","triggerAt"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();
        validate_one_of(
            &args.subject_type,
            "subjectType",
            &["task", "calendar_event", "waiting_item"],
        )?;
        let subject_id = bounded_required(&args.subject_id, "subjectId", 200)?;
        validate_optional_timestamp(Some(args.trigger_at.as_str()), "triggerAt")?;
        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "subjectType": args.subject_type,
            "subjectId": subject_id,
            "triggerAt": args.trigger_at,
            "title": bounded_optional(args.title.as_deref(), 300),
            "body": bounded_optional(args.body.as_deref(), 4_000)
        });
        propose(
            &ctx,
            Self::NAME,
            "create_reminder",
            arguments_json,
            action,
            json!({"subjectType": args.subject_type, "subjectId": subject_id, "triggerAt": args.trigger_at}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeUpdateReminderTool {
    const NAME: &'static str = "lifetrace_propose_update_reminder";
    type Args = UpdateReminderArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出修改已有提醒。可改触发时间、稍后提醒时间、标题/正文，或设为 scheduled/dismissed/cancelled；修改前先查询 reminderId。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。仅当用户明确修改/替换当前会话中的未审批提案时，填写要被替代的 pending approvalId"},
                "reminderId":{"type":"string"},
                "triggerAt":{"type":"string","description":"RFC3339 timestamp"},
                "snoozedUntil":{"type":"string","description":"RFC3339 timestamp"},
                "clearSnooze":{"type":"boolean","default":false},
                "status":{"type":"string","enum":["scheduled","dismissed","cancelled"]},
                "title":{"type":"string"},
                "clearTitle":{"type":"boolean","default":false},
                "body":{"type":"string"},
                "clearBody":{"type":"boolean","default":false}
            },
            "required":["reminderId"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_execution_write(&ctx)?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();
        let reminder_id = bounded_required(&args.reminder_id, "reminderId", 200)?;
        validate_optional_timestamp(args.trigger_at.as_deref(), "triggerAt")?;
        validate_optional_timestamp(args.snoozed_until.as_deref(), "snoozedUntil")?;
        if args.snoozed_until.is_some() && args.clear_snooze {
            return Err(ApprovalError::Invalid(
                "snoozedUntil and clearSnooze cannot be used together".to_owned(),
            ));
        }
        if args.title.is_some() && args.clear_title {
            return Err(ApprovalError::Invalid(
                "title and clearTitle cannot be used together".to_owned(),
            ));
        }
        if args.body.is_some() && args.clear_body {
            return Err(ApprovalError::Invalid(
                "body and clearBody cannot be used together".to_owned(),
            ));
        }
        if let Some(status) = args.status.as_deref() {
            validate_one_of(status, "status", &["scheduled", "dismissed", "cancelled"])?;
        }
        if args.trigger_at.is_none()
            && args.snoozed_until.is_none()
            && !args.clear_snooze
            && args.status.is_none()
            && args.title.is_none()
            && !args.clear_title
            && args.body.is_none()
            && !args.clear_body
        {
            return Err(ApprovalError::Invalid(
                "at least one reminder field must be changed".to_owned(),
            ));
        }
        let action = json!({
            "reminderId": reminder_id,
            "triggerAt": args.trigger_at,
            "snoozedUntil": args.snoozed_until,
            "clearSnooze": args.clear_snooze,
            "status": args.status,
            "title": bounded_optional(args.title.as_deref(), 300),
            "clearTitle": args.clear_title,
            "body": bounded_optional(args.body.as_deref(), 4_000),
            "clearBody": args.clear_body
        });
        propose(
            &ctx,
            Self::NAME,
            "update_reminder",
            arguments_json,
            action,
            json!({"reminderId": reminder_id}),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeCreateNoteTool {
    const NAME: &'static str = "lifetrace_propose_create_note";
    type Args = CreateNoteArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace Notes 笔记的写操作。正文使用 Markdown；可选 folderId 必须来自真实 note.folder。该工具不会直接创建笔记，只生成待用户批准的提案。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。用户要求修改尚未批准的创建笔记提案时，填写被替代的 approvalId"},
                "title":{"type":"string","description":"可选笔记标题；标题和正文至少一个非空"},
                "contentMarkdown":{"type":"string","description":"Markdown 正文，可为空字符串，但标题和正文至少一个非空"},
                "folderId":{"type":"string","description":"可选。真实 note.folder 的实体 ID；未指定时创建到 Notes 根目录"},
                "noteType":{"type":"string","enum":["quick","document"],"description":"默认 quick"}
            },
            "required":["contentMarkdown"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["sync:write", "notes:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let title = bounded_optional(args.title.as_deref(), 300);
        let content_markdown = args.content_markdown.trim_end().to_owned();
        if content_markdown.chars().count() > 100_000 {
            return Err(ApprovalError::Invalid(
                "contentMarkdown must not exceed 100000 characters".to_owned(),
            ));
        }
        if title.is_none() && content_markdown.trim().is_empty() {
            return Err(ApprovalError::Invalid(
                "title and contentMarkdown cannot both be empty".to_owned(),
            ));
        }

        let note_type = args.note_type.unwrap_or_else(|| "quick".to_owned());
        validate_one_of(&note_type, "noteType", &["quick", "document"])?;

        let folder_id = bounded_optional(args.folder_id.as_deref(), 200);
        if let Some(folder_id) = folder_id.as_deref() {
            let folder = sqlx::query(
                "SELECT entity_id FROM sync_entities WHERE user_id=$1 AND entity_type='note.folder' AND entity_id=$2 AND is_deleted=0",
            )
            .bind(ctx.user_id)
            .bind(folder_id)
            .fetch_optional(&ctx.pool)
            .await?;
            if folder.is_none() {
                return Err(ApprovalError::Invalid(
                    "folderId does not reference an existing note.folder".to_owned(),
                ));
            }
        }

        let action = json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": title,
            "contentMarkdown": content_markdown,
            "folderId": folder_id,
            "noteType": note_type
        });
        propose(
            &ctx,
            Self::NAME,
            "create_note",
            arguments_json,
            action,
            json!({
                "title": title,
                "contentMarkdown": content_markdown,
                "folderId": folder_id,
                "noteType": note_type
            }),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeSendMailTool {
    const NAME: &'static str = "lifetrace_propose_send_mail";
    type Args = SendMailArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "起草一封新邮件并生成发送审批。必须先用 lifetrace_list_mail_accounts 确认 accountId/identityId。该工具不会发送邮件；只有用户批准审批后才会通过 SMTP 发送。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。用户要求修改当前未批准邮件草稿时，填写被替代的 approvalId"},
                "accountId":{"type":"string","description":"来自 lifetrace_list_mail_accounts 的 accountId"},
                "identityId":{"type":"string","description":"可选。来自该账号 identities 的 identityId；省略时使用默认发件身份"},
                "to":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":50},
                "cc":{"type":"array","items":{"type":"string"},"maxItems":50},
                "bcc":{"type":"array","items":{"type":"string"},"maxItems":50},
                "subject":{"type":"string"},
                "bodyText":{"type":"string"}
            },
            "required":["accountId","to","subject","bodyText"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["mail:read", "mail:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();

        let (account_id, identity_id, from_address) =
            resolve_mail_sender(&ctx, &args.account_id, args.identity_id.as_deref()).await?;
        let to = normalize_mail_recipients(args.to, "to", true)?;
        let cc = normalize_mail_recipients(args.cc, "cc", false)?;
        let bcc = normalize_mail_recipients(args.bcc, "bcc", false)?;
        let subject = bounded_required(&args.subject, "subject", 300)?;
        let body_text = bounded_required(&args.body_text, "bodyText", 20_000)?;

        let action = json!({
            "accountId": account_id,
            "identityId": identity_id,
            "to": to,
            "cc": cc,
            "bcc": bcc,
            "subject": subject,
            "bodyText": body_text,
            "sourceMessageId": null,
            "inReplyToHeader": null
        });
        propose(
            &ctx,
            Self::NAME,
            "send_mail",
            arguments_json,
            action,
            json!({
                "from": from_address,
                "to": to,
                "cc": cc,
                "bcc": bcc,
                "subject": subject,
                "bodyText": body_text
            }),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

impl Tool for ProposeReplyMailTool {
    const NAME: &'static str = "lifetrace_propose_reply_mail";
    type Args = ReplyMailArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "起草对已有邮件的回复并生成发送审批。调用前必须先用 lifetrace_search_mail 定位准确 messageId；服务器会从原邮件推导账号、收件人与 In-Reply-To，避免模型猜测。该工具不会直接发送。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "supersedesApprovalId":{"type":"string","description":"可选。用户要求修改当前未批准回复草稿时，填写被替代的 approvalId"},
                "messageId":{"type":"string","description":"来自 lifetrace_search_mail 的原邮件 id"},
                "identityId":{"type":"string","description":"可选。指定同一账号下的发件身份"},
                "bodyText":{"type":"string"},
                "replyAll":{"type":"boolean","default":false}
            },
            "required":["messageId","bodyText"],
            "additionalProperties":false
        })
    }

    async fn call(
        &self,
        tool_context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let ctx = tool_context.require::<AgentInvocationContext>()?.clone();
        require_context_write_scopes(&ctx, &["mail:read", "mail:write"])?;
        let arguments_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_owned());
        let supersedes_approval_id = args.supersedes_approval_id.clone();
        let source_message_id = parse_uuid_field(&args.message_id, "messageId")?;

        let source = sqlx::query(
            "SELECT account_id,message_id,subject,from_json,to_json,cc_json,reply_to_json \
             FROM mail_messages WHERE user_id=$1 AND id=$2",
        )
        .bind(ctx.user_id)
        .bind(source_message_id)
        .fetch_optional(&ctx.pool)
        .await?
        .ok_or(ApprovalError::NotFound)?;

        let account_id = source.try_get::<Uuid, _>("account_id")?;
        let identity_arg = args.identity_id.as_deref();
        let (_, identity_id, from_address) =
            resolve_mail_sender_for_account(&ctx, account_id, identity_arg).await?;

        let reply_to_json = source.try_get::<Value, _>("reply_to_json")?;
        let from_json = source.try_get::<Value, _>("from_json")?;
        let original_to_json = source.try_get::<Value, _>("to_json")?;
        let original_cc_json = source.try_get::<Value, _>("cc_json")?;

        let mut to = collect_mail_addresses(&reply_to_json);
        if to.is_empty() {
            to = collect_mail_addresses(&from_json);
        }
        to = normalize_mail_recipients(to, "to", true)?;

        let mut cc: Vec<String> = Vec::new();
        if args.reply_all {
            let owned = owned_mail_address_keys(&ctx, account_id).await?;
            let to_keys = to
                .iter()
                .map(|value| mail_address_key(value))
                .collect::<Vec<_>>();
            for value in collect_mail_addresses(&original_to_json)
                .into_iter()
                .chain(collect_mail_addresses(&original_cc_json))
            {
                let key = mail_address_key(&value);
                if !owned.contains(&key)
                    && !to_keys.contains(&key)
                    && !cc.iter().any(|existing| mail_address_key(existing) == key)
                {
                    cc.push(value);
                }
            }
            cc = normalize_mail_recipients(cc, "cc", false)?;
        }

        let original_subject = source.try_get::<String, _>("subject")?;
        let subject = reply_mail_subject(&original_subject);
        let body_text = bounded_required(&args.body_text, "bodyText", 20_000)?;
        let in_reply_to_header = source.try_get::<Option<String>, _>("message_id")?;

        let action = json!({
            "accountId": account_id,
            "identityId": identity_id,
            "to": to,
            "cc": cc,
            "bcc": [],
            "subject": subject,
            "bodyText": body_text,
            "sourceMessageId": source_message_id,
            "inReplyToHeader": in_reply_to_header
        });
        propose(
            &ctx,
            Self::NAME,
            "reply_mail",
            arguments_json,
            action,
            json!({
                "from": from_address,
                "to": to,
                "cc": cc,
                "subject": subject,
                "bodyText": body_text,
                "sourceMessageId": source_message_id,
                "replyAll": args.reply_all
            }),
            supersedes_approval_id.as_deref(),
        )
        .await
    }
}

async fn propose(
    ctx: &AgentInvocationContext,
    tool_name: &str,
    action_name: &str,
    arguments_json: String,
    action: Value,
    preview: Value,
    supersedes_approval_id: Option<&str>,
) -> Result<Value, ApprovalError> {
    let tool_call_id = Uuid::new_v4();
    let approval_id = Uuid::new_v4();
    let action_json = serde_json::to_string(&action).unwrap_or_else(|_| "{}".to_owned());
    let supersedes_approval_id = supersedes_approval_id
        .map(|value| {
            Uuid::parse_str(value).map_err(|_| {
                ApprovalError::Invalid("supersedesApprovalId must be a UUID".to_owned())
            })
        })
        .transpose()?;
    let mut tx = ctx.pool.begin().await?;

    let superseded_tool_call_id = if let Some(superseded_id) = supersedes_approval_id {
        let previous = sqlx::query(
            "SELECT action_name,tool_call_id FROM agent_approvals \
             WHERE id=$1 AND user_id=$2 AND session_id=$3 AND status='pending' \
               AND (expires_at IS NULL OR expires_at>CURRENT_TIMESTAMP)",
        )
        .bind(superseded_id)
        .bind(ctx.user_id)
        .bind(ctx.session_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            ApprovalError::Conflict(
                "the approval being replaced is no longer pending in this conversation".to_owned(),
            )
        })?;
        let previous_action: String = previous.try_get("action_name")?;
        if previous_action != action_name {
            return Err(ApprovalError::Conflict(format!(
                "cannot replace {previous_action} approval with {action_name}"
            )));
        }
        previous.try_get::<Option<Uuid>, _>("tool_call_id")?
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO agent_tool_calls \
         (id,run_id,session_id,user_id,tool_name,arguments_json,status,requires_approval) \
         VALUES ($1,$2,$3,$4,$5,$6,'awaiting_approval',1)",
    )
    .bind(tool_call_id)
    .bind(ctx.run_id)
    .bind(ctx.session_id)
    .bind(ctx.user_id)
    .bind(tool_name)
    .bind(arguments_json)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO agent_approvals \
         (id,run_id,session_id,user_id,tool_call_id,action_name,action_json,status,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,'pending',datetime('now', $8))",
    )
    .bind(approval_id)
    .bind(ctx.run_id)
    .bind(ctx.session_id)
    .bind(ctx.user_id)
    .bind(tool_call_id)
    .bind(action_name)
    .bind(action_json)
    .bind(format!("+{APPROVAL_TTL_MINUTES} minutes"))
    .execute(&mut *tx)
    .await?;

    if let Some(superseded_id) = supersedes_approval_id {
        let changed = sqlx::query(
            "UPDATE agent_approvals SET status='cancelled',decided_at=CURRENT_TIMESTAMP, \
             superseded_by_approval_id=$4,cancellation_reason='superseded' \
             WHERE id=$1 AND user_id=$2 AND session_id=$3 AND status='pending'",
        )
        .bind(superseded_id)
        .bind(ctx.user_id)
        .bind(ctx.session_id)
        .bind(approval_id)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(ApprovalError::Conflict(
                "approval state changed while replacing the proposal".to_owned(),
            ));
        }
        if let Some(previous_tool_call_id) = superseded_tool_call_id {
            sqlx::query(
                "UPDATE agent_tool_calls SET status='denied',error_message='approval superseded', \
                 finished_at=CURRENT_TIMESTAMP \
                 WHERE id=$1 AND user_id=$2 AND status='awaiting_approval'",
            )
            .bind(previous_tool_call_id)
            .bind(ctx.user_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    tracing::info!(
        run_id = %ctx.run_id,
        session_id = %ctx.session_id,
        tool_call_id = %tool_call_id,
        approval_id = %approval_id,
        action_name,
        "agent write action proposed"
    );

    Ok(json!({
        "requiresApproval": true,
        "approvalId": approval_id,
        "actionName": action_name,
        "preview": preview,
        "supersedesApprovalId": supersedes_approval_id,
        "expiresInMinutes": APPROVAL_TTL_MINUTES
    }))
}

pub async fn list_for_session(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    session_id: Uuid,
    limit: i64,
) -> Result<Vec<AgentApproval>, ApprovalError> {
    let expired = sqlx::query(
        "UPDATE agent_approvals SET status='expired',decided_at=CURRENT_TIMESTAMP          WHERE user_id=$1 AND session_id=$2 AND status='pending'            AND expires_at IS NOT NULL AND expires_at<=CURRENT_TIMESTAMP          RETURNING tool_call_id",
    )
    .bind(user_id)
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    for row in expired {
        let tool_call_id: Option<Uuid> = row.try_get("tool_call_id")?;
        if let Some(tool_call_id) = tool_call_id {
            sqlx::query(
                "UPDATE agent_tool_calls SET status='denied',error_message='approval expired',                  finished_at=CURRENT_TIMESTAMP WHERE id=$1 AND user_id=$2                    AND status='awaiting_approval'",
            )
            .bind(tool_call_id)
            .bind(user_id)
            .execute(pool)
            .await?;
        }
    }

    let items = sqlx::query_as::<_, AgentApproval>(
        "SELECT a.id,a.run_id,a.session_id,a.tool_call_id,a.action_name,a.action_json, \
                a.status,a.requested_at,a.decided_at,a.expires_at, \
                a.superseded_by_approval_id,a.cancellation_reason \
         FROM agent_approvals a \
         JOIN agent_sessions s ON s.id=a.session_id \
         WHERE a.user_id=$1 AND a.session_id=$2 AND s.user_id=$1 \
           AND s.app_id=$3 AND s.scopes_json=$4 \
         ORDER BY CASE WHEN a.status='pending' THEN 0 ELSE 1 END,                   a.requested_at DESC,a.rowid DESC LIMIT $5",
    )
    .bind(user_id)
    .bind(session_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?;
    Ok(items)
}

pub async fn decide(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    user_id: Uuid,
    access: &AgentAccessPartition,
    approval_id: Uuid,
    decision: ApprovalDecision,
) -> Result<ApprovalDecisionOutput, ApprovalError> {
    let mut approval = load_approval(&state.pool, user_id, access, approval_id).await?;

    if approval.status == "pending"
        && approval
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        expire_approval(&state.pool, user_id, &approval).await?;
        return Err(ApprovalError::Expired);
    }

    match decision {
        ApprovalDecision::Reject => {
            if approval.status == "rejected" {
                return Ok(ApprovalDecisionOutput {
                    approval,
                    result: None,
                });
            }
            if approval.status != "pending" {
                return Err(ApprovalError::Conflict(format!(
                    "cannot reject approval in {} state",
                    approval.status
                )));
            }
            sqlx::query(
                "UPDATE agent_approvals SET status='rejected',decided_at=CURRENT_TIMESTAMP, \
                 decided_by_session_id=$3 WHERE id=$1 AND user_id=$2 AND status='pending'",
            )
            .bind(approval.id)
            .bind(user_id)
            .bind(principal.session_id.as_str())
            .execute(&state.pool)
            .await?;
            if let Some(tool_call_id) = approval.tool_call_id {
                sqlx::query(
                    "UPDATE agent_tool_calls SET status='denied',error_message='user rejected approval', \
                     finished_at=CURRENT_TIMESTAMP WHERE id=$1 AND user_id=$2",
                )
                .bind(tool_call_id)
                .bind(user_id)
                .execute(&state.pool)
                .await?;
            }
            approval = load_approval(&state.pool, user_id, access, approval_id).await?;
            tracing::info!(
                approval_id = %approval_id,
                session_id = %approval.session_id,
                "agent approval rejected"
            );
            Ok(ApprovalDecisionOutput {
                approval,
                result: None,
            })
        }
        ApprovalDecision::Approve => {
            require_principal_action_write(principal, &approval.action_name)?;
            if approval.status == "approved" {
                let result = load_tool_result(&state.pool, user_id, approval.tool_call_id).await?;
                return Ok(ApprovalDecisionOutput { approval, result });
            }
            if approval.status != "pending" {
                return Err(ApprovalError::Conflict(format!(
                    "cannot approve approval in {} state",
                    approval.status
                )));
            }

            let result = match execute_action(state, principal, &approval).await {
                Ok(value) => value,
                Err(error) => {
                    if let Some(tool_call_id) = approval.tool_call_id {
                        let message: String = error.to_string().chars().take(500).collect();
                        let _ = sqlx::query(
                            "UPDATE agent_tool_calls SET error_message=$3 \
                             WHERE id=$1 AND user_id=$2 AND status='awaiting_approval'",
                        )
                        .bind(tool_call_id)
                        .bind(user_id)
                        .bind(message)
                        .execute(&state.pool)
                        .await;
                    }
                    return Err(error);
                }
            };

            let encoded = serde_json::to_string(&result).unwrap_or_else(|_| "null".to_owned());
            let changed = sqlx::query(
                "UPDATE agent_approvals SET status='approved',decided_at=CURRENT_TIMESTAMP, \
                 decided_by_session_id=$3 WHERE id=$1 AND user_id=$2 AND status='pending'",
            )
            .bind(approval.id)
            .bind(user_id)
            .bind(principal.session_id.as_str())
            .execute(&state.pool)
            .await?;
            if changed.rows_affected() != 1 {
                approval = load_approval(&state.pool, user_id, access, approval_id).await?;
                if approval.status == "approved" {
                    let stored =
                        load_tool_result(&state.pool, user_id, approval.tool_call_id).await?;
                    return Ok(ApprovalDecisionOutput {
                        approval,
                        result: stored.or(Some(result)),
                    });
                }
                return Err(ApprovalError::Conflict(
                    "approval state changed while executing".to_owned(),
                ));
            }

            if let Some(tool_call_id) = approval.tool_call_id {
                sqlx::query(
                    "UPDATE agent_tool_calls SET status='completed',result_json=$3,error_message=NULL, \
                     finished_at=CURRENT_TIMESTAMP WHERE id=$1 AND user_id=$2",
                )
                .bind(tool_call_id)
                .bind(user_id)
                .bind(encoded)
                .execute(&state.pool)
                .await?;
            }

            approval = load_approval(&state.pool, user_id, access, approval_id).await?;
            tracing::info!(
                approval_id = %approval_id,
                session_id = %approval.session_id,
                action_name = %approval.action_name,
                "agent approval executed"
            );
            Ok(ApprovalDecisionOutput {
                approval,
                result: Some(result),
            })
        }
    }
}

async fn load_approval(
    pool: &SqlitePool,
    user_id: Uuid,
    access: &AgentAccessPartition,
    approval_id: Uuid,
) -> Result<AgentApproval, ApprovalError> {
    sqlx::query_as::<_, AgentApproval>(
        "SELECT a.id,a.run_id,a.session_id,a.tool_call_id,a.action_name,a.action_json, \
                a.status,a.requested_at,a.decided_at,a.expires_at, \
                a.superseded_by_approval_id,a.cancellation_reason \
         FROM agent_approvals a JOIN agent_sessions s ON s.id=a.session_id \
         WHERE a.id=$1 AND a.user_id=$2 AND s.user_id=$2 \
           AND s.app_id=$3 AND s.scopes_json=$4",
    )
    .bind(approval_id)
    .bind(user_id)
    .bind(&access.app_id)
    .bind(&access.scopes_json)
    .fetch_optional(pool)
    .await?
    .ok_or(ApprovalError::NotFound)
}

async fn expire_approval(
    pool: &SqlitePool,
    user_id: Uuid,
    approval: &AgentApproval,
) -> Result<(), ApprovalError> {
    sqlx::query(
        "UPDATE agent_approvals SET status='expired',decided_at=CURRENT_TIMESTAMP \
         WHERE id=$1 AND user_id=$2 AND status='pending'",
    )
    .bind(approval.id)
    .bind(user_id)
    .execute(pool)
    .await?;
    if let Some(tool_call_id) = approval.tool_call_id {
        sqlx::query(
            "UPDATE agent_tool_calls SET status='denied',error_message='approval expired', \
             finished_at=CURRENT_TIMESTAMP WHERE id=$1 AND user_id=$2",
        )
        .bind(tool_call_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn load_tool_result(
    pool: &SqlitePool,
    user_id: Uuid,
    tool_call_id: Option<Uuid>,
) -> Result<Option<Value>, ApprovalError> {
    let Some(tool_call_id) = tool_call_id else {
        return Ok(None);
    };
    let encoded: Option<String> =
        sqlx::query_scalar("SELECT result_json FROM agent_tool_calls WHERE id=$1 AND user_id=$2")
            .bind(tool_call_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    Ok(encoded.and_then(|value| serde_json::from_str(&value).ok()))
}

async fn execute_action(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
) -> Result<Value, ApprovalError> {
    match approval.action_name.as_str() {
        "create_note" => {
            let action: CreateNoteAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_note(state, principal, approval, action).await
        }
        "send_mail" | "reply_mail" => {
            let action: SendMailAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_send_mail(state, principal, approval, action).await
        }
        "create_task" => {
            let action: CreateTaskAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_task(state, principal, approval, action).await
        }
        "update_task" => {
            let action: UpdateTaskAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_update_task(state, principal, approval, action).await
        }
        "create_calendar_event" => {
            let action: CreateCalendarEventAction =
                serde_json::from_value(approval.action_json.clone())
                    .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_calendar_event(state, principal, approval, action).await
        }
        "create_project" => {
            let action: CreateProjectAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_project(state, principal, approval, action).await
        }
        "update_project" => {
            let action: UpdateProjectAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_update_project(state, principal, approval, action).await
        }
        "create_habit" => {
            let action: CreateHabitAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_habit(state, principal, approval, action).await
        }
        "update_habit" => {
            let action: UpdateHabitAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_update_habit(state, principal, approval, action).await
        }
        "create_waiting_item" => {
            let action: CreateWaitingItemAction =
                serde_json::from_value(approval.action_json.clone())
                    .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_waiting_item(state, principal, approval, action).await
        }
        "update_waiting_item" => {
            let action: UpdateWaitingItemAction =
                serde_json::from_value(approval.action_json.clone())
                    .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_update_waiting_item(state, principal, approval, action).await
        }
        "create_reminder" => {
            let action: CreateReminderAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_create_reminder(state, principal, approval, action).await
        }
        "update_reminder" => {
            let action: UpdateReminderAction = serde_json::from_value(approval.action_json.clone())
                .map_err(|error| ApprovalError::Invalid(error.to_string()))?;
            execute_update_reminder(state, principal, approval, action).await
        }
        other => Err(ApprovalError::Invalid(format!(
            "unsupported action name: {other}"
        ))),
    }
}

async fn execute_create_note(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateNoteAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(&principal.user_id, EntityType::NOTE_NOTE, &action.entity_id)
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_note",
                "entityType":EntityType::NOTE_NOTE,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    if let Some(folder_id) = action.folder_id.as_deref() {
        let folder = state
            .store
            .entity(&principal.user_id, EntityType::NOTE_FOLDER, folder_id)
            .await
            .map_err(|error| ApprovalError::Execution(error.to_string()))?
            .filter(|record| !record.deleted)
            .ok_or_else(|| ApprovalError::Invalid(
                "folderId no longer references an existing note.folder".to_owned(),
            ))?;
        if folder.entity_id != folder_id {
            return Err(ApprovalError::Invalid("folderId is invalid".to_owned()));
        }
    }

    let content_text = action.content_markdown.clone();
    let summary = content_text.chars().take(160).collect::<String>();
    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "title": action.title,
        "noteType": action.note_type,
        "folderId": action.folder_id,
        "contentJson": {
            "type": "markdown",
            "source": action.content_markdown,
            "editor": "codemirror",
            "properties": {
                "status": "",
                "source": "agent",
                "aliases": []
            }
        },
        "contentHtml": "",
        "contentText": content_text,
        "contentMarkdown": action.content_markdown,
        "summary": summary,
        "isPinned": false,
        "isFavorite": false,
        "isArchived": false,
        "aiSummary": null,
        "aiTags": null,
        "embeddingStatus": null,
        "lastAiProcessedAt": null
    });

    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::NOTE_NOTE,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_send_mail(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: SendMailAction,
) -> Result<Value, ApprovalError> {
    let message_id = MailService::new(state.pool.clone(), state.config.clone())
        .send(
            &principal.user_id,
            action.account_id,
            SendMailInput {
                identity_id: action.identity_id,
                attachment_draft_id: None,
                to: action.to.clone(),
                cc: action.cc.clone(),
                bcc: action.bcc.clone(),
                subject: action.subject.clone(),
                body_text: action.body_text.clone(),
                in_reply_to_message_id: action.source_message_id,
                in_reply_to_header: action.in_reply_to_header.clone(),
                idempotency_key: format!("agent-approval:{}", approval.id),
            },
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?;

    state.mail_realtime.publish_account_updated(
        principal.user_id.as_str(),
        action.account_id,
        0,
        "agent_send",
    );

    Ok(json!({
        "action": approval.action_name,
        "accountId": action.account_id,
        "messageId": message_id,
        "sourceMessageId": action.source_message_id,
        "subject": action.subject,
        "to": action.to,
        "sent": true
    }))
}

async fn execute_create_task(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateTaskAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_TASK,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_task",
                "entityType":EntityType::EXECUTION_TASK,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "projectId": action.project_id,
        "parentTaskId": null,
        "title": action.title,
        "description": action.description,
        "status": "todo",
        "priority": action.priority,
        "estimatedMinutes": null,
        "actualMinutes": null,
        "dueAt": action.due_at,
        "scheduledStartAt": action.scheduled_start_at,
        "scheduledEndAt": action.scheduled_end_at,
        "timezone": action.timezone,
        "context": action.context,
        "completedAt": null,
        "cancelledAt": null,
        "recurrenceRuleId": null
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_TASK,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_update_task(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: UpdateTaskAction,
) -> Result<Value, ApprovalError> {
    let current = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_TASK,
            &action.task_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .filter(|record| !record.deleted)
        .ok_or(ApprovalError::NotFound)?;

    let mut payload: Value = current.payload.clone().into();
    let object = payload.as_object_mut().ok_or_else(|| {
        ApprovalError::Invalid("stored task payload is not a JSON object".to_owned())
    })?;

    let mut changed = false;
    if let Some(title) = action.title.as_ref() {
        changed |= set_if_changed(object, "title", Value::String(title.clone()));
    }
    if let Some(status) = action.status.as_ref() {
        changed |= set_if_changed(object, "status", Value::String(status.clone()));
        if status == "done" {
            changed |= set_if_changed(
                object,
                "completedAt",
                serde_json::to_value(approval.requested_at).unwrap_or(Value::Null),
            );
            changed |= set_if_changed(object, "cancelledAt", Value::Null);
        } else if status == "cancelled" {
            changed |= set_if_changed(
                object,
                "cancelledAt",
                serde_json::to_value(approval.requested_at).unwrap_or(Value::Null),
            );
            changed |= set_if_changed(object, "completedAt", Value::Null);
        } else {
            changed |= set_if_changed(object, "completedAt", Value::Null);
            changed |= set_if_changed(object, "cancelledAt", Value::Null);
        }
    }
    if let Some(priority) = action.priority.as_ref() {
        changed |= set_if_changed(object, "priority", Value::String(priority.clone()));
    }
    if action.clear_due_at {
        changed |= set_if_changed(object, "dueAt", Value::Null);
    } else if let Some(due_at) = action.due_at.as_ref() {
        changed |= set_if_changed(object, "dueAt", Value::String(due_at.clone()));
    }
    if action.clear_schedule {
        changed |= set_if_changed(object, "scheduledStartAt", Value::Null);
        changed |= set_if_changed(object, "scheduledEndAt", Value::Null);
    } else if let (Some(start), Some(end)) = (
        action.scheduled_start_at.as_ref(),
        action.scheduled_end_at.as_ref(),
    ) {
        changed |= set_if_changed(object, "scheduledStartAt", Value::String(start.clone()));
        changed |= set_if_changed(object, "scheduledEndAt", Value::String(end.clone()));
        changed |= set_if_changed(object, "context", Value::Null);
    }
    if let Some(estimated_minutes) = action.estimated_minutes {
        changed |= set_if_changed(object, "estimatedMinutes", json!(estimated_minutes));
    }

    if !changed {
        return Ok(json!({
            "action":"update_task",
            "entityType":EntityType::EXECUTION_TASK,
            "entityId":action.task_id,
            "serverVersion":current.server_version.to_string(),
            "alreadySatisfied":true
        }));
    }

    update_meta_for_server_edit(&mut payload, approval.requested_at)?;
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_TASK,
            entity_id: action.task_id,
            base_server_version: ServerVersion::from_u64(current.server_version),
            payload,
            change_id: format!("agent-approval-{}-v{}", approval.id, current.server_version),
        },
    )
    .await
}

async fn execute_create_calendar_event(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateCalendarEventAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_CALENDAR_EVENT,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_calendar_event",
                "entityType":EntityType::EXECUTION_CALENDAR_EVENT,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "title": action.title,
        "description": action.description,
        "isAllDay": action.is_all_day,
        "startAt": action.start_at,
        "endAt": action.end_at,
        "startLocalDate": action.start_local_date,
        "endLocalDate": action.end_local_date,
        "timezone": action.timezone,
        "status": "scheduled",
        "recurrenceRuleId": null,
        "sourceTaskId": null
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_CALENDAR_EVENT,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_create_project(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateProjectAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_PROJECT,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_project",
                "entityType":EntityType::EXECUTION_PROJECT,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "name": action.name,
        "description": action.description,
        "status": "active",
        "color": action.color,
        "icon": action.icon,
        "sortOrder": 0
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_PROJECT,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_update_project(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: UpdateProjectAction,
) -> Result<Value, ApprovalError> {
    let current = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_PROJECT,
            &action.project_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .filter(|record| !record.deleted)
        .ok_or(ApprovalError::NotFound)?;

    let mut payload: Value = current.payload.clone().into();
    let object = payload.as_object_mut().ok_or_else(|| {
        ApprovalError::Invalid("stored project payload is not a JSON object".to_owned())
    })?;

    let mut changed = false;
    if let Some(name) = action.name.as_ref() {
        changed |= set_if_changed(object, "name", Value::String(name.clone()));
    }
    if action.clear_description {
        changed |= set_if_changed(object, "description", Value::Null);
    } else if let Some(description) = action.description.as_ref() {
        changed |= set_if_changed(object, "description", Value::String(description.clone()));
    }
    if let Some(status) = action.status.as_ref() {
        changed |= set_if_changed(object, "status", Value::String(status.clone()));
    }
    if let Some(color) = action.color.as_ref() {
        changed |= set_if_changed(object, "color", Value::String(color.clone()));
    }
    if let Some(icon) = action.icon.as_ref() {
        changed |= set_if_changed(object, "icon", Value::String(icon.clone()));
    }

    if !changed {
        return Ok(json!({
            "action":"update_project",
            "entityType":EntityType::EXECUTION_PROJECT,
            "entityId":action.project_id,
            "serverVersion":current.server_version.to_string(),
            "alreadySatisfied":true
        }));
    }

    update_meta_for_server_edit(&mut payload, approval.requested_at)?;
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_PROJECT,
            entity_id: action.project_id,
            base_server_version: ServerVersion::from_u64(current.server_version),
            payload,
            change_id: format!("agent-approval-{}-v{}", approval.id, current.server_version),
        },
    )
    .await
}

async fn execute_create_habit(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateHabitAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::HABIT_ACTIVITY,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_habit",
                "entityType":EntityType::HABIT_ACTIVITY,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    validate_habit_targets(action.minimum_target, action.normal_target)?;
    validate_optional_date(Some(action.start_date.as_str()), "startDate")?;
    let target_days = normalize_target_days(action.target_days)?;
    if action.schedule_type == "custom" && target_days.is_empty() {
        return Err(ApprovalError::Invalid(
            "custom habit schedule requires at least one target day".to_owned(),
        ));
    }
    let icon = action
        .name
        .chars()
        .next()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "✓".to_owned());
    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "name": action.name,
        "activityType": action.activity_type,
        "unit": action.unit,
        "minimumTarget": action.minimum_target,
        "normalTarget": action.normal_target,
        "targetPeriod": "daily",
        "targetDays": if action.schedule_type == "daily" { Vec::<u8>::new() } else { target_days },
        "icon": icon,
        "color": "#0f766e",
        "scheduleType": action.schedule_type,
        "startDate": action.start_date,
        "checkinMethod": "manual",
        "syncSource": "web",
        "description": action.description,
        "isArchived": false
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::HABIT_ACTIVITY,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_update_habit(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: UpdateHabitAction,
) -> Result<Value, ApprovalError> {
    let current = state
        .store
        .entity(
            &principal.user_id,
            EntityType::HABIT_ACTIVITY,
            &action.habit_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .filter(|record| !record.deleted)
        .ok_or(ApprovalError::NotFound)?;

    let mut payload: Value = current.payload.clone().into();
    let object = payload.as_object_mut().ok_or_else(|| {
        ApprovalError::Invalid("stored habit payload is not a JSON object".to_owned())
    })?;

    let mut changed = false;
    if let Some(name) = action.name.as_ref() {
        changed |= set_if_changed(object, "name", Value::String(name.clone()));
    }
    if action.clear_minimum_target {
        changed |= set_if_changed(object, "minimumTarget", Value::Null);
    } else if let Some(minimum_target) = action.minimum_target {
        changed |= set_if_changed(object, "minimumTarget", json!(minimum_target));
    }
    if let Some(normal_target) = action.normal_target {
        changed |= set_if_changed(object, "normalTarget", json!(normal_target));
    }
    if let Some(target_days) = action.target_days {
        changed |= set_if_changed(
            object,
            "targetDays",
            json!(normalize_target_days(target_days)?),
        );
    }
    if let Some(schedule_type) = action.schedule_type.as_ref() {
        changed |= set_if_changed(object, "scheduleType", Value::String(schedule_type.clone()));
        if schedule_type == "daily" {
            changed |= set_if_changed(object, "targetDays", json!([]));
        }
    }
    if let Some(start_date) = action.start_date.as_ref() {
        validate_optional_date(Some(start_date.as_str()), "startDate")?;
        changed |= set_if_changed(object, "startDate", Value::String(start_date.clone()));
    }
    if action.clear_description {
        changed |= set_if_changed(object, "description", Value::Null);
    } else if let Some(description) = action.description.as_ref() {
        changed |= set_if_changed(object, "description", Value::String(description.clone()));
    }
    if let Some(is_archived) = action.is_archived {
        changed |= set_if_changed(object, "isArchived", Value::Bool(is_archived));
    }

    let normal_target = object
        .get("normalTarget")
        .and_then(Value::as_f64)
        .ok_or_else(|| ApprovalError::Invalid("habit normalTarget is invalid".to_owned()))?;
    let minimum_target = match object.get("minimumTarget") {
        Some(Value::Null) | None => None,
        Some(value) => value.as_f64(),
    };
    validate_habit_targets(minimum_target, normal_target)?;

    let schedule_type = object
        .get("scheduleType")
        .and_then(Value::as_str)
        .unwrap_or("daily");
    if schedule_type == "custom" {
        let target_days = object
            .get("targetDays")
            .and_then(Value::as_array)
            .ok_or_else(|| ApprovalError::Invalid("habit targetDays is invalid".to_owned()))?;
        if target_days.is_empty() {
            return Err(ApprovalError::Invalid(
                "custom habit schedule requires at least one target day".to_owned(),
            ));
        }
    }

    if !changed {
        return Ok(json!({
            "action":"update_habit",
            "entityType":EntityType::HABIT_ACTIVITY,
            "entityId":action.habit_id,
            "serverVersion":current.server_version.to_string(),
            "alreadySatisfied":true
        }));
    }

    update_meta_for_server_edit(&mut payload, approval.requested_at)?;
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::HABIT_ACTIVITY,
            entity_id: action.habit_id,
            base_server_version: ServerVersion::from_u64(current.server_version),
            payload,
            change_id: format!("agent-approval-{}-v{}", approval.id, current.server_version),
        },
    )
    .await
}

async fn execute_create_waiting_item(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateWaitingItemAction,
) -> Result<Value, ApprovalError> {
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_WAITING_ITEM,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_waiting_item",
                "entityType":EntityType::EXECUTION_WAITING_ITEM,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "title": action.title,
        "description": action.description,
        "status": "open",
        "waitingFor": action.waiting_for,
        "expectedAt": action.expected_at,
        "followUpAt": action.follow_up_at,
        "resolvedAt": null,
        "resolutionSummary": null,
        "sourceTaskId": action.source_task_id
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_WAITING_ITEM,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_update_waiting_item(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: UpdateWaitingItemAction,
) -> Result<Value, ApprovalError> {
    let current = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_WAITING_ITEM,
            &action.waiting_item_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .filter(|record| !record.deleted)
        .ok_or(ApprovalError::NotFound)?;
    let mut payload: Value = current.payload.clone().into();
    let object = payload.as_object_mut().ok_or_else(|| {
        ApprovalError::Invalid("stored waiting item payload is not a JSON object".to_owned())
    })?;

    let mut changed = false;
    if let Some(title) = action.title.as_ref() {
        changed |= set_if_changed(object, "title", Value::String(title.clone()));
    }
    if action.clear_description {
        changed |= set_if_changed(object, "description", Value::Null);
    } else if let Some(description) = action.description.as_ref() {
        changed |= set_if_changed(object, "description", Value::String(description.clone()));
    }
    if let Some(waiting_for) = action.waiting_for.as_ref() {
        changed |= set_if_changed(object, "waitingFor", Value::String(waiting_for.clone()));
    }
    if action.clear_expected_at {
        changed |= set_if_changed(object, "expectedAt", Value::Null);
    } else if let Some(expected_at) = action.expected_at.as_ref() {
        changed |= set_if_changed(object, "expectedAt", Value::String(expected_at.clone()));
    }
    if action.clear_follow_up_at {
        changed |= set_if_changed(object, "followUpAt", Value::Null);
    } else if let Some(follow_up_at) = action.follow_up_at.as_ref() {
        changed |= set_if_changed(object, "followUpAt", Value::String(follow_up_at.clone()));
    }
    if action.clear_resolution_summary {
        changed |= set_if_changed(object, "resolutionSummary", Value::Null);
    } else if let Some(summary) = action.resolution_summary.as_ref() {
        changed |= set_if_changed(object, "resolutionSummary", Value::String(summary.clone()));
    }
    if let Some(status) = action.status.as_ref() {
        changed |= set_if_changed(object, "status", Value::String(status.clone()));
        if status == "resolved" {
            changed |= set_if_changed(
                object,
                "resolvedAt",
                serde_json::to_value(approval.requested_at).unwrap_or(Value::Null),
            );
        } else {
            changed |= set_if_changed(object, "resolvedAt", Value::Null);
            if action.resolution_summary.is_none() {
                changed |= set_if_changed(object, "resolutionSummary", Value::Null);
            }
        }
    }

    if !changed {
        return Ok(json!({
            "action":"update_waiting_item",
            "entityType":EntityType::EXECUTION_WAITING_ITEM,
            "entityId":action.waiting_item_id,
            "serverVersion":current.server_version.to_string(),
            "alreadySatisfied":true
        }));
    }
    update_meta_for_server_edit(&mut payload, approval.requested_at)?;
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_WAITING_ITEM,
            entity_id: action.waiting_item_id,
            base_server_version: ServerVersion::from_u64(current.server_version),
            payload,
            change_id: format!("agent-approval-{}-v{}", approval.id, current.server_version),
        },
    )
    .await
}

async fn execute_create_reminder(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: CreateReminderAction,
) -> Result<Value, ApprovalError> {
    ensure_reminder_subject_exists(state, principal, &action.subject_type, &action.subject_id)
        .await?;
    if let Some(existing) = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_REMINDER,
            &action.entity_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
    {
        if !existing.deleted {
            return Ok(json!({
                "action":"create_reminder",
                "entityType":EntityType::EXECUTION_REMINDER,
                "entityId":action.entity_id,
                "serverVersion":existing.server_version.to_string(),
                "alreadySatisfied":true
            }));
        }
    }

    let fire_key = format!(
        "{}:{}:{}",
        action.subject_type, action.subject_id, action.trigger_at
    );
    let payload = json!({
        "meta": base_meta(principal, &action.entity_id, approval.requested_at),
        "subjectType": action.subject_type,
        "subjectId": action.subject_id,
        "triggerAt": action.trigger_at,
        "status": "scheduled",
        "fireKey": fire_key,
        "snoozedUntil": null,
        "lastFiredAt": null,
        "title": action.title,
        "body": action.body
    });
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_REMINDER,
            entity_id: action.entity_id,
            base_server_version: ServerVersion::zero(),
            payload,
            change_id: format!("agent-approval-{}", approval.id),
        },
    )
    .await
}

async fn execute_update_reminder(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    action: UpdateReminderAction,
) -> Result<Value, ApprovalError> {
    let current = state
        .store
        .entity(
            &principal.user_id,
            EntityType::EXECUTION_REMINDER,
            &action.reminder_id,
        )
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .filter(|record| !record.deleted)
        .ok_or(ApprovalError::NotFound)?;
    let mut payload: Value = current.payload.clone().into();
    let object = payload.as_object_mut().ok_or_else(|| {
        ApprovalError::Invalid("stored reminder payload is not a JSON object".to_owned())
    })?;
    let subject_type = object
        .get("subjectType")
        .and_then(Value::as_str)
        .ok_or_else(|| ApprovalError::Invalid("reminder subjectType is invalid".to_owned()))?
        .to_owned();
    let subject_id = object
        .get("subjectId")
        .and_then(Value::as_str)
        .ok_or_else(|| ApprovalError::Invalid("reminder subjectId is invalid".to_owned()))?
        .to_owned();
    ensure_reminder_subject_exists(state, principal, &subject_type, &subject_id).await?;

    let mut changed = false;
    if let Some(trigger_at) = action.trigger_at.as_ref() {
        changed |= set_if_changed(object, "triggerAt", Value::String(trigger_at.clone()));
        changed |= set_if_changed(
            object,
            "fireKey",
            Value::String(format!("{subject_type}:{subject_id}:{trigger_at}")),
        );
    }
    if action.clear_snooze {
        changed |= set_if_changed(object, "snoozedUntil", Value::Null);
    } else if let Some(snoozed_until) = action.snoozed_until.as_ref() {
        changed |= set_if_changed(object, "snoozedUntil", Value::String(snoozed_until.clone()));
    }
    if action.clear_title {
        changed |= set_if_changed(object, "title", Value::Null);
    } else if let Some(title) = action.title.as_ref() {
        changed |= set_if_changed(object, "title", Value::String(title.clone()));
    }
    if action.clear_body {
        changed |= set_if_changed(object, "body", Value::Null);
    } else if let Some(body) = action.body.as_ref() {
        changed |= set_if_changed(object, "body", Value::String(body.clone()));
    }
    if let Some(status) = action.status.as_ref() {
        changed |= set_if_changed(object, "status", Value::String(status.clone()));
        if status == "scheduled" {
            changed |= set_if_changed(object, "lastFiredAt", Value::Null);
        } else {
            changed |= set_if_changed(object, "snoozedUntil", Value::Null);
        }
    }
    let status = object
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("scheduled");
    if status != "scheduled"
        && object
            .get("snoozedUntil")
            .is_some_and(|value| !value.is_null())
    {
        return Err(ApprovalError::Invalid(
            "only scheduled reminders may have snoozedUntil".to_owned(),
        ));
    }

    if !changed {
        return Ok(json!({
            "action":"update_reminder",
            "entityType":EntityType::EXECUTION_REMINDER,
            "entityId":action.reminder_id,
            "serverVersion":current.server_version.to_string(),
            "alreadySatisfied":true
        }));
    }
    update_meta_for_server_edit(&mut payload, approval.requested_at)?;
    push_upsert(
        state,
        principal,
        approval,
        SyncUpsertAction {
            entity_type: EntityType::EXECUTION_REMINDER,
            entity_id: action.reminder_id,
            base_server_version: ServerVersion::from_u64(current.server_version),
            payload,
            change_id: format!("agent-approval-{}-v{}", approval.id, current.server_version),
        },
    )
    .await
}

async fn ensure_reminder_subject_exists(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    subject_type: &str,
    subject_id: &str,
) -> Result<(), ApprovalError> {
    let entity_type = reminder_subject_entity_type(subject_type)?;
    let exists = state
        .store
        .entity(&principal.user_id, entity_type, subject_id)
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?
        .is_some_and(|record| !record.deleted);
    if exists {
        Ok(())
    } else {
        Err(ApprovalError::NotFound)
    }
}

fn reminder_subject_entity_type(subject_type: &str) -> Result<&'static str, ApprovalError> {
    match subject_type {
        "task" => Ok(EntityType::EXECUTION_TASK),
        "calendar_event" => Ok(EntityType::EXECUTION_CALENDAR_EVENT),
        "waiting_item" => Ok(EntityType::EXECUTION_WAITING_ITEM),
        _ => Err(ApprovalError::Invalid(
            "reminder subjectType is unsupported".to_owned(),
        )),
    }
}

struct SyncUpsertAction {
    entity_type: &'static str,
    entity_id: String,
    base_server_version: ServerVersion,
    payload: Value,
    change_id: String,
}

async fn push_upsert(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    approval: &AgentApproval,
    upsert: SyncUpsertAction,
) -> Result<Value, ApprovalError> {
    let request = PushRequestV1 {
        request_id: RequestId::new(format!("agent-{}", Uuid::new_v4())),
        client: SyncClientInfo {
            app_id: principal.app_id.clone(),
            client_version: "agent-server".to_owned(),
            platform: ClientPlatform::new("server"),
            protocol_version: 1,
            schema_version: 1,
            device_id: DeviceId::new(principal.device_id.as_str()),
        },
        changes: vec![SyncChangeV1 {
            change_id: ChangeId::new(upsert.change_id),
            entity_type: EntityType::new(upsert.entity_type),
            entity_id: EntityId::new(&upsert.entity_id),
            operation: ChangeOperation::new(ChangeOperation::UPSERT),
            base_server_version: upsert.base_server_version,
            entity_schema_version: 1,
            client_modified_at: approval.requested_at,
            payload: Some(JsonValue(upsert.payload)),
            atomic_group_id: None,
            dependencies: vec![],
        }],
    };

    let response = state
        .store
        .push(&principal.user_id, &request)
        .await
        .map_err(|error| ApprovalError::Execution(error.to_string()))?;
    let result = response
        .results
        .into_iter()
        .next()
        .ok_or_else(|| ApprovalError::Execution("sync returned no result".to_owned()))?;
    match result {
        accepted @ (PushChangeResultV1::Accepted { .. } | PushChangeResultV1::Duplicate { .. }) => {
            let sync_result = serde_json::to_value(&accepted)
                .map_err(|error| ApprovalError::Execution(error.to_string()))?;
            Ok(json!({
                "action": approval.action_name,
                "entityType": upsert.entity_type,
                "entityId": upsert.entity_id,
                "syncResult": sync_result
            }))
        }
        other => {
            let value = serde_json::to_value(&other)
                .map_err(|error| ApprovalError::Execution(error.to_string()))?;
            Err(ApprovalError::Execution(format!(
                "sync rejected agent action: {value}"
            )))
        }
    }
}

fn base_meta(
    principal: &AuthenticatedPrincipal,
    entity_id: &str,
    timestamp: DateTime<Utc>,
) -> Value {
    json!({
        "id": entity_id,
        "userId": principal.user_id.as_str(),
        "createdAt": timestamp,
        "updatedAt": timestamp,
        "deletedAt": null,
        "localVersion": 1,
        "serverVersion": null,
        "modifiedByDevice": null
    })
}

fn update_meta_for_server_edit(
    payload: &mut Value,
    timestamp: DateTime<Utc>,
) -> Result<(), ApprovalError> {
    let meta = payload
        .get_mut("meta")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ApprovalError::Invalid("entity payload has no meta object".to_owned()))?;
    meta.insert(
        "updatedAt".to_owned(),
        serde_json::to_value(timestamp).unwrap_or(Value::Null),
    );
    let next_local = meta
        .get("localVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(1);
    meta.insert("localVersion".to_owned(), json!(next_local));
    Ok(())
}

fn set_if_changed(object: &mut serde_json::Map<String, Value>, key: &str, value: Value) -> bool {
    if object.get(key) == Some(&value) {
        false
    } else {
        object.insert(key.to_owned(), value);
        true
    }
}

fn parse_uuid_field(value: &str, field: &str) -> Result<Uuid, ApprovalError> {
    Uuid::parse_str(value.trim())
        .map_err(|_| ApprovalError::Invalid(format!("{field} must be a UUID")))
}

fn mail_address_key(value: &str) -> String {
    let value = value.trim();
    if let (Some(start), Some(end)) = (value.rfind('<'), value.rfind('>')) {
        if start < end {
            return value[start + 1..end].trim().to_ascii_lowercase();
        }
    }
    value.to_ascii_lowercase()
}

fn collect_mail_addresses(value: &Value) -> Vec<String> {
    fn visit(value: &Value, output: &mut Vec<String>) {
        match value {
            Value::String(value) => {
                let value = value.trim();
                if value.contains('@') {
                    output.push(value.to_owned());
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit(value, output);
                }
            }
            Value::Object(object) => {
                let direct = object
                    .get("email")
                    .or_else(|| object.get("address"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| value.contains('@'));
                if let Some(address) = direct {
                    let display_name = object
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty());
                    output.push(match display_name {
                        Some(name) => format!("{name} <{address}>"),
                        None => address.to_owned(),
                    });
                } else {
                    for nested in object.values() {
                        visit(nested, output);
                    }
                }
            }
            _ => {}
        }
    }

    let mut values = Vec::new();
    visit(value, &mut values);
    let mut unique = Vec::new();
    for value in values {
        let key = mail_address_key(&value);
        if !unique
            .iter()
            .any(|existing: &String| mail_address_key(existing) == key)
        {
            unique.push(value);
        }
    }
    unique
}

fn normalize_mail_recipients(
    values: Vec<String>,
    field: &str,
    required: bool,
) -> Result<Vec<String>, ApprovalError> {
    if values.len() > 50 {
        return Err(ApprovalError::Invalid(format!(
            "{field} must not contain more than 50 recipients"
        )));
    }
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > 320 || !mail_address_key(value).contains('@') {
            return Err(ApprovalError::Invalid(format!(
                "{field} contains an invalid email address"
            )));
        }
        let key = mail_address_key(value);
        if !normalized
            .iter()
            .any(|existing: &String| mail_address_key(existing) == key)
        {
            normalized.push(value.to_owned());
        }
    }
    if required && normalized.is_empty() {
        return Err(ApprovalError::Invalid(format!(
            "{field} must contain at least one recipient"
        )));
    }
    Ok(normalized)
}

fn reply_mail_subject(subject: &str) -> String {
    let subject = subject.trim();
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject.to_owned()
    } else if subject.is_empty() {
        "Re: (无主题)".to_owned()
    } else {
        format!("Re: {subject}")
    }
}

async fn resolve_mail_sender(
    ctx: &AgentInvocationContext,
    account_id: &str,
    identity_id: Option<&str>,
) -> Result<(Uuid, Option<Uuid>, String), ApprovalError> {
    let account_id = parse_uuid_field(account_id, "accountId")?;
    resolve_mail_sender_for_account(ctx, account_id, identity_id).await
}

async fn resolve_mail_sender_for_account(
    ctx: &AgentInvocationContext,
    account_id: Uuid,
    identity_id: Option<&str>,
) -> Result<(Uuid, Option<Uuid>, String), ApprovalError> {
    let account_email: String = sqlx::query_scalar(
        "SELECT email_address FROM mail_accounts WHERE user_id=$1 AND id=$2 AND deleted_at IS NULL",
    )
    .bind(ctx.user_id)
    .bind(account_id)
    .fetch_optional(&ctx.pool)
    .await?
    .ok_or(ApprovalError::NotFound)?;

    if let Some(identity_id) = identity_id {
        let identity_id = parse_uuid_field(identity_id, "identityId")?;
        let email: String = sqlx::query_scalar(
            "SELECT email_address FROM mail_identities \
             WHERE user_id=$1 AND account_id=$2 AND id=$3 AND deleted_at IS NULL",
        )
        .bind(ctx.user_id)
        .bind(account_id)
        .bind(identity_id)
        .fetch_optional(&ctx.pool)
        .await?
        .ok_or(ApprovalError::NotFound)?;
        Ok((account_id, Some(identity_id), email))
    } else {
        let email: Option<String> = sqlx::query_scalar(
            "SELECT email_address FROM mail_identities \
             WHERE user_id=$1 AND account_id=$2 AND deleted_at IS NULL \
             ORDER BY is_default DESC,created_at ASC LIMIT 1",
        )
        .bind(ctx.user_id)
        .bind(account_id)
        .fetch_optional(&ctx.pool)
        .await?;
        Ok((account_id, None, email.unwrap_or(account_email)))
    }
}

async fn owned_mail_address_keys(
    ctx: &AgentInvocationContext,
    account_id: Uuid,
) -> Result<Vec<String>, ApprovalError> {
    let rows = sqlx::query(
        "SELECT email_address FROM mail_identities \
         WHERE user_id=$1 AND account_id=$2 AND deleted_at IS NULL \
         UNION SELECT email_address FROM mail_accounts \
         WHERE user_id=$1 AND id=$2 AND deleted_at IS NULL",
    )
    .bind(ctx.user_id)
    .bind(account_id)
    .fetch_all(&ctx.pool)
    .await?;
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        values.push(mail_address_key(
            &row.try_get::<String, _>("email_address")?,
        ));
    }
    Ok(values)
}

fn require_execution_write(ctx: &AgentInvocationContext) -> Result<(), ApprovalError> {
    require_context_write_scopes(ctx, &["sync:write", "execution:write"])
}

fn require_context_write_scopes(
    ctx: &AgentInvocationContext,
    required_scopes: &[&str],
) -> Result<(), ApprovalError> {
    for required in required_scopes {
        if !ctx.scopes.contains(*required) {
            return Err(ApprovalError::Permission((*required).to_owned()));
        }
    }
    Ok(())
}

fn require_principal_action_write(
    principal: &AuthenticatedPrincipal,
    action_name: &str,
) -> Result<(), ApprovalError> {
    let required_scopes: &[&str] = match action_name {
        "create_task"
        | "update_task"
        | "create_calendar_event"
        | "create_project"
        | "update_project"
        | "create_waiting_item"
        | "update_waiting_item"
        | "create_reminder"
        | "update_reminder" => &["sync:write", "execution:write"],
        "create_habit" | "update_habit" => &["sync:write", "habits:write"],
        "create_note" => &["sync:write", "notes:write"],
        "send_mail" | "reply_mail" => &["mail:write"],
        other => {
            return Err(ApprovalError::Invalid(format!(
                "unsupported action name: {other}"
            )))
        }
    };
    for required in required_scopes {
        if !principal.scopes.contains(*required) {
            return Err(ApprovalError::Permission((*required).to_owned()));
        }
    }
    Ok(())
}

fn normalize_target_days(mut values: Vec<u8>) -> Result<Vec<u8>, ApprovalError> {
    if values.iter().any(|value| !(1..=7).contains(value)) {
        return Err(ApprovalError::Invalid(
            "targetDays values must be between 1 and 7".to_owned(),
        ));
    }
    values.sort_unstable();
    values.dedup();
    Ok(values)
}

fn validate_habit_targets(
    minimum_target: Option<f64>,
    normal_target: f64,
) -> Result<(), ApprovalError> {
    if !normal_target.is_finite() || normal_target <= 0.0 {
        return Err(ApprovalError::Invalid(
            "normalTarget must be a finite value greater than zero".to_owned(),
        ));
    }
    if let Some(minimum_target) = minimum_target {
        if !minimum_target.is_finite() || minimum_target < 0.0 {
            return Err(ApprovalError::Invalid(
                "minimumTarget must be a finite non-negative value".to_owned(),
            ));
        }
        if minimum_target > normal_target {
            return Err(ApprovalError::Invalid(
                "minimumTarget must not exceed normalTarget".to_owned(),
            ));
        }
    }
    Ok(())
}

fn bounded_required(value: &str, field: &str, max: usize) -> Result<String, ApprovalError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApprovalError::Invalid(format!("{field} must not be empty")));
    }
    if value.chars().count() > max {
        return Err(ApprovalError::Invalid(format!(
            "{field} must not exceed {max} characters"
        )));
    }
    Ok(value.to_owned())
}

fn bounded_optional(value: Option<&str>, max: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max).collect())
}

fn validate_one_of(value: &str, field: &str, allowed: &[&str]) -> Result<(), ApprovalError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(ApprovalError::Invalid(format!(
            "{field} has an unsupported value"
        )))
    }
}

fn validate_optional_timestamp(value: Option<&str>, field: &str) -> Result<(), ApprovalError> {
    if let Some(value) = value {
        DateTime::parse_from_rfc3339(value)
            .map_err(|_| ApprovalError::Invalid(format!("{field} must be an RFC3339 timestamp")))?;
    }
    Ok(())
}

fn validate_optional_date(value: Option<&str>, field: &str) -> Result<(), ApprovalError> {
    if let Some(value) = value {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| ApprovalError::Invalid(format!("{field} must use YYYY-MM-DD")))?;
    }
    Ok(())
}

fn validate_range(
    start: &str,
    end: &str,
    start_field: &str,
    end_field: &str,
) -> Result<(), ApprovalError> {
    let start = DateTime::parse_from_rfc3339(start)
        .map_err(|_| ApprovalError::Invalid(format!("{start_field} is invalid")))?;
    let end = DateTime::parse_from_rfc3339(end)
        .map_err(|_| ApprovalError::Invalid(format!("{end_field} is invalid")))?;
    if end < start {
        return Err(ApprovalError::Invalid(format!(
            "{end_field} must not be earlier than {start_field}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_task_status_and_time_ranges() {
        assert!(validate_one_of("done", "status", &["todo", "done"]).is_ok());
        assert!(validate_one_of("unknown", "status", &["todo", "done"]).is_err());
        assert!(validate_range(
            "2026-09-29T10:00:00+08:00",
            "2026-09-29T11:00:00+08:00",
            "start",
            "end"
        )
        .is_ok());
        assert!(validate_range(
            "2026-09-29T12:00:00+08:00",
            "2026-09-29T11:00:00+08:00",
            "start",
            "end"
        )
        .is_err());
    }
}
