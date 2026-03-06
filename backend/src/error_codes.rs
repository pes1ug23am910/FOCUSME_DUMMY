// ============================================================
// FILE:        error_codes.rs
// MODULE:      Phase 5 — Cloud Backend > Error Code Constants
// TASK:        Session 9 A2 (error standardization)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 9
// DEPENDENCIES: None
// TEST COVERAGE: Validated via error.rs tests
// KNOWN LIMITATIONS: None
// ============================================================

//! Machine-readable error code constants for the FocusMe Cloud API.
//!
//! These constants are used in the `code` field of every JSON error response,
//! allowing clients to programmatically handle specific errors without parsing
//! human-readable messages. Each constant maps 1:1 to an `AppError` variant
//! or a specific sub-case within a variant.

// ── Authentication Errors ───────────────────────────────────

/// Login failed — email not found or password incorrect.
pub const AUTH_INVALID_CREDENTIALS: &str = "AUTH_INVALID_CREDENTIALS";

/// Registration failed — an account with this email already exists.
pub const AUTH_EMAIL_EXISTS: &str = "AUTH_EMAIL_EXISTS";

/// JWT access token has expired. Client should call POST /auth/refresh.
pub const AUTH_TOKEN_EXPIRED: &str = "AUTH_TOKEN_EXPIRED";

/// JWT access token is malformed, has an invalid signature, or is otherwise invalid.
pub const AUTH_TOKEN_INVALID: &str = "AUTH_TOKEN_INVALID";

/// Refresh token was already used — indicates possible token theft.
/// All refresh tokens for this user have been revoked.
pub const AUTH_REFRESH_REUSE: &str = "AUTH_REFRESH_REUSE";

// ── Sync Errors ─────────────────────────────────────────────

/// Push/upsert failed — server version is newer than client version.
/// Client must pull first and resolve conflicts locally.
pub const SYNC_VERSION_CONFLICT: &str = "SYNC_VERSION_CONFLICT";

/// The requested plan (by local_id) was not found for this user.
pub const SYNC_PLAN_NOT_FOUND: &str = "SYNC_PLAN_NOT_FOUND";

// ── Family Errors ───────────────────────────────────────────

/// Family invite token has expired (>7 days old).
pub const FAMILY_INVITE_EXPIRED: &str = "FAMILY_INVITE_EXPIRED";

/// User is already a member of this family group.
pub const FAMILY_ALREADY_MEMBER: &str = "FAMILY_ALREADY_MEMBER";

// ── General Errors ──────────────────────────────────────────

/// Request body failed validation (missing fields, invalid format, etc.).
pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";

/// Rate limit exceeded. Check the `Retry-After` header for wait time.
pub const RATE_LIMIT_EXCEEDED: &str = "RATE_LIMIT_EXCEEDED";

/// Access denied — authenticated but insufficient permissions for this resource.
pub const FORBIDDEN: &str = "FORBIDDEN";

/// Requested resource was not found.
pub const NOT_FOUND: &str = "NOT_FOUND";

/// An unexpected internal error occurred. Details are logged server-side.
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
