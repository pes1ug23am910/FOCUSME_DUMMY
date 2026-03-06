// ============================================================
// FILE:        middleware/security_headers.rs
// MODULE:      Phase 5 — Cloud Backend > Security Headers
// TASK:        T-060 (Session 8 hardening)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 8
// DEPENDENCIES: axum, tower
// TEST COVERAGE: Header injection verified in integration tests
// KNOWN LIMITATIONS: None. Headers are static and cannot be configured
//   per-route. Override in reverse proxy if needed.
// ============================================================

use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};

/// Security headers middleware for Axum.
///
/// Injects the following headers on every response:
///
/// - `X-Content-Type-Options: nosniff` — Prevents MIME-type sniffing.
/// - `X-Frame-Options: DENY` — Prevents clickjacking via iframes.
/// - `X-XSS-Protection: 1; mode=block` — Legacy XSS filter (IE/old Chrome).
/// - `Referrer-Policy: strict-origin-when-cross-origin` — Limits referrer leakage.
/// - `Content-Security-Policy: default-src 'none'; frame-ancestors 'none'`
///     — API-only CSP — no resources allowed.
/// - `Permissions-Policy: geolocation=(), microphone=(), camera=()`
///     — Disables browser feature access from API responses.
///
/// Also removes the `Server` header to avoid leaking server software info.
pub async fn security_headers_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let mut response = next.run(req).await;

    let headers = response.headers_mut();

    // Prevent MIME-type sniffing attacks
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().unwrap(),
    );

    // Prevent clickjacking — no framing allowed
    headers.insert(
        "x-frame-options",
        "DENY".parse().unwrap(),
    );

    // Legacy XSS protection for older browsers
    headers.insert(
        "x-xss-protection",
        "1; mode=block".parse().unwrap(),
    );

    // Limit referrer information sent cross-origin
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // API-only CSP — no resources should be loaded from API responses
    headers.insert(
        "content-security-policy",
        "default-src 'none'; frame-ancestors 'none'".parse().unwrap(),
    );

    // Disable unnecessary browser permissions
    headers.insert(
        "permissions-policy",
        "geolocation=(), microphone=(), camera=()".parse().unwrap(),
    );

    // Remove Server header to avoid leaking software information
    headers.remove("server");

    response
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn test_security_headers_injected() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(security_headers_middleware));

        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();

        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            response.headers().get("x-frame-options").unwrap(),
            "DENY"
        );
        assert_eq!(
            response.headers().get("x-xss-protection").unwrap(),
            "1; mode=block"
        );
        assert_eq!(
            response.headers().get("referrer-policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            response.headers().get("content-security-policy").unwrap(),
            "default-src 'none'; frame-ancestors 'none'"
        );
        assert_eq!(
            response.headers().get("permissions-policy").unwrap(),
            "geolocation=(), microphone=(), camera=()"
        );
        assert!(response.headers().get("server").is_none());
    }
}
