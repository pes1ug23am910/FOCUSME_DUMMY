// ============================================================
// FILE:        main.rs
// MODULE:      Phase 5 — Cloud Backend > Entry Point
// TASK:        T-060
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 7
// DEPENDENCIES: axum 0.7, tokio, tracing, sqlx, dotenvy
// TEST COVERAGE: Health endpoint, graceful shutdown
// KNOWN LIMITATIONS: No TLS termination — expects reverse proxy (nginx/Caddy).
// ============================================================

mod analytics;
mod analytics_schema;
mod auth;
mod db;
mod error;
mod error_codes;
mod middleware;
mod routes;

#[cfg(test)]
mod tests;

use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, error};

use crate::analytics::AnalyticsClient;
use crate::auth::AuthService;
use crate::middleware::RateLimiter;

/// Shared application state — passed to all route handlers via Axum State.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth: Arc<AuthService>,
    pub analytics: Arc<AnalyticsClient>,
    pub rate_limiter: RateLimiter,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// GET /health — returns 200 with version info.
/// Used by load balancers, Docker HEALTHCHECK, and monitoring.
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Build the Axum router with all route groups.
///
/// Route structure:
/// ```
/// GET  /health                     — health check (unauthenticated)
/// POST /api/v1/auth/register       — user registration
/// POST /api/v1/auth/login          — user login → JWT pair
/// POST /api/v1/auth/refresh        — refresh token rotation
/// GET  /api/v1/plans               — list plans (delta sync via ?since=)
/// POST /api/v1/plans               — create/upsert plan
/// DELETE /api/v1/plans/:local_id   — soft-delete plan
/// POST /api/v1/sync/push           — batch push local changes
/// GET  /api/v1/sync/pull           — pull changes since last sync
/// POST /api/v1/family/invite       — invite family member
/// GET  /api/v1/family/members      — list family members
/// POST /api/v1/family/plans/share/:plan_id — share plan with member
/// GET  /api/v1/family/dashboard    — aggregate family dashboard
/// ```
pub fn build_router(state: AppState) -> Router {
    use axum::middleware as mw;
    use crate::auth::auth_middleware;
    use crate::middleware::{rate_limit_middleware, security_headers_middleware, request_id_middleware};

    let cors = CorsLayer::new()
        .allow_origin(Any) // TODO: Restrict to FRONTEND_URL in production
        .allow_methods(Any)
        .allow_headers(Any);

    // Protected routes — require valid Bearer JWT access token.
    let protected = Router::new()
        .nest("/plans", routes::plan_routes())
        .nest("/sync", routes::sync_routes())
        .nest("/family", routes::family_routes())
        .route_layer(mw::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .route("/health", get(health_handler))
        .nest("/health", routes::health_check_routes())
        // Auth routes are public (register, login, refresh).
        .nest("/api/v1/auth", routes::auth_routes())
        // All other /api/v1/* routes require authentication.
        .nest("/api/v1", protected)
        // Middleware layers — applied bottom-up (last added = outermost).
        // 1. Tracing (outermost — logs all requests including rate-limited)
        .layer(TraceLayer::new_for_http())
        // 2. Request ID (generates UUID per request, adds X-Request-Id header)
        .layer(mw::from_fn(request_id_middleware))
        // 3. CORS headers
        .layer(cors)
        // 4. Security headers on every response
        .layer(mw::from_fn(security_headers_middleware))
        // 5. Rate limiting (innermost layer before routing)
        .layer(mw::from_fn(rate_limit_middleware))
        // 6. Inject rate limiter into request extensions
        .layer(axum::Extension(state.rate_limiter.clone()))
        .with_state(state)
}

/// Main entry point — initializes database, builds router, starts server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (if present — not required in production)
    dotenvy::dotenv().ok();

    // Initialize structured tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "focusme_backend=debug,tower_http=debug".into()),
        )
        .init();

    info!("FocusMe Cloud Backend v{}", env!("CARGO_PKG_VERSION"));

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let db = db::create_pool(&database_url).await?;
    info!("Database connection established");

    // Run migrations
    db::run_migrations(&db).await?;
    info!("Database migrations complete");

    // Build application state
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");

    let auth_service = AuthService::new(db.clone(), jwt_secret);

    let rate_limiter = RateLimiter::new();
    info!("Rate limiter initialized (auth: 10/min, API: 100/min per IP)");

    let analytics = AnalyticsClient::new();

    let state = AppState {
        db: db.clone(),
        auth: Arc::new(auth_service),
        analytics: Arc::new(analytics),
        rate_limiter,
    };

    // Build router
    let app = build_router(state);

    // Determine bind address
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on http://{}", addr);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shut down gracefully");
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, shutting down..."),
        _ = terminate => info!("Received SIGTERM, shutting down..."),
    }
}
