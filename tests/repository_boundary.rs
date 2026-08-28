#[test]
fn cloud_repository_owns_execution_and_auth_server_contracts() {
    let guard = include_str!("../src/postgres_repository/push/execution_guard.rs");
    let worker = include_str!("../src/bin/execution_worker.rs");
    let migration = include_str!("../migrations/0021_execution_worker.sql");
    let config = include_str!("../src/config.rs");

    assert!(guard.contains("finish_before_start"));
    assert!(guard.contains("in_progress"));
    assert!(guard.contains("done"));
    assert!(worker.contains("execution_worker_leases"));
    assert!(worker.contains("fire_due_reminders"));
    assert!(worker.contains("materialize_task_occurrences"));
    assert!(worker.contains("materialize_calendar_occurrences"));
    assert!(worker.contains("deterministic_id"));
    assert!(migration.contains("lease_until"));
    assert!(config.contains("auth_password_min_length: 9"));
}

#[test]
fn cloud_image_and_compose_keep_execution_worker_and_current_images() {
    let dockerfile = include_str!("../Dockerfile");
    let production = include_str!("../deploy/cloud/docker-compose.production.yml");
    let local = include_str!("../deploy/cloud/docker-compose.local.yml");
    let wsl = include_str!("../deploy/cloud/docker-compose.wsl.yml");

    assert!(dockerfile.contains("--bin execution_worker"));
    assert!(dockerfile.contains("target/release/execution_worker /app/execution_worker"));
    assert!(production.contains("lifetrace-execution-worker:"));
    assert!(production.contains("entrypoint: [\"/app/execution_worker\"]"));
    assert!(local.contains("lifetrace-execution-worker:"));
    assert!(production.contains("ghcr.io/lifetracemanage/lifetrace-web:main"));

    for compose in [production, wsl] {
        assert!(!compose.contains("sunxiao0721/beecount-cloud"));
        assert!(!compose.contains("  beecount-cloud:"));
        assert!(!compose.contains("beecount_data:/data"));
    }
}

#[test]
fn beecount_compatibility_stays_on_lifetrace_cloud_over_direct_http() {
    let production = include_str!("../deploy/cloud/docker-compose.production.yml");
    let caddy = include_str!("../deploy/cloud/Caddyfile.production");

    assert!(production.contains("\"8869:8869\""));
    assert!(!production.contains("\"443:443\""));
    assert!(caddy.contains("http://:8869"));
    assert!(caddy.contains("handle /ready"));
    assert!(caddy.contains("rewrite * /health/ready"));
    assert!(caddy.contains("handle /api/v1/*"));
    assert!(caddy.contains("/api/v1/integrations/beecount/compat/"));
    assert!(caddy.contains("reverse_proxy lifetrace-cloud:8787"));
    assert!(!caddy.contains("reverse_proxy beecount-cloud:8080"));
    assert!(!caddy.contains("https://"));
}
