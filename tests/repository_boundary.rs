#[test]
fn cloud_repository_owns_execution_and_auth_server_contracts() {
    let guard = include_str!("../src/repository/sqlite/push/execution_guard.rs");
    let worker = include_str!("../src/workers/execution.rs");
    let migration = include_str!("../migrations/0001_sqlite.sql");
    let config = include_str!("../src/config.rs");

    assert!(guard.contains("finish_before_start"));
    assert!(guard.contains("in_progress"));
    assert!(guard.contains("done"));
    assert!(worker.contains("execution_worker_leases"));
    assert!(worker.contains("fire_due_reminders"));
    assert!(worker.contains("materialize_task_occurrences"));
    assert!(worker.contains("materialize_calendar_occurrences"));
    assert!(worker.contains("deterministic_id"));
    assert!(migration.contains("execution_worker_leases"));
    assert!(migration.contains("lease_until"));
    assert!(config.contains("auth_password_min_length: 9"));
}

#[test]
fn production_uses_split_web_and_single_sqlite_cloud() {
    let cloud_dockerfile = include_str!("../Dockerfile");
    let web_dockerfile = include_str!("../apps/web/Dockerfile");
    let nginx = include_str!("../apps/web/nginx.conf");
    let production = include_str!("../deploy/cloud/docker-compose.production.yml");
    let main = include_str!("../src/main.rs");

    assert!(cloud_dockerfile.contains("ENTRYPOINT [\"/app/lifetrace-cloud\"]"));
    assert!(cloud_dockerfile.contains("LIFETRACE_DATABASE_PATH=/data/lifetrace.db"));
    assert!(!cloud_dockerfile.contains("node:22"));
    assert!(!cloud_dockerfile.contains("/app/web"));
    assert!(!cloud_dockerfile.contains("execution_worker"));
    assert!(!cloud_dockerfile.contains("mail_worker"));

    assert!(web_dockerfile.contains("FROM node:22-alpine AS web-builder"));
    assert!(web_dockerfile.contains("FROM nginx:1.27-alpine"));
    assert!(web_dockerfile.contains("/usr/share/nginx/html/"));
    assert!(nginx.contains("proxy_pass http://cloud:8787"));
    assert!(nginx.contains("client_max_body_size 256m"));

    assert!(production.contains("cloud:"));
    assert!(production.contains("web:"));
    assert!(production.contains("\"80:80\""));
    assert!(production.contains("\"8869:8869\""));
    assert!(production.contains("lifetrace_data:/data"));
    assert!(!production.contains("postgres:"));
    assert!(!production.contains("mail-worker:"));
    assert!(!production.contains("execution-worker:"));

    assert!(main.contains("workers::mail::run"));
    assert!(main.contains("workers::execution::run"));
    assert!(main.contains("rewrite_beecount_request"));
    assert!(main.contains("bootstrap-user"));
    assert!(main.contains("create-invite"));
}

#[test]
fn beecount_compatibility_stays_inside_lifetrace_cloud() {
    let production = include_str!("../deploy/cloud/docker-compose.production.yml");
    let main = include_str!("../src/main.rs");

    assert!(production.contains("\"8869:8869\""));
    assert!(!production.contains("\"443:443\""));
    assert!(!production.contains("beecount-cloud"));
    assert!(main.contains("BEECOUNT_BIND_ADDRESS"));
    assert!(main.contains("/api/v1/integrations/beecount/compat"));
    assert!(main.contains("/health/ready"));
}
