// ============================================================
// FILE:        tests/sync_test.rs
// MODULE:      Phase 5 — Cloud Backend > Sync Tests
// TASK:        T-062
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// PURPOSE:     Integration tests for plan CRUD + sync endpoints
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

/// Register a user and return the access token.
async fn register_and_get_token(state: &crate::AppState) -> String {
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

    body["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

// ── Plan CRUD Tests ──────────────────────────────────────────

#[tokio::test]
async fn test_create_plan() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/plans")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "local_id": "plan-001",
                        "plan_json": {
                            "name": "Focus on Rust",
                            "duration_minutes": 45,
                            "block_sites": ["reddit.com", "twitter.com"]
                        }
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

    assert_eq!(body["local_id"], "plan-001");
    assert_eq!(body["version"], 1);
    assert_eq!(body["deleted"], false);
}

#[tokio::test]
async fn test_list_plans_empty() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/plans")
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

    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_list_plans_after_create() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    // Create a plan
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/plans")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "local_id": "plan-list-test",
                        "plan_json": {"name": "Test Plan"}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // List plans
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/plans")
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

    let plans = body.as_array().unwrap();
    assert!(!plans.is_empty());
}

#[tokio::test]
async fn test_upsert_plan_version_conflict() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    // Create a plan (version 1)
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/plans")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "local_id": "conflict-test",
                        "plan_json": {"name": "Version 1"}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Update with wrong expected_version → 409
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/plans")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "local_id": "conflict-test",
                        "plan_json": {"name": "Version 2 attempt"},
                        "expected_version": 99
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_soft_delete_plan() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    // Create a plan
    let app = test_router(state.clone());
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/plans")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "local_id": "delete-test",
                        "plan_json": {"name": "To be deleted"}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Soft-delete
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/plans/delete-test")
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

    assert_eq!(body["status"], "deleted");
}

// ── Sync Push/Pull Tests ─────────────────────────────────────

#[tokio::test]
async fn test_push_sync_events() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sync/push")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from(
                    json!({
                        "events": [
                            {
                                "event_type": "create",
                                "local_id": "plan-sync-001",
                                "payload": {"name": "Synced Plan"}
                            },
                            {
                                "event_type": "update",
                                "local_id": "plan-sync-001",
                                "payload": {"name": "Updated Synced Plan"}
                            }
                        ]
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

    assert_eq!(body["accepted"], 2);
    assert!(body["server_time"].is_string());
}

#[tokio::test]
async fn test_pull_sync_full() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    // Full pull (no ?since= parameter)
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/sync/pull")
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

    assert!(body["plans"].is_array());
    assert!(body["events"].is_array());
    assert!(body["server_time"].is_string());
}

#[tokio::test]
async fn test_pull_sync_delta() {
    let state = test_state().await;
    let token = register_and_get_token(&state).await;

    // Delta pull with future timestamp → should be empty
    let app = test_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/sync/pull?since=2099-01-01T00:00:00Z")
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

    assert!(body["plans"].as_array().unwrap().is_empty());
    assert!(body["events"].as_array().unwrap().is_empty());
}
