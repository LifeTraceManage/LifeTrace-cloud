//! PostgreSQL-backed EPIC-12 file service acceptance tests.

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use lifetrace_cloud::auth::security::RequestContext;
use lifetrace_cloud::{app, AppState, Config};
use lifetrace_contracts::auth::v1::{LoginRequestV1, RegisterRequestV1, Scope};
use lifetrace_contracts::sync::v1::AppId;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

fn config(url: String) -> Config {
    Config {
        database_url: Some(url),
        migration_on_startup: true,
        dev_auth_enabled: false,
        auth_registration_mode: "open".to_owned(),
        auth_password_pepper: Some("file-test-password-pepper-012345678901234".to_owned()),
        auth_token_hash_pepper: Some("file-test-token-pepper-012345678901234567".to_owned()),
        cursor_signing_key: Some("file-test-cursor-signing-key".to_owned()),
        page_token_signing_key: Some("file-test-page-token-key".to_owned()),
        public_web_base_url: Some("http://localhost:3000".to_owned()),
        cors_allowed_origins: vec!["http://localhost:3000".to_owned()],
        ..Config::default()
    }
}

async fn state() -> Option<AppState> {
    let url = database_url()?;
    let state = AppState::new(config(url));
    state.initialize().await.unwrap();
    Some(state)
}

fn context() -> RequestContext {
    RequestContext {
        ip: Some("127.0.0.1".parse().unwrap()),
        user_agent: Some("LifeTrace file integration test".to_owned()),
        origin: Some("http://localhost:3000".to_owned()),
    }
}

const TEST_PASSWORD: &str = "正确 horse battery staple 文件密码";

fn registration() -> RegisterRequestV1 {
    registration_for(
        format!("file-{}@example.test", Uuid::new_v4()),
        AppId::DESKTOP,
        vec![Scope::new("files:read"), Scope::new("files:write")],
    )
}

fn registration_for(
    email: String,
    app_id: &str,
    requested_scopes: Vec<Scope>,
) -> RegisterRequestV1 {
    RegisterRequestV1 {
        email,
        password: TEST_PASSWORD.to_owned(),
        display_name: Some("File Test".to_owned()),
        invite_token: None,
        app_id: AppId::new(app_id),
        device_id: Uuid::new_v4().to_string(),
        device_name: "File Integration Device".to_owned(),
        platform: "windows".to_owned(),
        client_version: Some("0.3.1".to_owned()),
        requested_scopes,
    }
}

fn login_for(email: &str, app_id: &str, requested_scopes: Vec<Scope>) -> LoginRequestV1 {
    LoginRequestV1 {
        email: email.to_owned(),
        password: TEST_PASSWORD.to_owned(),
        app_id: AppId::new(app_id),
        device_id: Uuid::new_v4().to_string(),
        device_name: "File Integration Device".to_owned(),
        platform: "android".to_owned(),
        client_version: Some("0.3.1".to_owned()),
        requested_scopes,
        public_device: false,
    }
}

fn configure_object_storage() {
    std::env::set_var(
        "FILE_OBJECT_STORAGE_ENDPOINT",
        "https://storage.example.com",
    );
    std::env::set_var("FILE_OBJECT_STORAGE_BUCKET", "lifetrace-files");
    std::env::set_var("FILE_OBJECT_STORAGE_REGION", "us-east-1");
    std::env::set_var("FILE_OBJECT_STORAGE_ACCESS_KEY_ID", "AKIDFILETEST");
    std::env::set_var(
        "FILE_OBJECT_STORAGE_SECRET_ACCESS_KEY",
        "file-test-secret-key",
    );
    std::env::set_var("FILE_OBJECT_STORAGE_PRESIGN_TTL_SECONDS", "300");
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn prepare_deduplicates_and_signed_headers_are_browser_safe() {
    let Some(state) = state().await else {
        return;
    };
    configure_object_storage();
    let tokens = state
        .auth_service
        .register(registration(), &context())
        .await
        .unwrap();
    let bearer = format!("Bearer {}", tokens.access_token);
    let router = app(state);
    let payload = json!({
        "domain": "notes_attachments",
        "originalName": "diagram.png",
        "mimeType": "image/png",
        "sizeBytes": 1024,
        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "entityType": "note.note",
        "entityId": format!("note-{}", Uuid::new_v4())
    });

    let first = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", &bearer)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body = json_body(first).await;
    assert_eq!(first_body["deduplicated"], false);
    assert_eq!(first_body["file"]["status"], "pending");
    assert!(first_body["upload"]["url"]
        .as_str()
        .unwrap()
        .contains("X-Amz-Signature="));
    assert!(
        first_body["upload"]["requiredHeaders"]["x-amz-checksum-sha256"]
            .as_str()
            .is_some()
    );
    assert!(first_body["upload"]["requiredHeaders"]["host"].is_null());

    let second = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", bearer)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = json_body(second).await;
    assert_eq!(second_body["deduplicated"], true);
    assert_eq!(second_body["file"]["id"], first_body["file"]["id"]);
}

#[tokio::test]
async fn file_id_cannot_cross_user_boundary() {
    let Some(state) = state().await else {
        return;
    };
    configure_object_storage();
    let owner = state
        .auth_service
        .register(registration(), &context())
        .await
        .unwrap();
    let stranger = state
        .auth_service
        .register(registration(), &context())
        .await
        .unwrap();
    let router = app(state);

    let created = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", owner.access_token))
                .body(Body::from(
                    json!({
                        "domain": "finance_imports",
                        "originalName": "statement.csv",
                        "mimeType": "text/csv",
                        "sizeBytes": 128,
                        "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = json_body(created).await;
    let id = body["file"]["id"].as_str().unwrap();

    let hidden = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/files/{id}"))
                .header("authorization", format!("Bearer {}", stranger.access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
}


#[tokio::test]
async fn assets_client_can_prepare_only_assets_attachment_domain() {
    let Some(state) = state().await else {
        return;
    };
    configure_object_storage();
    let email = format!("assets-file-{}@example.test", Uuid::new_v4());
    let tokens = state
        .auth_service
        .register(
            registration_for(
                email,
                AppId::ASSETS,
                vec![Scope::new("files:read"), Scope::new("files:write")],
            ),
            &context(),
        )
        .await
        .unwrap();
    assert!(tokens.scopes.iter().any(|scope| scope.as_str() == "files:read"));
    assert!(tokens.scopes.iter().any(|scope| scope.as_str() == "files:write"));

    let bearer = format!("Bearer {}", tokens.access_token);
    let router = app(state);
    let allowed = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", &bearer)
                .body(Body::from(
                    json!({
                        "domain": "assets_attachments",
                        "originalName": "asset.jpg",
                        "mimeType": "image/jpeg",
                        "sizeBytes": 128,
                        "sha256": "1111111111111111111111111111111111111111111111111111111111111111",
                        "entityType": "asset.asset",
                        "entityId": "asset-test-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(allowed.status(), StatusCode::CREATED);

    let cross_domain = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", &bearer)
                .body(Body::from(
                    json!({
                        "domain": "notes_attachments",
                        "originalName": "note.txt",
                        "mimeType": "text/plain",
                        "sizeBytes": 16,
                        "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
                        "entityType": "note.note",
                        "entityId": "note-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cross_domain.status(), StatusCode::FORBIDDEN);

    let invalid_owner = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", bearer)
                .body(Body::from(
                    json!({
                        "domain": "assets_attachments",
                        "originalName": "wrong.jpg",
                        "mimeType": "image/jpeg",
                        "sizeBytes": 64,
                        "sha256": "3333333333333333333333333333333333333333333333333333333333333333",
                        "entityType": "note.note",
                        "entityId": "note-2"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid_owner.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn assets_client_cannot_access_another_domain_by_file_id() {
    let Some(state) = state().await else {
        return;
    };
    configure_object_storage();
    let email = format!("assets-cross-domain-{}@example.test", Uuid::new_v4());
    let desktop = state
        .auth_service
        .register(
            registration_for(
                email.clone(),
                AppId::DESKTOP,
                vec![Scope::new("files:read"), Scope::new("files:write")],
            ),
            &context(),
        )
        .await
        .unwrap();
    let assets = state
        .auth_service
        .login(
            login_for(
                &email,
                AppId::ASSETS,
                vec![Scope::new("files:read"), Scope::new("files:write")],
            ),
            &context(),
        )
        .await
        .unwrap();

    let router = app(state);
    let created = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", desktop.access_token))
                .body(Body::from(
                    json!({
                        "domain": "notes_attachments",
                        "originalName": "private.txt",
                        "mimeType": "text/plain",
                        "sizeBytes": 32,
                        "sha256": "4444444444444444444444444444444444444444444444444444444444444444",
                        "entityType": "note.note",
                        "entityId": "note-cross-domain"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = json_body(created).await;
    let id = body["file"]["id"].as_str().unwrap();
    let assets_bearer = format!("Bearer {}", assets.access_token);

    for (method, uri) in [
        (Method::GET, format!("/api/v1/files/{id}")),
        (
            Method::POST,
            format!("/api/v1/files/{id}/download-url"),
        ),
        (
            Method::POST,
            format!("/api/v1/files/{id}/upload-url"),
        ),
        (Method::DELETE, format!("/api/v1/files/{id}")),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("authorization", &assets_bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[tokio::test]
async fn assets_domain_still_requires_generic_file_scope() {
    let Some(state) = state().await else {
        return;
    };
    configure_object_storage();
    let email = format!("assets-no-file-scope-{}@example.test", Uuid::new_v4());
    let tokens = state
        .auth_service
        .register(
            registration_for(
                email,
                AppId::ASSETS,
                vec![Scope::new("assets:read"), Scope::new("assets:write")],
            ),
            &context(),
        )
        .await
        .unwrap();
    let router = app(state);

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/files")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", tokens.access_token))
                .body(Body::from(
                    json!({
                        "domain": "assets_attachments",
                        "originalName": "no-scope.jpg",
                        "mimeType": "image/jpeg",
                        "sizeBytes": 12,
                        "sha256": "5555555555555555555555555555555555555555555555555555555555555555",
                        "entityType": "asset.asset",
                        "entityId": "asset-no-file-scope"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
