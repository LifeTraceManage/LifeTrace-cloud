use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lifetrace_cloud::agent::{context::AgentAccessPartition, session as agent_session};
use lifetrace_cloud::{app, AppState, Config};
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
        deepseek_api_key: None,
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
