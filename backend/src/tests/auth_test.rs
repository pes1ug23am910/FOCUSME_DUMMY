// ============================================================
// FILE:        tests/auth_test.rs
// MODULE:      Phase 5 — Cloud Backend > Auth Tests
// TASK:        T-061
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// PURPOSE:     Integration tests for authentication endpoints
// DEPENDENCIES: reqwest, axum (test utils)
// KNOWN LIMITATIONS: Requires running PostgreSQL + DATABASE_URL set.
// ============================================================

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

use super::{test_state, test_router, unique_email, test_password};

// ── Registration Tests ───────────────────────────────────────

#[tokio::test]
async fn test_register_success() {
    let state = test_state().await;
    let app = test_router(state);
    let email = unique_email();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "email": email,
                        "password": test_password()
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(body["user"]["id"].is_string());
    assert_eq!(body["user"]["email"], email);
    assert!(body["tokens"]["access_token"].is_string());
    assert!(body["tokens"]["refresh_token"].is_string());
    assert_eq!(body["tokens"]["token_type"], "Bearer");
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let state = test_state().await;
    let email = unique_email();

    // Register first time
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Register same email again → 409 Conflict
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_register_invalid_email() {
    let state = test_state().await;
    let app = test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": "not-an-email", "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_short_password() {
    let state = test_state().await;
    let app = test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": unique_email(), "password": "short" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ── Login Tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_login_success() {
    let state = test_state().await;
    let email = unique_email();

    // Register
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(body["tokens"]["access_token"].is_string());
}

#[tokio::test]
async fn test_login_wrong_password() {
    let state = test_state().await;
    let email = unique_email();

    // Register
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login with wrong password
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "WrongPassword123!" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let state = test_state().await;
    let app = test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": "nobody@nowhere.com", "password": test_password() })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ── Refresh Tests ────────────────────────────────────────────

#[tokio::test]
async fn test_refresh_token_rotation() {
    let state = test_state().await;
    let email = unique_email();

    // Register and get tokens
    let app = test_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    let refresh_token = body["tokens"]["refresh_token"].as_str().unwrap();

    // Refresh
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh_token }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(body["tokens"]["access_token"].is_string());
    assert!(body["tokens"]["refresh_token"].is_string());
}

// ── Protected Route Tests ────────────────────────────────────

#[tokio::test]
async fn test_protected_route_without_token() {
    let state = test_state().await;
    let app = test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_route_with_valid_token() {
    let state = test_state().await;
    let email = unique_email();

    // Register and get access token
    let app = test_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": test_password() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    let access_token = body["tokens"]["access_token"].as_str().unwrap();

    // Access protected route
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/plans")
                .header("Authorization", format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ── Health Check Test ────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let state = test_state().await;
    let app = test_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(body["status"], "ok");
}
