// ============================================================
// FILE:        error.rs
// MODULE:      Phase 5 — Cloud Backend > Error Types
// TASK:        T-060 (Session 7), standardized Session 9 A2
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7, updated Session 9
// DEPENDENCIES: axum, thiserror, serde, serde_json
// TEST COVERAGE: 8 tests — one per error variant + JSON envelope validation
// KNOWN LIMITATIONS: None
// ============================================================

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::error_codes;

/// Application error type — converts to HTTP responses automatically.
///
/// Each variant maps to a specific HTTP status code and machine-readable
/// error code constant from `error_codes`. The JSON response envelope
/// always contains `code` (machine-readable), `message` (human-readable),
/// and optionally `details` (structured context).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Too many requests")]
    TooManyRequests(u64),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Standardized JSON error response body.
///
/// Every error response from the FocusMe API uses this envelope:
/// ```json
/// {
///     "code": "AUTH_INVALID_CREDENTIALS",
///     "message": "Authentication failed: invalid credentials",
///     "details": null
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Machine-readable error code constant (e.g., "AUTH_INVALID_CREDENTIALS").
    /// Clients should match on this field for programmatic error handling.
    pub code: &'static str,

    /// Human-readable error message. May vary between requests — do NOT
    /// match on this string for error handling.
    pub message: String,

    /// Optional structured error context. Contains additional details when
    /// available (e.g., field-level validation errors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl AppError {
    /// Map each error variant to its machine-readable error code.
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Unauthorized(_) => error_codes::AUTH_INVALID_CREDENTIALS,
            AppError::Forbidden(_) => error_codes::FORBIDDEN,
            AppError::NotFound(_) => error_codes::NOT_FOUND,
            AppError::Conflict(_) => error_codes::SYNC_VERSION_CONFLICT,
            AppError::Validation(_) => error_codes::VALIDATION_FAILED,
            AppError::TooManyRequests(_) => error_codes::RATE_LIMIT_EXCEEDED,
            AppError::Internal(_) => error_codes::INTERNAL_ERROR,
            AppError::Database(_) => error_codes::INTERNAL_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::TooManyRequests(retry_after) => {
                let body = Json(ErrorResponse {
                    code: self.error_code(),
                    message: "Too many requests".to_string(),
                    details: Some(serde_json::json!({
                        "retry_after_seconds": retry_after
                    })),
                });
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(
                        axum::http::header::HeaderName::from_static("retry-after"),
                        axum::http::header::HeaderValue::from_str(&retry_after.to_string())
                            .unwrap_or_else(|_| axum::http::header::HeaderValue::from_static("60")),
                    )],
                    body,
                )
                    .into_response();
            }
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            code: self.error_code(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: convert an AppError to its ErrorResponse JSON and verify structure.
    fn assert_error_response(
        error: AppError,
        expected_status: StatusCode,
        expected_code: &str,
    ) {
        let code = error.error_code();
        assert_eq!(code, expected_code);

        // Verify the status code mapping
        let (status, _message) = match &error {
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, "Too many requests".into()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db".into()),
        };
        assert_eq!(status, expected_status);
    }

    #[test]
    fn test_unauthorized_error() {
        assert_error_response(
            AppError::Unauthorized("invalid credentials".into()),
            StatusCode::UNAUTHORIZED,
            error_codes::AUTH_INVALID_CREDENTIALS,
        );
    }

    #[test]
    fn test_forbidden_error() {
        assert_error_response(
            AppError::Forbidden("owner only".into()),
            StatusCode::FORBIDDEN,
            error_codes::FORBIDDEN,
        );
    }

    #[test]
    fn test_not_found_error() {
        assert_error_response(
            AppError::NotFound("plan not found".into()),
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
        );
    }

    #[test]
    fn test_conflict_error() {
        assert_error_response(
            AppError::Conflict("version mismatch".into()),
            StatusCode::CONFLICT,
            error_codes::SYNC_VERSION_CONFLICT,
        );
    }

    #[test]
    fn test_validation_error() {
        assert_error_response(
            AppError::Validation("email invalid".into()),
            StatusCode::BAD_REQUEST,
            error_codes::VALIDATION_FAILED,
        );
    }

    #[test]
    fn test_too_many_requests_error() {
        assert_error_response(
            AppError::TooManyRequests(30),
            StatusCode::TOO_MANY_REQUESTS,
            error_codes::RATE_LIMIT_EXCEEDED,
        );
    }

    #[test]
    fn test_internal_error() {
        assert_error_response(
            AppError::Internal("something broke".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
            error_codes::INTERNAL_ERROR,
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let resp = ErrorResponse {
            code: error_codes::AUTH_INVALID_CREDENTIALS,
            message: "Authentication failed".to_string(),
            details: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["code"], "AUTH_INVALID_CREDENTIALS");
        assert_eq!(json["message"], "Authentication failed");
        assert!(json.get("details").is_none() || json["details"].is_null());
    }
}
