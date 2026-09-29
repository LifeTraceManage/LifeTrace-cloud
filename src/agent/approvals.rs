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
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::agent::context::{AgentAccessPartition, AgentInvocationContext};
use crate::auth::AuthenticatedPrincipal;
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

pub struct ProposeCreateTaskTool;
pub struct ProposeUpdateTaskTool;
pub struct ProposeCreateCalendarEventTool;
pub struct ProposeCreateProjectTool;
pub struct ProposeUpdateProjectTool;
pub struct ProposeCreateHabitTool;
pub struct ProposeUpdateHabitTool;

impl Tool for ProposeCreateTaskTool {
    const NAME: &'static str = "lifetrace_propose_create_task";
    type Args = CreateTaskArgs;
    type Output = Value;
    type Error = ApprovalError;

    fn description(&self) -> String {
        "提出创建 LifeTrace 任务的写操作。该工具不会直接写入数据，只生成一个必须由用户显式批准的审批请求。时间信息不明确时先向用户确认，不要猜测。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "title":{"type":"string"},
                "description":{"type":"string"},
                "projectId":{"type":"string"},
                "priority":{"type":"string","enum":["low","normal","high","urgent"]},
                "dueAt":{"type":"string","description":"RFC3339 timestamp"},
                "scheduledStartAt":{"type":"string","description":"RFC3339 timestamp"},
                "scheduledEndAt":{"type":"string","description":"RFC3339 timestamp"},
                "timezone":{"type":"string","description":"IANA timezone, for example Asia/Shanghai"},
                "context":{"type":"string"}
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
        "提出修改已有 LifeTrace 任务的写操作。支持标题、状态、优先级和截止时间；不会直接执行，必须由用户显式批准。".to_owned()
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "taskId":{"type":"string"},
                "title":{"type":"string"},
                "status":{"type":"string","enum":["todo","in_progress","waiting","done","cancelled"]},
                "priority":{"type":"string","enum":["low","normal","high","urgent"]},
                "dueAt":{"type":"string","description":"RFC3339 timestamp"},
                "clearDueAt":{"type":"boolean","default":false}
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
            "clearDueAt": args.clear_due_at
        });
        propose(
            &ctx,
            Self::NAME,
            "update_task",
            arguments_json,
            action,
            json!({"taskId": task_id}),
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

        let name = bounded_required(&args.name, "name", 300)?;
        let activity_type = args.activity_type.unwrap_or_else(|| "completion".to_owned());
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
) -> Result<Value, ApprovalError> {
    let tool_call_id = Uuid::new_v4();
    let approval_id = Uuid::new_v4();
    let action_json = serde_json::to_string(&action).unwrap_or_else(|_| "{}".to_owned());
    let mut tx = ctx.pool.begin().await?;

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
    let items = sqlx::query_as::<_, AgentApproval>(
        "SELECT a.id,a.run_id,a.session_id,a.tool_call_id,a.action_name,a.action_json, \
                a.status,a.requested_at,a.decided_at,a.expires_at \
         FROM agent_approvals a \
         JOIN agent_sessions s ON s.id=a.session_id \
         WHERE a.user_id=$1 AND a.session_id=$2 AND s.user_id=$1 \
           AND s.app_id=$3 AND s.scopes_json=$4 \
         ORDER BY a.requested_at DESC,a.rowid DESC LIMIT $5",
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
            require_principal_execution_write(principal)?;
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
                a.status,a.requested_at,a.decided_at,a.expires_at \
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
        other => Err(ApprovalError::Invalid(format!(
            "unsupported action name: {other}"
        ))),
    }
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
        .ok_or_else(|| ApprovalError::Invalid("task payload has no meta object".to_owned()))?;
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

fn require_execution_write(ctx: &AgentInvocationContext) -> Result<(), ApprovalError> {
    for required in ["sync:write", "execution:write"] {
        if !ctx.scopes.contains(required) {
            return Err(ApprovalError::Permission(required.to_owned()));
        }
    }
    Ok(())
}

fn require_principal_execution_write(
    principal: &AuthenticatedPrincipal,
) -> Result<(), ApprovalError> {
    for required in ["sync:write", "execution:write"] {
        if !principal.scopes.contains(required) {
            return Err(ApprovalError::Permission(required.to_owned()));
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
