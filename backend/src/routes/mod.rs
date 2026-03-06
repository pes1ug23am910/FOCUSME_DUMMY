// ============================================================
// FILE:        routes/mod.rs
// MODULE:      Phase 5 — Cloud Backend > Route Registry
// TASK:        T-060
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: axum
// ============================================================

pub mod auth_routes;
pub mod family;
pub mod health;
pub mod sync;

use axum::Router;
use crate::AppState;

/// Auth route group: /api/v1/auth/*
pub fn auth_routes() -> Router<AppState> {
    auth_routes::router()
}

/// Plan/sync route group: /api/v1/plans/*
pub fn plan_routes() -> Router<AppState> {
    sync::plan_router()
}

/// Sync route group: /api/v1/sync/*
pub fn sync_routes() -> Router<AppState> {
    sync::sync_router()
}

/// Family route group: /api/v1/family/*
pub fn family_routes() -> Router<AppState> {
    family::router()
}

/// Health check route group: /health/*
pub fn health_check_routes() -> Router<AppState> {
    health::health_routes()
}
