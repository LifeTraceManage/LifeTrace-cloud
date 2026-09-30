use std::collections::BTreeSet;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lifetrace_cloud::agent::{
    approvals::{
        CreateProjectArgs, CreateTaskArgs, ProposeCreateProjectTool, ProposeCreateTaskTool,
    },
    context::{AgentAccessPartition, AgentInvocationContext},
    session as agent_session,
};
use lifetrace_cloud::{app, AppState, Config};
use rig::tool::{Tool, ToolContext};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "agent-test-token";
const USER_ID: &str = "11111111-1111-4111-8111-111111111111";

async fn test_state_and_app() -> (AppState, Router) {
    let config = Config {
        database_path: ":memory:".to_owned(),
        dev_auth_token: TOKEN.to_owned(),
        dev_auth_user_id: USER_ID.to_owned(),
        dev_auth_device_id: "agent-test-device".to_owned(),
        model_api_key: None,
        ..Config::default()
    };
    let state = AppState::new(config);
    state.initialize().await.unwrap();
    let router = app(state.clone());
    (state, router)
}

async fn test_app() -> Router {
    test_state_and_app().await.1
}

async fn send(app: Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn seed_approval(
    state: &AppState,
    session_id: Uuid,
    run_id: Uuid,
    action_name: &str,
    action: Value,
) -> Uuid {
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM agent_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let tool_call_id = Uuid::new_v4();
    let approval_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO agent_tool_calls \
         (id,run_id,session_id,user_id,tool_name,arguments_json,status,requires_approval) \
         VALUES ($1,$2,$3,$4,'test_write_proposal','{}','awaiting_approval',1)",
    )
    .bind(tool_call_id)
    .bind(run_id)
    .bind(session_id)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO agent_approvals \
         (id,run_id,session_id,user_id,tool_call_id,action_name,action_json,status,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,'pending',datetime('now','+15 minutes'))",
    )
    .bind(approval_id)
    .bind(run_id)
    .bind(session_id)
    .bind(user_id)
    .bind(tool_call_id)
    .bind(action_name)
    .bind(action.to_string())
    .execute(&state.pool)
    .await
    .unwrap();

    approval_id
}

#[tokio::test]
async fn assistant_creates_and_reuses_persisted_session() {
    let app = test_app().await;

    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"总结一下我的 LifeTrace 数据"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["provider"], "local");
    assert!(first["reply"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));

    let session_id = first["sessionId"].as_str().unwrap().to_owned();
    let first_run_id = first["runId"].as_str().unwrap().to_owned();

    let (status, second) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({
            "prompt":"继续说说下一步",
            "sessionId": session_id
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["sessionId"], first["sessionId"]);
    assert_ne!(second["runId"], first_run_id);

    let (status, sessions) = send(
        app.clone(),
        Method::GET,
        "/api/v1/assistant/sessions?limit=10",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sessions["items"].as_array().unwrap().len(), 1);
    assert_eq!(sessions["items"][0]["id"], first["sessionId"]);

    let messages_uri = format!(
        "/api/v1/assistant/sessions/{}/messages?limit=20",
        first["sessionId"].as_str().unwrap()
    );
    let (status, messages) = send(app, Method::GET, &messages_uri, Value::Null).await;
    assert_eq!(status, StatusCode::OK);

    let items = messages["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0]["role"], "user");
    assert_eq!(items[1]["role"], "assistant");
    assert_eq!(items[2]["role"], "user");
    assert_eq!(items[3]["role"], "assistant");
}

#[tokio::test]
async fn assistant_rejects_unknown_session_without_creating_a_run() {
    let app = test_app().await;
    let (status, body) = send(
        app,
        Method::POST,
        "/api/v1/assistant",
        json!({
            "prompt":"继续",
            "sessionId":"00000000-0000-0000-0000-000000000001"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "LIFETRACE_INVALID_REQUEST");
}

#[tokio::test]
async fn assistant_rejects_oversized_prompt() {
    let app = test_app().await;
    let (status, body) = send(
        app,
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"x".repeat(4_001)}),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "LIFETRACE_INVALID_REQUEST");
}

#[tokio::test]
async fn assistant_history_is_partitioned_by_app_and_scopes() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app,
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"读取我的数据"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM agent_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    let wrong_scope_partition = AgentAccessPartition {
        app_id: "lifetrace-desktop".to_owned(),
        scopes_json: "[]".to_owned(),
    };

    let sessions = agent_session::list_sessions(&state.pool, user_id, &wrong_scope_partition, 10)
        .await
        .unwrap();
    assert!(sessions.is_empty());

    let messages =
        agent_session::list_messages(&state.pool, user_id, &wrong_scope_partition, session_id, 20)
            .await
            .unwrap();
    assert!(messages.is_empty());

    let resumed = agent_session::ensure_session(
        &state.pool,
        user_id,
        &wrong_scope_partition,
        Some(session_id),
        "继续",
    )
    .await;
    assert!(matches!(resumed, Err(sqlx::Error::RowNotFound)));
}

#[tokio::test]
async fn assistant_privacy_export_contains_current_access_partition() {
    let (_state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"给我一个数据概览"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, exported) = send(
        app,
        Method::GET,
        "/api/v1/privacy/export/agent",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let agent = &exported["sections"]["agent"];
    assert_eq!(agent["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(agent["messages"].as_array().unwrap().len(), 2);
    assert_eq!(agent["runs"].as_array().unwrap().len(), 1);
    let exported_session_id = first["sessionId"].as_str().unwrap().replace('-', "");
    assert_eq!(agent["sessions"][0]["id"], exported_session_id);
}

#[tokio::test]
async fn assistant_approval_executes_task_once_and_is_idempotent() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备创建一个测试任务"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let task_id = Uuid::new_v4().to_string();
    let approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": task_id,
            "title": "Agent approval task",
            "description": null,
            "projectId": null,
            "priority": "high",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "Asia/Singapore",
            "context": "agent-test"
        }),
    )
    .await;

    let approvals_uri = format!("/api/v1/assistant/sessions/{session_id}/approvals?limit=10");
    let (status, listed) = send(app.clone(), Method::GET, &approvals_uri, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    assert_eq!(listed["items"][0]["status"], "pending");

    let decision_uri = format!("/api/v1/assistant/approvals/{approval_id}/decision");
    let (status, approved) = send(
        app.clone(),
        Method::POST,
        &decision_uri,
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved["approval"]["status"], "approved");
    assert_eq!(approved["result"]["entityType"], "execution.task");
    assert_eq!(approved["result"]["entityId"], task_id);

    let stored: (i64, String) = sqlx::query_as(
        "SELECT COUNT(*),MAX(payload->>'title') FROM sync_entities \
         WHERE entity_type='execution.task' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&task_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(stored.0, 1);
    assert_eq!(stored.1, "Agent approval task");

    let (status, approved_again) = send(
        app,
        Method::POST,
        &decision_uri,
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved_again["approval"]["status"], "approved");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sync_entities \
         WHERE entity_type='execution.task' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&task_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn assistant_rejected_approval_never_writes_entity() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备创建一条稍后拒绝的任务"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let task_id = Uuid::new_v4().to_string();
    let approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": task_id,
            "title": "Must not be created",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "UTC",
            "context": null
        }),
    )
    .await;

    let decision_uri = format!("/api/v1/assistant/approvals/{approval_id}/decision");
    let (status, rejected) = send(
        app,
        Method::POST,
        &decision_uri,
        json!({"decision":"reject"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rejected["approval"]["status"], "rejected");
    assert!(rejected["result"].is_null());

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sync_entities \
         WHERE entity_type='execution.task' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&task_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn assistant_approval_creates_and_updates_project_and_habit() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备测试 Project 和习惯审批"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();

    let project_id = Uuid::new_v4().to_string();
    let create_project = seed_approval(
        &state,
        session_id,
        run_id,
        "create_project",
        json!({
            "entityId": project_id,
            "name": "Agent Project",
            "description": "created by approval test",
            "color": "#49715d",
            "icon": "target"
        }),
    )
    .await;
    let (status, created_project) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{create_project}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(created_project["result"]["entityType"], "execution.project");
    assert_eq!(created_project["result"]["entityId"], project_id);

    let project: (String, String) = sqlx::query_as(
        "SELECT payload->>'name',payload->>'status' FROM sync_entities \
         WHERE entity_type='execution.project' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&project_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(project.0, "Agent Project");
    assert_eq!(project.1, "active");

    let update_project = seed_approval(
        &state,
        session_id,
        run_id,
        "update_project",
        json!({
            "projectId": project_id,
            "name": "Agent Project Updated",
            "description": null,
            "clearDescription": true,
            "status": "archived",
            "color": null,
            "icon": null
        }),
    )
    .await;
    let (status, updated_project) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{update_project}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_project["approval"]["status"], "approved");

    let project: (String, String, Option<String>) = sqlx::query_as(
        "SELECT payload->>'name',payload->>'status',payload->>'description' \
         FROM sync_entities WHERE entity_type='execution.project' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&project_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(project.0, "Agent Project Updated");
    assert_eq!(project.1, "archived");
    assert_eq!(project.2, None);

    let habit_id = Uuid::new_v4().to_string();
    let create_habit = seed_approval(
        &state,
        session_id,
        run_id,
        "create_habit",
        json!({
            "entityId": habit_id,
            "name": "阅读",
            "activityType": "duration",
            "unit": "分钟",
            "minimumTarget": 10.0,
            "normalTarget": 30.0,
            "targetDays": [],
            "scheduleType": "daily",
            "startDate": "2026-09-29",
            "description": "每天阅读"
        }),
    )
    .await;
    let (status, created_habit) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{create_habit}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(created_habit["result"]["entityType"], "habit.activity");
    assert_eq!(created_habit["result"]["entityId"], habit_id);

    let habit: (String, f64, String) = sqlx::query_as(
        "SELECT payload->>'name',json_extract(payload,'$.normalTarget'),payload->>'scheduleType' \
         FROM sync_entities WHERE entity_type='habit.activity' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&habit_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(habit.0, "阅读");
    assert_eq!(habit.1, 30.0);
    assert_eq!(habit.2, "daily");

    let update_habit = seed_approval(
        &state,
        session_id,
        run_id,
        "update_habit",
        json!({
            "habitId": habit_id,
            "name": "深度阅读",
            "minimumTarget": 15.0,
            "clearMinimumTarget": false,
            "normalTarget": 45.0,
            "targetDays": [1,3,5],
            "scheduleType": "custom",
            "startDate": null,
            "description": null,
            "clearDescription": true,
            "isArchived": false
        }),
    )
    .await;
    let (status, updated_habit) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{update_habit}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated_habit["approval"]["status"], "approved");

    let habit: (String, f64, f64, String, String, Option<String>) = sqlx::query_as(
        "SELECT payload->>'name',json_extract(payload,'$.minimumTarget'), \
                json_extract(payload,'$.normalTarget'),payload->>'scheduleType', \
                json_extract(payload,'$.targetDays'),payload->>'description' \
         FROM sync_entities WHERE entity_type='habit.activity' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&habit_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(habit.0, "深度阅读");
    assert_eq!(habit.1, 15.0);
    assert_eq!(habit.2, 45.0);
    assert_eq!(habit.3, "custom");
    assert_eq!(habit.4, "[1,3,5]");
    assert_eq!(habit.5, None);
}

#[tokio::test]
async fn assistant_approval_manages_waiting_item_and_reminder() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备测试 Waiting Item 和 Reminder 审批"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();

    let waiting_id = Uuid::new_v4().to_string();
    let create_waiting = seed_approval(
        &state,
        session_id,
        run_id,
        "create_waiting_item",
        json!({
            "entityId": waiting_id,
            "title": "等待导师反馈",
            "description": "论文计划确认",
            "waitingFor": "导师",
            "expectedAt": "2026-10-02T12:00:00+08:00",
            "followUpAt": "2026-10-03T09:00:00+08:00",
            "sourceTaskId": null
        }),
    )
    .await;
    let (status, waiting_created) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{create_waiting}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        waiting_created["result"]["entityType"],
        "execution.waiting_item"
    );

    let update_waiting = seed_approval(
        &state,
        session_id,
        run_id,
        "update_waiting_item",
        json!({
            "waitingItemId": waiting_id,
            "title": null,
            "description": null,
            "clearDescription": false,
            "waitingFor": null,
            "expectedAt": null,
            "clearExpectedAt": false,
            "followUpAt": null,
            "clearFollowUpAt": true,
            "status": "resolved",
            "resolutionSummary": "导师已确认",
            "clearResolutionSummary": false
        }),
    )
    .await;
    let (status, waiting_updated) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{update_waiting}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(waiting_updated["approval"]["status"], "approved");

    let waiting: (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT payload->>'status',payload->>'followUpAt',payload->>'resolutionSummary' \
         FROM sync_entities WHERE entity_type='execution.waiting_item' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&waiting_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(waiting.0, "resolved");
    assert_eq!(waiting.1, None);
    assert_eq!(waiting.2.as_deref(), Some("导师已确认"));

    let reminder_id = Uuid::new_v4().to_string();
    let create_reminder = seed_approval(
        &state,
        session_id,
        run_id,
        "create_reminder",
        json!({
            "entityId": reminder_id,
            "subjectType": "waiting_item",
            "subjectId": waiting_id,
            "triggerAt": "2026-10-01T09:00:00+08:00",
            "title": "检查论文计划",
            "body": "确认是否需要继续跟进"
        }),
    )
    .await;
    let (status, reminder_created) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{create_reminder}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        reminder_created["result"]["entityType"],
        "execution.reminder"
    );

    let reminder: (String, String, String) = sqlx::query_as(
        "SELECT payload->>'subjectType',payload->>'subjectId',payload->>'status' \
         FROM sync_entities WHERE entity_type='execution.reminder' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&reminder_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(reminder.0, "waiting_item");
    assert_eq!(reminder.1, waiting_id);
    assert_eq!(reminder.2, "scheduled");

    let update_reminder = seed_approval(
        &state,
        session_id,
        run_id,
        "update_reminder",
        json!({
            "reminderId": reminder_id,
            "triggerAt": "2026-10-01T10:00:00+08:00",
            "snoozedUntil": null,
            "clearSnooze": false,
            "status": "dismissed",
            "title": "论文计划已处理",
            "clearTitle": false,
            "body": null,
            "clearBody": true
        }),
    )
    .await;
    let (status, reminder_updated) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{update_reminder}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reminder_updated["approval"]["status"], "approved");

    let reminder: (String, String, Option<String>, String) = sqlx::query_as(
        "SELECT payload->>'status',payload->>'title',payload->>'body',payload->>'fireKey' \
         FROM sync_entities WHERE entity_type='execution.reminder' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&reminder_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(reminder.0, "dismissed");
    assert_eq!(reminder.1, "论文计划已处理");
    assert_eq!(reminder.2, None);
    assert!(reminder.3.contains("2026-10-01T10:00:00+08:00"));
}

#[tokio::test]
async fn assistant_reminder_approval_rejects_removed_memo_subject() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"测试已经移除的 Memo 提醒类型"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_reminder",
        json!({
            "entityId": Uuid::new_v4().to_string(),
            "subjectType": "memo",
            "subjectId": Uuid::new_v4().to_string(),
            "triggerAt": "2026-10-01T09:00:00+08:00",
            "title": null,
            "body": null
        }),
    )
    .await;

    let (status, _) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{approval_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sync_entities WHERE entity_type='execution.reminder'",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn assistant_reminder_approval_rejects_missing_subject() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"测试不存在目标的提醒"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_reminder",
        json!({
            "entityId": Uuid::new_v4().to_string(),
            "subjectType": "task",
            "subjectId": Uuid::new_v4().to_string(),
            "triggerAt": "2026-10-01T09:00:00+08:00",
            "title": null,
            "body": null
        }),
    )
    .await;

    let (status, _) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{approval_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sync_entities WHERE entity_type='execution.reminder'",
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn assistant_can_schedule_and_reopen_cancelled_task_in_planner() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"测试把取消任务重新安排进 Planner"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let task_id = Uuid::new_v4().to_string();

    let create_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": task_id,
            "title": "Planner scheduling task",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": "2026-09-30T17:00:00+08:00",
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "Asia/Singapore",
            "context": "inbox"
        }),
    )
    .await;
    let (status, _) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{create_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let cancel_id = seed_approval(
        &state,
        session_id,
        run_id,
        "update_task",
        json!({
            "taskId": task_id,
            "title": null,
            "status": "cancelled",
            "priority": null,
            "dueAt": null,
            "clearDueAt": false,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "clearSchedule": false,
            "estimatedMinutes": null
        }),
    )
    .await;
    let (status, _) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{cancel_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let schedule_id = seed_approval(
        &state,
        session_id,
        run_id,
        "update_task",
        json!({
            "taskId": task_id,
            "title": null,
            "status": "todo",
            "priority": null,
            "dueAt": null,
            "clearDueAt": false,
            "scheduledStartAt": "2026-09-30T09:00:00+08:00",
            "scheduledEndAt": "2026-09-30T10:00:00+08:00",
            "clearSchedule": false,
            "estimatedMinutes": 60
        }),
    )
    .await;
    let (status, scheduled) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{schedule_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scheduled["approval"]["status"], "approved");

    let stored: (String, Option<String>, String, String, Option<String>, i64) = sqlx::query_as(
        "SELECT payload->>'status',payload->>'cancelledAt',payload->>'scheduledStartAt', \
                payload->>'scheduledEndAt',payload->>'context', \
                json_extract(payload,'$.estimatedMinutes') \
         FROM sync_entities WHERE entity_type='execution.task' AND entity_id=$1 AND is_deleted=0",
    )
    .bind(&task_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(stored.0, "todo");
    assert_eq!(stored.1, None);
    assert_eq!(stored.2, "2026-09-30T09:00:00+08:00");
    assert_eq!(stored.3, "2026-09-30T10:00:00+08:00");
    assert_eq!(stored.4, None);
    assert_eq!(stored.5, 60);
}

#[tokio::test]
async fn assistant_session_delete_cascades_conversation_records() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"创建一个随后删除的会话"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let _approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": "pending approval removed with session",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "UTC",
            "context": null
        }),
    )
    .await;

    let (status, _) = send(
        app,
        Method::DELETE,
        &format!("/api/v1/assistant/sessions/{session_id}"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    for table in [
        "agent_sessions",
        "agent_runs",
        "agent_messages",
        "agent_tool_calls",
        "agent_approvals",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE session_id=$1");
        let count: i64 = if table == "agent_sessions" {
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_sessions WHERE id=$1")
                .bind(session_id)
                .fetch_one(&state.pool)
                .await
                .unwrap()
        } else {
            sqlx::query_scalar(&sql)
                .bind(session_id)
                .fetch_one(&state.pool)
                .await
                .unwrap()
        };
        assert_eq!(count, 0, "{table} should be deleted with the session");
    }
}

#[tokio::test]
async fn assistant_approval_listing_expires_stale_pending_cards() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"测试过期审批自动收起"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": "expired approval",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "UTC",
            "context": null
        }),
    )
    .await;

    sqlx::query("UPDATE agent_approvals SET expires_at=datetime('now','-1 minute') WHERE id=$1")
        .bind(approval_id)
        .execute(&state.pool)
        .await
        .unwrap();

    let (status, listed) = send(
        app,
        Method::GET,
        &format!("/api/v1/assistant/sessions/{session_id}/approvals?limit=20"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let item = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == approval_id.to_string())
        .unwrap();
    assert_eq!(item["status"], "expired");

    let tool_status: String = sqlx::query_scalar(
        "SELECT t.status FROM agent_tool_calls t \
         JOIN agent_approvals a ON a.tool_call_id=t.id WHERE a.id=$1",
    )
    .bind(approval_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(tool_status, "denied");
}

#[tokio::test]
async fn assistant_new_proposal_supersedes_old_pending_approval_atomically() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app.clone(),
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备一个稍后修改的任务审批"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM agent_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    let old_task_id = Uuid::new_v4().to_string();
    let old_approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": old_task_id,
            "title": "旧方案任务",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "Asia/Singapore",
            "context": null
        }),
    )
    .await;

    let scopes = BTreeSet::from(["sync:write".to_owned(), "execution:write".to_owned()]);
    let mut tool_context = ToolContext::new();
    tool_context.insert(AgentInvocationContext {
        pool: state.pool.clone(),
        user_id,
        scopes: Arc::new(scopes),
        run_id,
        session_id,
    });

    let result = ProposeCreateTaskTool
        .call(
            &mut tool_context,
            CreateTaskArgs {
                title: "修改后的任务".to_owned(),
                description: Some("这是替代版本".to_owned()),
                project_id: None,
                priority: Some("high".to_owned()),
                due_at: None,
                scheduled_start_at: None,
                scheduled_end_at: None,
                timezone: Some("Asia/Singapore".to_owned()),
                context: None,
                leave_unscheduled: false,
                supersedes_approval_id: Some(old_approval_id.to_string()),
            },
        )
        .await
        .unwrap();

    let new_approval_id =
        Uuid::parse_str(result["approvalId"].as_str().expect("new approval id")).unwrap();
    assert_eq!(result["supersedesApprovalId"], old_approval_id.to_string());

    let (status, listed) = send(
        app.clone(),
        Method::GET,
        &format!("/api/v1/assistant/sessions/{session_id}/approvals?limit=20"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = listed["items"].as_array().unwrap();
    let old = items
        .iter()
        .find(|item| item["id"] == old_approval_id.to_string())
        .unwrap();
    let new = items
        .iter()
        .find(|item| item["id"] == new_approval_id.to_string())
        .unwrap();
    assert_eq!(old["status"], "cancelled");
    assert_eq!(old["cancellationReason"], "superseded");
    assert_eq!(old["supersededByApprovalId"], new_approval_id.to_string());
    assert_eq!(new["status"], "pending");

    let old_tool_status: String = sqlx::query_scalar(
        "SELECT t.status FROM agent_tool_calls t \
         JOIN agent_approvals a ON a.tool_call_id=t.id WHERE a.id=$1",
    )
    .bind(old_approval_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(old_tool_status, "denied");

    let (status, _) = send(
        app.clone(),
        Method::POST,
        &format!("/api/v1/assistant/approvals/{old_approval_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, approved) = send(
        app,
        Method::POST,
        &format!("/api/v1/assistant/approvals/{new_approval_id}/decision"),
        json!({"decision":"approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(approved["approval"]["status"], "approved");

    let tasks: Vec<String> = sqlx::query_scalar(
        "SELECT payload->>'title' FROM sync_entities \
         WHERE entity_type='execution.task' AND is_deleted=0 ORDER BY payload->>'title'",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap();
    assert_eq!(tasks, vec!["修改后的任务".to_owned()]);
}

#[tokio::test]
async fn assistant_cannot_supersede_pending_approval_with_different_action_type() {
    let (state, app) = test_state_and_app().await;
    let (status, first) = send(
        app,
        Method::POST,
        "/api/v1/assistant",
        json!({"prompt":"准备测试不同审批类型不能互相替代"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let session_id = Uuid::parse_str(first["sessionId"].as_str().unwrap()).unwrap();
    let run_id = Uuid::parse_str(first["runId"].as_str().unwrap()).unwrap();
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM agent_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    let old_approval_id = seed_approval(
        &state,
        session_id,
        run_id,
        "create_task",
        json!({
            "entityId": Uuid::new_v4().to_string(),
            "title": "不能被 Project 替代",
            "description": null,
            "projectId": null,
            "priority": "normal",
            "dueAt": null,
            "scheduledStartAt": null,
            "scheduledEndAt": null,
            "timezone": "UTC",
            "context": null
        }),
    )
    .await;

    let scopes = BTreeSet::from(["sync:write".to_owned(), "execution:write".to_owned()]);
    let mut tool_context = ToolContext::new();
    tool_context.insert(AgentInvocationContext {
        pool: state.pool.clone(),
        user_id,
        scopes: Arc::new(scopes),
        run_id,
        session_id,
    });

    let error = ProposeCreateProjectTool
        .call(
            &mut tool_context,
            CreateProjectArgs {
                name: "错误替代 Project".to_owned(),
                description: None,
                color: None,
                icon: None,
                supersedes_approval_id: Some(old_approval_id.to_string()),
            },
        )
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("cannot replace create_task approval"));

    let status: String = sqlx::query_scalar("SELECT status FROM agent_approvals WHERE id=$1")
        .bind(old_approval_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_approvals WHERE session_id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}
