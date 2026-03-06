// ============================================================
// FILE:        tests/family_test.rs
// MODULE:      Phase 5 — Cloud Backend > Family Tests
// TASK:        T-064
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// PURPOSE:     Integration tests for family dashboard endpoints
// DEPENDENCIES: reqwest, axum (test utils)
// KNOWN LIMITATIONS: Requires running PostgreSQL + DATABASE_URL set.
// ============================================================

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::{test_state, test_router, unique_email, test_password};

// ── Helper ───────────────────────────────────────────────────

/// Register a user and return (access_token, user_id).
async fn register_user(state: &crate::AppState) -> (String, String) {
    let app = test_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "email": unique_email(),
                        "password": test_password()
                    })
                    .to_string(),
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

    (
        body["tokens"]["access_token"]
            .as_str()
            .unwrap()
            .to_string(),
        body["user"]["id"].as_str().unwrap().to_string(),
    )
}

// ── Family Group Tests ───────────────────────────────────────

#[tokio::test]
async fn test_create_family_group() {
    let state = test_state().await;
    let (token, _user_id) = register_user(&state).await;

    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "name": "Smith Family" }).to_string(),
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

    assert_eq!(body["name"], "Smith Family");
    assert_eq!(body["member_count"], 1);
}

#[tokio::test]
async fn test_create_family_empty_name() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(json!({ "name": "" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_families() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    // Create a family
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "name": "Test Family" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // List families
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/family")
                .header("Authorization", format!("Bearer {}", token))
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

    assert!(!body.as_array().unwrap().is_empty());
}

// ── Invite Tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_invite_member() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    // Create a family group first
    let app = test_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "name": "Invite Test Family" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let family: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    let family_id = family["id"].as_str().unwrap();

    // Invite a member
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family/invite")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "family_id": family_id,
                        "email": "invited@example.com"
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

    assert!(body["invite_token"].is_string());
    assert!(body["expires_at"].is_string());
}

#[tokio::test]
async fn test_invite_non_owner_forbidden() {
    let state = test_state().await;
    let (owner_token, _) = register_user(&state).await;
    let (other_token, _) = register_user(&state).await;

    // Owner creates a family group
    let app = test_router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", owner_token))
                .body(Body::from(
                    json!({ "name": "Owner's Family" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let family: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    let family_id = family["id"].as_str().unwrap();

    // Non-owner tries to invite → 403
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family/invite")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", other_token))
                .body(Body::from(
                    json!({
                        "family_id": family_id,
                        "email": "intruder@example.com"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ── Dashboard Tests ──────────────────────────────────────────

#[tokio::test]
async fn test_dashboard_with_family() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    // Create a family first
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "name": "Dashboard Family" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get dashboard
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/family/dashboard")
                .header("Authorization", format!("Bearer {}", token))
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

    assert_eq!(body["family"]["name"], "Dashboard Family");
    assert!(body["members"].is_array());
    assert!(body["shared_plans"].is_array());
}

#[tokio::test]
async fn test_dashboard_no_family() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    // No family created → 404
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/family/dashboard")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ── Members Tests ────────────────────────────────────────────

#[tokio::test]
async fn test_list_members() {
    let state = test_state().await;
    let (token, _) = register_user(&state).await;

    // Create a family
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/family")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({ "name": "Members Test" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // List members
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/family/members")
                .header("Authorization", format!("Bearer {}", token))
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

    // Owner should be listed as a member
    let members = body.as_array().unwrap();
    assert!(!members.is_empty());
    assert_eq!(members[0]["role"], "owner");
}
