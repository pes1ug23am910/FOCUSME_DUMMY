// ============================================================
// FILE:        routes/family.rs
// MODULE:      Phase 5 — Cloud Backend > Family Dashboard Routes
// TASK:        T-064
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: axum 0.7, serde, uuid, chrono
// TEST COVERAGE: invite, members, share_plan, dashboard
// KNOWN LIMITATIONS:
//   - Invite uses random token — no email delivery (future Phase 6).
//   - Family group limited to one per user for MVP.
//   - Dashboard aggregation is a basic query — no caching.
// ============================================================

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db;
use crate::error::AppError;
use crate::AppState;

// ── Request / Response DTOs ──────────────────────────────────

/// POST /family request body — create a family group.
#[derive(Debug, Deserialize)]
pub struct CreateFamilyRequest {
    pub name: String,
}

/// POST /family/invite request body.
#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    /// Family group UUID to invite into.
    pub family_id: Uuid,
    /// Email of the person to invite.
    pub email: String,
}

/// POST /family/invite/accept request body.
#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    /// The invite token received (e.g. via email link).
    pub invite_token: String,
}

/// POST /family/plans/share/:plan_id request body.
#[derive(Debug, Deserialize)]
pub struct SharePlanRequest {
    /// Family group UUID.
    pub family_id: Uuid,
    /// Target user UUID (optional — if omitted, shared with whole family).
    pub shared_with: Option<Uuid>,
}

/// Family group response with member count.
#[derive(Debug, Serialize)]
pub struct FamilyResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub member_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Invite response with token.
#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub invite_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Family dashboard — aggregate view of family activity.
#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub family: db::FamilyGroup,
    pub members: Vec<db::FamilyMember>,
    pub shared_plans: Vec<db::SharedPlan>,
    pub total_plans: usize,
    pub active_members: usize,
}

// ── Router ───────────────────────────────────────────────────

/// Build the family router (mounted at /api/v1/family).
///
/// All routes require authentication (enforced by parent router middleware).
///
/// Routes:
/// - POST /              → create_family (create family group)
/// - GET  /              → list_families (user's family groups)
/// - GET  /members       → list_members (all members across groups)
/// - POST /invite        → invite_member (send invite)
/// - POST /invite/accept → accept_invite (accept invite with token)
/// - POST /plans/share/:plan_id → share_plan (share a plan)
/// - GET  /dashboard     → dashboard (family activity summary)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_family).get(list_families))
        .route("/members", get(list_members))
        .route("/invite", post(invite_member))
        .route("/invite/accept", post(accept_invite))
        .route("/plans/share/{plan_id}", post(share_plan))
        .route("/dashboard", get(dashboard))
}

// ── Handlers ─────────────────────────────────────────────────

/// POST /api/v1/family
///
/// Create a new family group. The authenticated user becomes the owner
/// and is automatically added as a member with role "owner".
async fn create_family(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateFamilyRequest>,
) -> Result<Json<FamilyResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;

    if body.name.trim().is_empty() {
        return Err(AppError::Validation(
            "Family group name cannot be empty".to_string(),
        ));
    }

    let group = db::create_family_group(&state.db, &body.name, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(FamilyResponse {
        id: group.id,
        name: group.name,
        owner_id: group.owner_id,
        member_count: 1, // Owner is the first member
        created_at: group.created_at,
    }))
}

/// GET /api/v1/family
///
/// List all family groups the authenticated user belongs to.
async fn list_families(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<db::FamilyGroup>>, AppError> {
    let user_id = parse_user_id(&claims)?;

    let families = db::get_user_families(&state.db, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(families))
}

/// GET /api/v1/family/members
///
/// List all members across the user's family groups.
/// Returns members from all groups the user belongs to.
async fn list_members(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<db::FamilyMember>>, AppError> {
    let user_id = parse_user_id(&claims)?;

    // Get all families the user belongs to, then aggregate members
    let families = db::get_user_families(&state.db, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut all_members = Vec::new();
    for family in &families {
        let members = db::get_family_members(&state.db, family.id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        all_members.extend(members);
    }

    Ok(Json(all_members))
}

/// POST /api/v1/family/invite
///
/// Invite a person to a family group by email.
/// Generates a random invite token valid for 7 days.
///
/// Only the family owner can send invites.
async fn invite_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<InviteRequest>,
) -> Result<Json<InviteResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;

    // Verify the user owns this family group
    let families = db::get_user_families(&state.db, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let is_owner = families.iter().any(|f| f.id == body.family_id && f.owner_id == user_id);
    if !is_owner {
        return Err(AppError::Forbidden(
            "Only the family owner can send invitations".to_string(),
        ));
    }

    // Generate invite token and expiry
    let invite_token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(7);

    db::create_family_invite(
        &state.db,
        body.family_id,
        &body.email,
        &invite_token,
        expires_at,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(InviteResponse {
        invite_token,
        expires_at,
    }))
}

/// POST /api/v1/family/invite/accept
///
/// Accept a family invitation using the invite token.
/// Links the authenticated user to the family group.
async fn accept_invite(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AcceptInviteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = parse_user_id(&claims)?;

    let accepted = db::accept_family_invite(&state.db, &body.invite_token, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !accepted {
        return Err(AppError::NotFound(
            "Invite not found, expired, or already accepted".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "status": "accepted",
        "message": "You have joined the family group"
    })))
}

/// POST /api/v1/family/plans/share/:plan_id
///
/// Share a plan with a family group member (or the whole group).
async fn share_plan(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(plan_id): Path<Uuid>,
    Json(body): Json<SharePlanRequest>,
) -> Result<Json<db::SharedPlan>, AppError> {
    let user_id = parse_user_id(&claims)?;

    // Verify the user is a member of the family
    let families = db::get_user_families(&state.db, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !families.iter().any(|f| f.id == body.family_id) {
        return Err(AppError::Forbidden(
            "You are not a member of this family group".to_string(),
        ));
    }

    let shared = db::share_plan(
        &state.db,
        body.family_id,
        plan_id,
        user_id,
        body.shared_with,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(shared))
}

/// GET /api/v1/family/dashboard
///
/// Aggregate family dashboard — summarizes members, shared plans,
/// and activity for the user's primary family group.
async fn dashboard(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<DashboardResponse>, AppError> {
    let user_id = parse_user_id(&claims)?;

    // Get the user's families (use the first one for dashboard MVP)
    let families = db::get_user_families(&state.db, user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let family = families.into_iter().next().ok_or_else(|| {
        AppError::NotFound("You are not a member of any family group".to_string())
    })?;

    let members = db::get_family_members(&state.db, family.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let shared_plans = db::get_shared_plans(&state.db, family.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let active_members = members.iter().filter(|m| m.accepted_at.is_some()).count();

    Ok(Json(DashboardResponse {
        family,
        total_plans: shared_plans.len(),
        active_members,
        members,
        shared_plans,
    }))
}

// ── Helpers ──────────────────────────────────────────────────

/// Parse user UUID from JWT claims subject.
fn parse_user_id(claims: &Claims) -> Result<Uuid, AppError> {
    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()))
}
