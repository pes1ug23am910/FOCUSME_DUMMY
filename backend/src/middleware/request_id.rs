// ============================================================
// FILE:        middleware/request_id.rs
// MODULE:      Phase 5 — Cloud Backend > Request ID Middleware
// TASK:        Session 9 A4 (observability)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 9
// DEPENDENCIES: axum, uuid, tracing
// TEST COVERAGE: 3 tests — ID generated, ID pass-through, ID in response
// KNOWN LIMITATIONS:
//   - Uses UUID v4 — not k-sortable. Consider ULIDs if log ordering by
//     request_id is needed.
// ============================================================

//! Request ID middleware for request tracing and log correlation.
//!
//! Generates a UUID v4 per request and:
//! 1. Injects it into the tracing span as `request_id` field
//! 2. Returns it in the `X-Request-Id` response header
//! 3. If the incoming request already has `X-Request-Id` (e.g., from a
//!    reverse proxy), that value is preserved (pass-through mode)

use axum::{
    body::Body,
    http::{header::HeaderValue, Request, Response},
    middleware::Next,
};
use uuid::Uuid;

/// Header name for request ID propagation.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Axum middleware function that assigns a unique request ID to each request.
///
/// Behavior:
/// - If the incoming request has an `X-Request-Id` header, its value is
///   preserved and passed through to the response (proxy forwarding).
/// - If no `X-Request-Id` header is present, a new UUID v4 is generated.
/// - The request ID is added to the response as `X-Request-Id`.
/// - The request ID is recorded in the current tracing span for structured
///   log correlation.
///
/// # Example log output
/// ```text
/// 2026-03-05T12:00:00Z INFO request_id=a1b2c3d4-... method=GET path=/health
/// ```
pub async fn request_id_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Extract existing request ID or generate a new one.
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Record in tracing span for structured log correlation.
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
    );
    let _guard = span.enter();

    // Process the request.
    let mut response = next.run(req).await;

    // Inject request ID into response header.
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, value);
    }

    response
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    /// Simple handler for testing.
    async fn ok_handler() -> &'static str {
        "ok"
    }

    /// Build a test router with request_id middleware.
    fn test_router() -> Router {
        Router::new()
            .route("/test", get(ok_handler))
            .layer(middleware::from_fn(request_id_middleware))
    }

    #[tokio::test]
    async fn test_request_id_generated_when_absent() {
        let app = test_router();

        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // Response must contain X-Request-Id header
        let id = resp
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("Response should have X-Request-Id header");

        let id_str = id.to_str().unwrap();
        // Must be a valid UUID v4 format (36 chars with hyphens)
        assert_eq!(id_str.len(), 36, "Request ID should be UUID format");
        assert!(
            Uuid::parse_str(id_str).is_ok(),
            "Request ID should be valid UUID"
        );
    }

    #[tokio::test]
    async fn test_request_id_passed_through_when_present() {
        let app = test_router();
        let existing_id = "proxy-generated-request-id-12345";

        let req = Request::builder()
            .uri("/test")
            .header(REQUEST_ID_HEADER, existing_id)
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // Response must contain the same X-Request-Id we sent
        let id = resp
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("Response should have X-Request-Id header");

        assert_eq!(id.to_str().unwrap(), existing_id);
    }

    #[tokio::test]
    async fn test_request_id_in_response_header() {
        let app = test_router();

        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();

        // Verify the header exists and is non-empty
        let id = resp.headers().get(REQUEST_ID_HEADER).unwrap();
        assert!(!id.is_empty(), "X-Request-Id must not be empty");
    }
}
