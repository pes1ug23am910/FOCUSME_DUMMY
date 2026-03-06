// ============================================================
// FILE:        routes/sync.rs
// MODULE:      Phase 5 — Cloud Backend > Plan & Sync Routes
// TASK:        T-062
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: axum 0.7, serde, chrono, uuid
// TEST COVERAGE: plan CRUD, push/pull sync, conflict detection
// KNOWN LIMITATIONS:
//   - Conflict resolution is optimistic (version-based 409).
//   - No batch plan upload — single plan per request.
//   - No pagination on plan list (delta sync with ?since= mitigates).
// ============================================================

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db;
use crate::error::AppError;
use crate::AppState;

// ── Request / Response DTOs ──────────────────────────────────

/// Query parameters for GET /plans and GET /sync/pull.
#[derive(Debug, Deserialize)]
pub struct SinceQuery {
    /// ISO-8601 timestamp — only return records modified after this time.
    /// If omitted, returns all records.
    pub since: Option<DateTime<Utc>>,
}

/// POST /plans request body — create or update a plan.
#[derive(Debug, Deserialize)]
pub struct UpsertPlanRequest {
    /// Client-generated local plan ID (stable across syncs).
    pub local_id: String,
    /// Full plan data as JSON.
    pub plan_json: serde_json::Value,
    /// Expected version for optimistic concurrency.
    /// Omit or set to 0 for new plans.
    pub expected_version: Option<i32>,
}

/// POST /sync/push request body — batch push sync events.
#[derive(Debug, Deserialize)]
pub struct PushSyncRequest {
    /// Device UUID (optional — server can also derive from token).
    pub device_id: Option<Uuid>,
    /// Array of sync events to record.
    pub events: Vec<SyncEventInput>,
}

/// Individual sync event in a push batch.
#[derive(Debug, Deserialize)]
pub struct SyncEventInput {
    /// Event type: "create", "update", "delete", "restore".
    pub event_type: String,
    /// Local plan ID this event relates to.
    pub local_id: String,
    /// Optional event payload (diff, full plan, metadata).
    pub payload: serde_json::Value,
}

/// Plan response wrapper with sync metadata.
#[derive(Debug, Serialize)]
pub struct PlanResponse {
    pub id: Uuid,
    pub local_id: String,
    pub plan_json: serde_json::Value,
    pub version: i32,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<db::CloudPlan> for PlanResponse {
    fn from(p: db::CloudPlan) -> Self {
        PlanResponse {
            id: p.id,
            local_id: p.local_id,
            plan_json: p.plan_json,
            version: p.version,
            deleted: p.deleted_at.is_some(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

/// Sync pull response — plans + events since timestamp.
#[derive(Debug, Serialize)]
pub struct PullSyncResponse {
    pub plans: Vec<PlanResponse>,
    pub events: Vec<db::SyncEvent>,
    /// Server timestamp — clients should use this as `since` on next pull.
    pub server_time: DateTime<Utc>,
}

/// Sync push acknowledgement.
#[derive(Debug, Serialize)]
pub struct PushSyncResponse {
    pub accepted: usize,
    pub server_time: DateTime<Utc>,
}

// ── Plan Router ──────────────────────────────────────────────

/// Build the plan router (mounted at /api/v1/plans).
///
/// All routes require authentication (enforced by parent router middleware).
///
/// Routes:
/// - GET  /          → list_plans (delta sync via ?since=)
/// - POST /          → upsert_plan (create or update)
/// - DELETE /:local_id → soft_delete_plan
pub fn plan_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_plans).post(upsert_plan))
        .route("/{local_id}", delete(delete_plan))
}

/// Build the sync router (mounted at /api/v1/sync).
///
/// All routes require authentication (enforced by parent router middleware).
///
/// Routes:
/// - POST /push → push_sync (batch local changes)
/// - GET  /pull → pull_sync (delta since timestamp)
pub fn sync_router() -> Router<AppState> {
    Router::new()
        .route("/push", post(push_sync))
        .route("/pull", get(pull_sync))
}

// ── Plan Handlers ────────────────────────────────────────────

/// GET /api/v1/plans?since=2024-01-01T00:00:00Z
///
/// Returns all plans modified since the given timestamp.
/// If `since` is omitted, returns all plans for the user.
/// Includes soft-deleted plans so clients can sync deletions.
async fn list_plans(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SinceQuery>,
) -> Result<Json<Vec<PlanResponse>>, AppError> {
    let user_id = parse_user_id(&claims)?;

    let plans = db::get_plans_since(&state.db, user_id, query.since)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let response: Vec<PlanResponse> = plans.into_iter().map(Into::into).collect();
    Ok(Json(response))
}

/// POST /api/v1/plans
///
/// Create or update a plan. Uses `local_id` for upsert matching.
///
/// **Conflict detection:** If `expected_version` is provided and doesn't
/// match the server's current version, returns 409 Conflict with the
/// server's current plan data for the client to resolve.
async fn upsert_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpsertPlanRequest>,
) -> Result<Json<PlanResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;

    // Conflict check: if expected_version is provided, verify it.
    if let Some(expected) = body.expected_version {
        if expected > 0 {
            if let Some(existing) =
                db::get_plan_by_local_id(&state.db, user_id, &body.local_id)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?
            {
                if existing.version != expected {
                    return Err(AppError::Conflict(format!(
                        "Version conflict: server has version {}, client expected {}. \
                         Resolve the conflict and retry.",
                        existing.version, expected
                    )));
                }
            }
        }
    }

    let plan = db::upsert_cloud_plan(&state.db, user_id, &body.local_id, &body.plan_json)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(plan.into()))
}

/// DELETE /api/v1/plans/:local_id
///
/// Soft-deletes a plan (sets deleted_at, preserves data for sync).
/// Other devices will see the deletion on their next pull.
async fn delete_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(local_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = parse_user_id(&claims)?;

    db::soft_delete_plan(&state.db, user_id, &local_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "deleted",
        "local_id": local_id
    })))
}

// ── Sync Handlers ────────────────────────────────────────────

/// POST /api/v1/sync/push
///
/// Batch-push local changes (sync events) to the server.
/// Each event is recorded for audit trail and conflict resolution.
///
/// Clients should push after local changes and before pull.
async fn push_sync(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<PushSyncRequest>,
) -> Result<Json<PushSyncResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;
    let mut accepted = 0;

    for event in &body.events {
        db::record_sync_event(
            &state.db,
            user_id,
            body.device_id,
            &event.event_type,
            &event.local_id,
            &event.payload,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        accepted += 1;
    }

    Ok(Json(PushSyncResponse {
        accepted,
        server_time: Utc::now(),
    }))
}

/// GET /api/v1/sync/pull?since=2024-01-01T00:00:00Z
///
/// Pull all changes (plans + sync events) since the given timestamp.
/// Returns the current server time for the client to use on next pull.
///
/// Delta sync: only returns records modified after `since`.
/// Full sync: omit `since` to get everything.
async fn pull_sync(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SinceQuery>,
) -> Result<Json<PullSyncResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;
    let server_time = Utc::now();

    let plans = db::get_plans_since(&state.db, user_id, query.since)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let events = db::get_sync_events_since(&state.db, user_id, query.since)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let plan_responses: Vec<PlanResponse> = plans.into_iter().map(Into::into).collect();

    Ok(Json(PullSyncResponse {
        plans: plan_responses,
        events,
        server_time,
    }))
}

// ── Helpers ──────────────────────────────────────────────────

/// Parse user UUID from JWT claims subject.
fn parse_user_id(claims: &Claims) -> Result<Uuid, AppError> {
    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))
}
