// ============================================================
// FILE:        tests/mod.rs
// MODULE:      Phase 5 — Cloud Backend > Test Suite
// TASK:        T-060
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// PURPOSE:     Test module registry + shared test utilities
// ============================================================

mod auth_test;
mod sync_test;
mod family_test;
mod api_contract_test;

use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::analytics::AnalyticsClient;
use crate::auth::AuthService;
use crate::middleware::RateLimiter;
use crate::AppState;

/// Create a test application state with a real database pool.
///
/// Requires a running PostgreSQL instance (e.g. via docker-compose).
/// Set `DATABASE_URL` in the test environment.
///
/// Each test should use a unique email to avoid collisions.
#[allow(dead_code)]
pub async fn test_state() -> AppState {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests");

    let db = crate::db::create_pool(&database_url)
        .await
        .expect("Failed to create test database pool");

    crate::db::run_migrations(&db)
        .await
        .expect("Failed to run test migrations");

    let jwt_secret = "test-secret-do-not-use-in-production".to_string();
    let auth = AuthService::new(db.clone(), jwt_secret);

    AppState {
        db,
        auth: Arc::new(auth),
        analytics: Arc::new(AnalyticsClient::new()),
        rate_limiter: RateLimiter::new(),
    }
}

/// Build a test router with the given state.
#[allow(dead_code)]
pub fn test_router(state: AppState) -> Router {
    crate::build_router(state)
}

/// Generate a unique test email to avoid collisions between tests.
#[allow(dead_code)]
pub fn unique_email() -> String {
    format!("test-{}@focusme-test.local", Uuid::new_v4())
}

/// Generate a valid test password.
#[allow(dead_code)]
pub fn test_password() -> &'static str {
    "TestPassword123!"
}
