use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use lifetrace_cloud::agent::{context::AgentAccessPartition, session as agent_session};
use lifetrace_cloud::{app, AppState, Config};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const TOKEN: &str = "agent-test-token";

async fn test_state_and_app() -> (AppState, Router) {
    let config = Config {
        database_path: ":memory:".to_owned(),
        dev_auth_token: TOKEN.to_owned(),
        dev_auth_user_id: "agent-test-user".to_owned(),
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
    assert_eq!(body["code"], "invalid_request");
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
    assert_eq!(body["code"], "invalid_request");
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
    let user_id_raw: String = sqlx::query_scalar("SELECT user_id FROM agent_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    let user_id = Uuid::parse_str(&user_id_raw).unwrap();

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
