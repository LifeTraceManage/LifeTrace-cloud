use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, Method};

use crate::auth::security::cookie_value;
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::error::ApiError;
use crate::state::AppState;

fn web_session_requires_csrf(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

impl FromRequestParts<AppState> for AuthenticatedPrincipal {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        // Native and service clients continue to authenticate with Bearer
        // credentials. If an Authorization header is present, never silently
        // fall back to the browser cookie.
        if authorization.is_some() {
            return state
                .auth
                .authenticate(AuthCredential::Bearer(authorization))
                .await;
        }

        let session = cookie_value(&parts.headers, &state.config.auth_cookie_name);

        // Browser read requests may use the HttpOnly Web Session cookie.
        if !web_session_requires_csrf(&parts.method) {
            return state
                .auth
                .authenticate(AuthCredential::WebSession(session.as_deref()))
                .await;
        }

        // Browser mutations require both the Web Session cookie and the CSRF
        // token bound to that session. This keeps cookie fallback from turning
        // Bearer-only write routes into CSRF-vulnerable endpoints.
        let raw_session = session.as_deref().unwrap_or_default();
        let csrf = parts
            .headers
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let origin = parts
            .headers
            .get("origin")
            .and_then(|value| value.to_str().ok());

        state
            .auth_service
            .verify_web_csrf(raw_session, csrf, origin)
            .await
    }
}

/// Authenticate a read-capable HTTP request with a Bearer token when present,
/// otherwise fall back to the HttpOnly web-session cookie.
pub async fn authenticate_bearer_or_web_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization.is_some() {
        return state
            .auth
            .authenticate(AuthCredential::Bearer(authorization))
            .await;
    }

    let session = cookie_value(headers, &state.config.auth_cookie_name);
    state
        .auth
        .authenticate(AuthCredential::WebSession(session.as_deref()))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_session_reads_do_not_require_csrf() {
        assert!(!web_session_requires_csrf(&Method::GET));
        assert!(!web_session_requires_csrf(&Method::HEAD));
        assert!(!web_session_requires_csrf(&Method::OPTIONS));
    }

    #[test]
    fn web_session_mutations_require_csrf() {
        assert!(web_session_requires_csrf(&Method::POST));
        assert!(web_session_requires_csrf(&Method::PUT));
        assert!(web_session_requires_csrf(&Method::PATCH));
        assert!(web_session_requires_csrf(&Method::DELETE));
    }
}
