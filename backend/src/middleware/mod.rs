// ============================================================
// FILE:        middleware/mod.rs
// MODULE:      Phase 5 — Cloud Backend > Middleware Registry
// TASK:        T-060 (Session 8 hardening)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 8
// DEPENDENCIES: axum
// ============================================================

pub mod rate_limit;
pub mod request_id;
pub mod security_headers;

pub use rate_limit::{rate_limit_middleware, RateLimiter};
pub use request_id::request_id_middleware;
pub use security_headers::security_headers_middleware;
