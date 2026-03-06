// ============================================================
// FILE:        routes/auth_routes.rs
// MODULE:      Phase 5 — Cloud Backend > Auth Routes
// TASK:        T-061
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: axum 0.7, serde
// TEST COVERAGE: register, login, refresh endpoints
// KNOWN LIMITATIONS:
//   - No rate limiting (reverse-proxy responsibility).
//   - No CAPTCHA on registration.
// ============================================================

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::Deserialize;

use crate::auth::TokenPair;
use crate::db::UserResponse;
use crate::error::AppError;
use crate::AppState;

// ── Request / Response DTOs ──────────────────────────────────

/// POST /api/v1/auth/register request body.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// POST /api/v1/auth/login request body.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// POST /api/v1/auth/refresh request body.
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Successful auth response — user info + tokens.
#[derive(Debug, serde::Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

/// Successful refresh response — new token pair only.
#[derive(Debug, serde::Serialize)]
pub struct RefreshResponse {
    pub tokens: TokenPair,
}

// ── Router ───────────────────────────────────────────────────

/// Build the auth router.
///
/// Routes (all unauthenticated):
/// - POST /register  → register_handler
/// - POST /login     → login_handler
/// - POST /refresh   → refresh_handler
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
}

// ── Handlers ─────────────────────────────────────────────────

/// POST /api/v1/auth/register
///
/// Creates a new user account and returns a JWT token pair.
///
/// Request:  `{ "email": "user@example.com", "password": "hunter2..." }`
/// Response: `{ "user": {...}, "tokens": { "access_token": "...", ... } }`
/// Errors:
///   - 400: Invalid email or password too short.
///   - 409: Email already registered.
async fn register_handler(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (user, tokens) = state
        .auth
        .register(&body.email, &body.password)
        .await?;

    Ok(Json(AuthResponse { user, tokens }))
}

/// POST /api/v1/auth/login
///
/// Authenticates user with email + password, returns JWT token pair.
///
/// Request:  `{ "email": "user@example.com", "password": "hunter2..." }`
/// Response: `{ "user": {...}, "tokens": { "access_token": "...", ... } }`
/// Errors:
///   - 401: Invalid credentials (intentionally vague for security).
async fn login_handler(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (user, tokens) = state
        .auth
        .login(&body.email, &body.password)
        .await?;

    Ok(Json(AuthResponse { user, tokens }))
}

/// POST /api/v1/auth/refresh
///
/// Exchanges a valid refresh token for a new token pair.
/// Implements refresh token rotation — each refresh token is single-use.
///
/// Request:  `{ "refresh_token": "eyJhbG..." }`
/// Response: `{ "tokens": { "access_token": "...", ... } }`
/// Errors:
///   - 401: Invalid, expired, or already-used refresh token.
async fn refresh_handler(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    let tokens = state
        .auth
        .refresh(&body.refresh_token)
        .await?;

    Ok(Json(RefreshResponse { tokens }))
}
