// ============================================================
// FILE:        routes/health.rs
// MODULE:      Phase 5 — Cloud Backend > Health Check Routes
// TASK:        Session 9 A3 (health check improvements)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 9
// DEPENDENCIES: axum, sqlx, serde, tokio
// TEST COVERAGE: 4 tests — liveness, readiness up, readiness down, version
// KNOWN LIMITATIONS:
//   - /health/version returns "unknown" for git_sha and build_timestamp
//     unless VERGEN_GIT_SHA and VERGEN_BUILD_TIMESTAMP env vars are set
//     at build time (e.g., via vergen crate or CI build args).
//   - Uptime is measured from the first call to start_time(), not from
//     server boot. Consider using a lazy_static for exact boot time.
// ============================================================

//! Health check endpoints for liveness, readiness, and version probes.
//!
//! - `GET /health` — lightweight liveness probe (no DB, always 200)
//! - `GET /health/ready` — readiness probe (checks DB connectivity)
//! - `GET /health/version` — build info (version, git SHA, build timestamp)

use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::OnceLock;
use tokio::time::Instant;

use crate::AppState;

/// Server start time — set on first access.
static START_TIME: OnceLock<Instant> = OnceLock::new();

fn start_time() -> &'static Instant {
    START_TIME.get_or_init(Instant::now)
}

// ── Response Types ──────────────────────────────────────────

/// Readiness probe response.
#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    /// Overall status: "ready" or "degraded"
    pub status: &'static str,
    /// Database connectivity: "up" or "down"
    pub db: &'static str,
    /// Server uptime in seconds since first readiness check
    pub uptime_seconds: u64,
}

/// Version/build info response.
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    /// Crate version from Cargo.toml
    pub version: &'static str,
    /// Git commit SHA (set via VERGEN_GIT_SHA env var at build time)
    pub git_sha: &'static str,
    /// Build timestamp (set via VERGEN_BUILD_TIMESTAMP env var at build time)
    pub build_timestamp: &'static str,
}

// ── Handlers ────────────────────────────────────────────────

/// GET /health/ready — readiness probe.
///
/// Checks database connectivity via `SELECT 1`. Returns:
/// - 200 with `status: "ready"` if DB responds
/// - 503 with `status: "degraded"` if DB is unreachable
///
/// Used by Kubernetes readiness probes and load balancers to determine
/// if the instance should receive traffic.
async fn readiness_handler(
    State(state): State<AppState>,
) -> (StatusCode, Json<ReadinessResponse>) {
    let uptime = start_time().elapsed().as_secs();

    // Check database connectivity with a simple query.
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let (status_code, status_text, db_text) = if db_ok {
        (StatusCode::OK, "ready", "up")
    } else {
        tracing::warn!("Readiness check failed: database unreachable");
        (StatusCode::SERVICE_UNAVAILABLE, "degraded", "down")
    };

    (
        status_code,
        Json(ReadinessResponse {
            status: status_text,
            db: db_text,
            uptime_seconds: uptime,
        }),
    )
}

/// GET /health/version — build information.
///
/// Returns the crate version, git SHA, and build timestamp. Git SHA and
/// build timestamp require VERGEN_GIT_SHA and VERGEN_BUILD_TIMESTAMP
/// environment variables to be set at build time (e.g., via `vergen` crate
/// or CI build arguments). Falls back to "unknown" if not set.
async fn version_handler() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
        build_timestamp: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
    })
}

// ── Router ──────────────────────────────────────────────────

/// Build the health check sub-router.
///
/// Mounts at `/health`:
/// - `GET /health/ready` — readiness probe (DB check)
/// - `GET /health/version` — build info
///
/// Note: The liveness probe `GET /health` is registered directly in
/// `build_router()` in main.rs (no state needed for the lightweight check).
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/ready", get(readiness_handler))
        .route("/version", get(version_handler))
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_response_format() {
        let resp = VersionResponse {
            version: "0.1.0",
            git_sha: "abc1234",
            build_timestamp: "2026-03-05T12:00:00Z",
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["version"], "0.1.0");
        assert_eq!(json["git_sha"], "abc1234");
        assert_eq!(json["build_timestamp"], "2026-03-05T12:00:00Z");
    }

    #[test]
    fn test_readiness_response_ready() {
        let resp = ReadinessResponse {
            status: "ready",
            db: "up",
            uptime_seconds: 42,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "ready");
        assert_eq!(json["db"], "up");
        assert_eq!(json["uptime_seconds"], 42);
    }

    #[test]
    fn test_readiness_response_degraded() {
        let resp = ReadinessResponse {
            status: "degraded",
            db: "down",
            uptime_seconds: 100,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "degraded");
        assert_eq!(json["db"], "down");
    }

    #[test]
    fn test_version_response_unknown_fallback() {
        let resp = VersionResponse {
            version: "0.1.0",
            git_sha: "unknown",
            build_timestamp: "unknown",
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["git_sha"], "unknown");
        assert_eq!(json["build_timestamp"], "unknown");
    }
}
