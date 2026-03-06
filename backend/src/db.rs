// ============================================================
// FILE:        db.rs
// MODULE:      Phase 5 — Cloud Backend > Database Layer
// TASK:        T-060
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: sqlx 0.7 (postgres), uuid, chrono, serde
// TEST COVERAGE: Pool creation, migration runner
// KNOWN LIMITATIONS: No connection pool tuning in dev — uses sqlx defaults.
//                    Production should set max_connections via env var.
// DECISION:    D-014 — PostgreSQL for cloud backend (JSONB, robust indexing,
//              pgcrypto UUID generation, TIMESTAMPTZ for global users)
// ============================================================

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

// ── Pool Management ──────────────────────────────────────────

/// Create a PostgreSQL connection pool.
///
/// Uses `max_connections = 10` for development.
/// Production deployments should tune this based on
/// expected concurrency and Postgres `max_connections`.
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;

    Ok(pool)
}

/// Run database migrations from the migrations/ directory.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;

    Ok(())
}

// ── Data Models ──────────────────────────────────────────────

/// User record (never includes password_hash in API responses)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User response (safe for API — excludes password_hash)
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        UserResponse {
            id: u.id,
            email: u.email,
            email_verified: u.email_verified,
            created_at: u.created_at,
        }
    }
}

/// Device record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_name: String,
    pub platform: String,
    pub last_seen: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    pub push_token: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Cloud plan record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CloudPlan {
    pub id: Uuid,
    pub user_id: Uuid,
    pub local_id: String,
    pub plan_json: serde_json::Value,
    pub version: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync event record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncEvent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub local_id: String,
    pub payload: serde_json::Value,
    pub synced_at: DateTime<Utc>,
}

/// Family group record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FamilyGroup {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Family member record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FamilyMember {
    pub id: Uuid,
    pub family_id: Uuid,
    pub user_id: Option<Uuid>,
    pub email: String,
    pub role: String,
    pub invite_token: Option<String>,
    pub invite_expires_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Shared plan record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SharedPlan {
    pub id: Uuid,
    pub family_id: Uuid,
    pub plan_id: Uuid,
    pub shared_by: Uuid,
    pub shared_with: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── User Queries ─────────────────────────────────────────────

/// Create a new user with Argon2id password hash.
pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
) -> Result<User> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

/// Find a user by email address.
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Find a user by ID.
pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

// ── Cloud Plan Queries ───────────────────────────────────────

/// Upsert a cloud plan (create or update by user_id + local_id).
/// Increments version on update. Returns the upserted plan.
pub async fn upsert_cloud_plan(
    pool: &PgPool,
    user_id: Uuid,
    local_id: &str,
    plan_json: &serde_json::Value,
) -> Result<CloudPlan> {
    let plan = sqlx::query_as::<_, CloudPlan>(
        r#"
        INSERT INTO cloud_plans (user_id, local_id, plan_json, version)
        VALUES ($1, $2, $3, 1)
        ON CONFLICT (user_id, local_id)
        DO UPDATE SET
            plan_json = EXCLUDED.plan_json,
            version = cloud_plans.version + 1,
            deleted_at = NULL,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(local_id)
    .bind(plan_json)
    .fetch_one(pool)
    .await?;

    Ok(plan)
}

/// Get all active (non-deleted) plans for a user, optionally filtered by
/// `since` timestamp for delta sync.
pub async fn get_plans_since(
    pool: &PgPool,
    user_id: Uuid,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<CloudPlan>> {
    let plans = match since {
        Some(ts) => {
            sqlx::query_as::<_, CloudPlan>(
                r#"
                SELECT * FROM cloud_plans
                WHERE user_id = $1 AND updated_at > $2
                ORDER BY updated_at ASC
                "#,
            )
            .bind(user_id)
            .bind(ts)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, CloudPlan>(
                r#"
                SELECT * FROM cloud_plans
                WHERE user_id = $1 AND deleted_at IS NULL
                ORDER BY updated_at ASC
                "#,
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(plans)
}

/// Get a specific cloud plan by user_id and local_id.
pub async fn get_plan_by_local_id(
    pool: &PgPool,
    user_id: Uuid,
    local_id: &str,
) -> Result<Option<CloudPlan>> {
    let plan = sqlx::query_as::<_, CloudPlan>(
        r#"
        SELECT * FROM cloud_plans
        WHERE user_id = $1 AND local_id = $2
        "#,
    )
    .bind(user_id)
    .bind(local_id)
    .fetch_optional(pool)
    .await?;

    Ok(plan)
}

/// Soft-delete a cloud plan (set deleted_at, increment version).
pub async fn soft_delete_plan(
    pool: &PgPool,
    user_id: Uuid,
    local_id: &str,
) -> Result<Option<CloudPlan>> {
    let plan = sqlx::query_as::<_, CloudPlan>(
        r#"
        UPDATE cloud_plans
        SET deleted_at = NOW(), version = version + 1
        WHERE user_id = $1 AND local_id = $2 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(local_id)
    .fetch_optional(pool)
    .await?;

    Ok(plan)
}

// ── Sync Event Queries ───────────────────────────────────────

/// Record a sync event for audit trail.
pub async fn record_sync_event(
    pool: &PgPool,
    user_id: Uuid,
    device_id: Option<Uuid>,
    event_type: &str,
    local_id: &str,
    payload: &serde_json::Value,
) -> Result<SyncEvent> {
    let event = sqlx::query_as::<_, SyncEvent>(
        r#"
        INSERT INTO sync_events (user_id, device_id, event_type, local_id, payload)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(device_id)
    .bind(event_type)
    .bind(local_id)
    .bind(payload)
    .fetch_one(pool)
    .await?;

    Ok(event)
}

/// Get sync events since a timestamp (for pull sync).
pub async fn get_sync_events_since(
    pool: &PgPool,
    user_id: Uuid,
    since: DateTime<Utc>,
) -> Result<Vec<SyncEvent>> {
    let events = sqlx::query_as::<_, SyncEvent>(
        r#"
        SELECT * FROM sync_events
        WHERE user_id = $1 AND synced_at > $2
        ORDER BY synced_at ASC
        "#,
    )
    .bind(user_id)
    .bind(since)
    .fetch_all(pool)
    .await?;

    Ok(events)
}

// ── Refresh Token Queries ────────────────────────────────────

/// Store a hashed refresh token.
pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Validate and consume a refresh token (mark as revoked).
/// Returns the user_id if token is valid and not expired/revoked.
pub async fn validate_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<Uuid>> {
    let result = sqlx::query_as::<_, (Uuid,)>(
        r#"
        UPDATE refresh_tokens
        SET revoked = TRUE
        WHERE token_hash = $1 AND revoked = FALSE AND expires_at > NOW()
        RETURNING user_id
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|r| r.0))
}

/// Revoke all refresh tokens for a user (e.g., on password change).
pub async fn revoke_all_refresh_tokens(pool: &PgPool, user_id: Uuid) -> Result<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// ── Family Queries ───────────────────────────────────────────

/// Create a family group (owner is automatically added as 'owner' member).
pub async fn create_family_group(
    pool: &PgPool,
    owner_id: Uuid,
    name: &str,
) -> Result<FamilyGroup> {
    let group = sqlx::query_as::<_, FamilyGroup>(
        r#"
        INSERT INTO family_groups (name, owner_id)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(owner_id)
    .fetch_one(pool)
    .await?;

    // Add owner as a member
    sqlx::query(
        r#"
        INSERT INTO family_members (family_id, user_id, email, role, accepted_at)
        VALUES ($1, $2, (SELECT email FROM users WHERE id = $2), 'owner', NOW())
        "#,
    )
    .bind(group.id)
    .bind(owner_id)
    .execute(pool)
    .await?;

    Ok(group)
}

/// Get family group(s) a user belongs to.
pub async fn get_user_families(pool: &PgPool, user_id: Uuid) -> Result<Vec<FamilyGroup>> {
    let families = sqlx::query_as::<_, FamilyGroup>(
        r#"
        SELECT fg.* FROM family_groups fg
        JOIN family_members fm ON fm.family_id = fg.id
        WHERE fm.user_id = $1
        ORDER BY fg.created_at
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(families)
}

/// Get all members of a family group.
pub async fn get_family_members(
    pool: &PgPool,
    family_id: Uuid,
) -> Result<Vec<FamilyMember>> {
    let members = sqlx::query_as::<_, FamilyMember>(
        "SELECT * FROM family_members WHERE family_id = $1 ORDER BY created_at",
    )
    .bind(family_id)
    .fetch_all(pool)
    .await?;

    Ok(members)
}

/// Create a family member invite.
pub async fn create_family_invite(
    pool: &PgPool,
    family_id: Uuid,
    email: &str,
    invite_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<FamilyMember> {
    let member = sqlx::query_as::<_, FamilyMember>(
        r#"
        INSERT INTO family_members (family_id, email, role, invite_token, invite_expires_at)
        VALUES ($1, $2, 'member', $3, $4)
        RETURNING *
        "#,
    )
    .bind(family_id)
    .bind(email)
    .bind(invite_token)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(member)
}

/// Accept a family invite (link user to member record).
pub async fn accept_family_invite(
    pool: &PgPool,
    invite_token: &str,
    user_id: Uuid,
) -> Result<Option<FamilyMember>> {
    let member = sqlx::query_as::<_, FamilyMember>(
        r#"
        UPDATE family_members
        SET user_id = $1, invite_token = NULL, accepted_at = NOW()
        WHERE invite_token = $2 AND invite_expires_at > NOW() AND accepted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(invite_token)
    .fetch_optional(pool)
    .await?;

    Ok(member)
}

/// Share a plan with a family member (or entire family if shared_with is None).
pub async fn share_plan(
    pool: &PgPool,
    family_id: Uuid,
    plan_id: Uuid,
    shared_by: Uuid,
    shared_with: Option<Uuid>,
) -> Result<SharedPlan> {
    let shared = sqlx::query_as::<_, SharedPlan>(
        r#"
        INSERT INTO shared_plans (family_id, plan_id, shared_by, shared_with)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(family_id)
    .bind(plan_id)
    .bind(shared_by)
    .bind(shared_with)
    .fetch_one(pool)
    .await?;

    Ok(shared)
}

/// Get plans shared with a user (either directly or to their entire family).
pub async fn get_shared_plans(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CloudPlan>> {
    let plans = sqlx::query_as::<_, CloudPlan>(
        r#"
        SELECT cp.* FROM cloud_plans cp
        JOIN shared_plans sp ON sp.plan_id = cp.id
        JOIN family_members fm ON fm.family_id = sp.family_id
        WHERE fm.user_id = $1
          AND (sp.shared_with = $1 OR sp.shared_with IS NULL)
          AND cp.deleted_at IS NULL
        ORDER BY cp.updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(plans)
}

// ── Device Queries ───────────────────────────────────────────

/// Register or update a device for a user.
pub async fn upsert_device(
    pool: &PgPool,
    user_id: Uuid,
    device_name: &str,
    platform: &str,
) -> Result<Device> {
    let device = sqlx::query_as::<_, Device>(
        r#"
        INSERT INTO devices (user_id, device_name, platform)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(device_name)
    .bind(platform)
    .fetch_one(pool)
    .await?;

    Ok(device)
}

/// Update last_seen and last_sync for a device.
pub async fn update_device_sync(
    pool: &PgPool,
    device_id: Uuid,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE devices SET last_seen = NOW(), last_sync = NOW()
        WHERE id = $1
        "#,
    )
    .bind(device_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get all devices for a user.
pub async fn get_user_devices(pool: &PgPool, user_id: Uuid) -> Result<Vec<Device>> {
    let devices = sqlx::query_as::<_, Device>(
        "SELECT * FROM devices WHERE user_id = $1 ORDER BY last_seen DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(devices)
}
