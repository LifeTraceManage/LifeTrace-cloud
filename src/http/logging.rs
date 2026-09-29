use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn middleware(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    let response = next.run(request).await;
    let status = response.status();
    let duration_ms = started.elapsed().as_millis() as u64;

    if path.starts_with("/health/") && status.is_success() {
        tracing::debug!(
            target: "lifetrace::http",
            method = %method,
            path = %path,
            status = status.as_u16(),
            duration_ms,
            request_id = %request_id,
            "http request completed"
        );
    } else if status.is_server_error() {
        tracing::error!(
            target: "lifetrace::http",
            method = %method,
            path = %path,
            status = status.as_u16(),
            duration_ms,
            request_id = %request_id,
            "http request failed"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            target: "lifetrace::http",
            method = %method,
            path = %path,
            status = status.as_u16(),
            duration_ms,
            request_id = %request_id,
            "http request rejected"
        );
    } else {
        tracing::info!(
            target: "lifetrace::http",
            method = %method,
            path = %path,
            status = status.as_u16(),
            duration_ms,
            request_id = %request_id,
            "http request completed"
        );
    }

    response
}
