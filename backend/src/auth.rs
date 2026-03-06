// ============================================================
// FILE:        auth.rs
// MODULE:      Phase 5 — Cloud Backend > Authentication
// TASK:        T-061
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: argon2 0.5, jsonwebtoken 9, axum 0.7
// TEST COVERAGE: register, login, verify_token, refresh, middleware
// KNOWN LIMITATIONS:
//   - No email verification flow yet (email_verified always false).
//   - No rate limiting on login — should be handled at reverse-proxy level.
//   - No password complexity enforcement — client-side responsibility.
// ============================================================

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Algorithm, Params, Version,
};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;

// ── Constants ────────────────────────────────────────────────

/// Access token lifetime: 15 minutes.
const ACCESS_TOKEN_EXPIRY_MINUTES: i64 = 15;

/// Refresh token lifetime: 30 days.
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 30;

/// Argon2id memory cost: 64 MiB (OWASP recommendation).
const ARGON2_MEMORY_KIB: u32 = 65_536;

/// Argon2id time cost: 3 iterations.
const ARGON2_TIME_COST: u32 = 3;

/// Argon2id parallelism: 4 lanes.
const ARGON2_PARALLELISM: u32 = 4;

// ── JWT Claims ───────────────────────────────────────────────

/// JWT claims payload.
///
/// Access tokens carry `token_type: "access"`.
/// Refresh tokens carry `token_type: "refresh"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — user UUID.
    pub sub: String,
    /// User email.
    pub email: String,
    /// Token type: "access" or "refresh".
    pub token_type: String,
    /// Issued at (epoch seconds).
    pub iat: i64,
    /// Expiry (epoch seconds).
    pub exp: i64,
}

/// Token pair returned after login or refresh.
#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

// ── AuthService ──────────────────────────────────────────────

/// Core authentication service — handles registration, login,
/// token issuance, verification, and refresh rotation.
#[derive(Clone)]
pub struct AuthService {
    db: PgPool,
    jwt_secret: String,
}

impl AuthService {
    /// Create a new AuthService.
    pub fn new(db: PgPool, jwt_secret: String) -> Self {
        Self { db, jwt_secret }
    }

    // ── Registration ─────────────────────────────────────────

    /// Register a new user with email + password.
    ///
    /// 1. Validate email format (basic check).
    /// 2. Check for duplicate email → 409 Conflict.
    /// 3. Hash password with Argon2id (m=64MiB, t=3, p=4).
    /// 4. Insert user record.
    /// 5. Issue token pair.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(db::UserResponse, TokenPair), AppError> {
        // Validate email (basic format check)
        if !email.contains('@') || email.len() < 5 {
            return Err(AppError::Validation("Invalid email address".to_string()));
        }

        // Validate password length
        if password.len() < 8 {
            return Err(AppError::Validation(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // Check for duplicate email
        if db::find_user_by_email(&self.db, email)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .is_some()
        {
            return Err(AppError::Conflict(
                "An account with this email already exists".to_string(),
            ));
        }

        // Hash password with Argon2id
        let password_hash = self.hash_password(password)?;

        // Create user
        let user = db::create_user(&self.db, email, &password_hash)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Issue token pair
        let tokens = self.issue_token_pair(&user.id, &user.email).await?;

        Ok((user.into(), tokens))
    }

    // ── Login ────────────────────────────────────────────────

    /// Authenticate a user with email + password.
    ///
    /// 1. Find user by email → 401 if not found.
    /// 2. Verify password against Argon2id hash → 401 if mismatch.
    /// 3. Issue new token pair.
    /// 4. Store refresh token hash.
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(db::UserResponse, TokenPair), AppError> {
        // Find user
        let user = db::find_user_by_email(&self.db, email)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

        // Verify password
        if !self.verify_password(password, &user.password_hash)? {
            return Err(AppError::Unauthorized(
                "Invalid email or password".to_string(),
            ));
        }

        // Issue token pair
        let tokens = self.issue_token_pair(&user.id, &user.email).await?;

        Ok((user.into(), tokens))
    }

    // ── Token Verification ───────────────────────────────────

    /// Verify and decode a JWT access token.
    ///
    /// Returns the Claims on success.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            tracing::debug!(error = %e, "Token verification failed");
            AppError::Unauthorized("Invalid or expired token".to_string())
        })?;

        // Ensure it's an access token
        if token_data.claims.token_type != "access" {
            return Err(AppError::Unauthorized(
                "Invalid token type — expected access token".to_string(),
            ));
        }

        Ok(token_data.claims)
    }

    // ── Token Refresh ────────────────────────────────────────

    /// Refresh an expired access token using a valid refresh token.
    ///
    /// This implements **refresh token rotation**:
    /// 1. Decode refresh token.
    /// 2. Hash it and look up in DB → 401 if not found or already revoked.
    /// 3. Revoke the old refresh token (one-time use).
    /// 4. Issue a new token pair.
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AppError> {
        // Decode the refresh token
        let token_data = decode::<Claims>(
            refresh_token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("Invalid or expired refresh token".to_string()))?;

        if token_data.claims.token_type != "refresh" {
            return Err(AppError::Unauthorized(
                "Invalid token type — expected refresh token".to_string(),
            ));
        }

        // Hash the refresh token for DB lookup
        let token_hash = Self::sha256_hash(refresh_token);

        // Validate and atomically revoke the refresh token
        let valid = db::validate_refresh_token(&self.db, &token_hash)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !valid {
            // Token reuse detected — revoke all tokens for this user (security measure)
            let user_id = Uuid::parse_str(&token_data.claims.sub)
                .map_err(|_| AppError::Unauthorized("Invalid token subject".to_string()))?;
            let _ = db::revoke_all_refresh_tokens(&self.db, user_id).await;

            tracing::warn!(
                user_id = %token_data.claims.sub,
                "Refresh token reuse detected — all tokens revoked"
            );

            return Err(AppError::Unauthorized(
                "Refresh token has been revoked".to_string(),
            ));
        }

        // Parse user ID
        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid token subject".to_string()))?;

        // Issue new token pair
        let tokens = self
            .issue_token_pair(&user_id, &token_data.claims.email)
            .await?;

        Ok(tokens)
    }

    // ── Internal Helpers ─────────────────────────────────────

    /// Hash a password with Argon2id using OWASP-recommended parameters.
    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(
            ARGON2_MEMORY_KIB,
            ARGON2_TIME_COST,
            ARGON2_PARALLELISM,
            None,
        )
        .map_err(|e| AppError::Internal(format!("Argon2 params error: {e}")))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(format!("Password hashing failed: {e}")))?
            .to_string();

        Ok(hash)
    }

    /// Verify a password against an Argon2id hash.
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::Internal(format!("Invalid password hash format: {e}")))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Issue a new access + refresh token pair.
    ///
    /// Access token: 15 minutes.
    /// Refresh token: 30 days, stored as SHA-256 hash in DB.
    async fn issue_token_pair(
        &self,
        user_id: &Uuid,
        email: &str,
    ) -> Result<TokenPair, AppError> {
        let now = Utc::now();

        // Access token
        let access_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            token_type: "access".to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::minutes(ACCESS_TOKEN_EXPIRY_MINUTES)).timestamp(),
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Failed to encode access token: {e}")))?;

        // Refresh token
        let refresh_exp = now + Duration::days(REFRESH_TOKEN_EXPIRY_DAYS);
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            token_type: "refresh".to_string(),
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Failed to encode refresh token: {e}")))?;

        // Store refresh token hash in DB
        let token_hash = Self::sha256_hash(&refresh_token);
        db::store_refresh_token(&self.db, *user_id, &token_hash, refresh_exp)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in: ACCESS_TOKEN_EXPIRY_MINUTES * 60,
        })
    }

    /// SHA-256 hash a token for database storage.
    ///
    /// We never store raw tokens — only their hashes.
    fn sha256_hash(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// ── Auth Middleware ──────────────────────────────────────────

/// Axum middleware that extracts and validates the Bearer token
/// from the `Authorization` header, injecting `Claims` into
/// request extensions for downstream handlers.
///
/// Usage:
/// ```rust
/// use axum::middleware;
/// Router::new()
///     .route("/protected", get(handler))
///     .route_layer(middleware::from_fn_with_state(state, auth_middleware))
/// ```
pub async fn auth_middleware(
    State(state): State<crate::AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract the Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    // Parse "Bearer <token>"
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            AppError::Unauthorized("Invalid Authorization header format".to_string())
        })?;

    // Verify the token
    let claims = state.auth.verify_token(token)?;

    // Inject claims into request extensions for downstream handlers
    req.extensions_mut().insert(claims);

    // Continue to the next handler
    Ok(next.run(req).await)
}
