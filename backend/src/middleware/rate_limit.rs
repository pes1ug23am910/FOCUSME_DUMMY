// ============================================================
// FILE:        middleware/rate_limit.rs
// MODULE:      Phase 5 — Cloud Backend > Rate Limiting
// TASK:        T-060 (Session 8 hardening)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 8
// DEPENDENCIES: tokio, axum, tower, std::collections::HashMap
// TEST COVERAGE: Unit tests for token bucket logic
// KNOWN LIMITATIONS: In-process only — not shared across instances.
//   Revisit with Redis if horizontal scaling required (see D-015).
// ============================================================

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

// ── Configuration ───────────────────────────────────────────

/// Maximum requests per minute for authentication endpoints.
const AUTH_RATE_LIMIT: f64 = 10.0;

/// Maximum requests per minute for general API endpoints.
const API_RATE_LIMIT: f64 = 100.0;

/// Cleanup interval — remove idle buckets older than this.
const BUCKET_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Cleanup task interval.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60); // 1 minute

// ── Token Bucket ────────────────────────────────────────────

/// A single token bucket for rate limiting.
#[derive(Debug, Clone)]
struct Bucket {
    /// Current number of tokens available.
    tokens: f64,

    /// When the bucket was last refilled.
    last_refill: Instant,

    /// Last time any request hit this bucket (for TTL cleanup).
    last_access: Instant,

    /// Tokens added per second (rate / 60).
    rate_per_sec: f64,

    /// Maximum tokens the bucket can hold.
    capacity: f64,
}

impl Bucket {
    fn new(requests_per_minute: f64) -> Self {
        let now = Instant::now();
        Self {
            tokens: requests_per_minute,
            last_refill: now,
            last_access: now,
            rate_per_sec: requests_per_minute / 60.0,
            capacity: requests_per_minute,
        }
    }

    /// Attempt to consume one token. Returns true if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        self.last_access = Instant::now();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;
    }

    /// Seconds until the next token becomes available.
    fn retry_after_secs(&self) -> u64 {
        if self.tokens >= 1.0 {
            return 0;
        }
        let deficit = 1.0 - self.tokens;
        (deficit / self.rate_per_sec).ceil() as u64
    }

    /// Whether this bucket has been idle longer than TTL.
    fn is_expired(&self) -> bool {
        self.last_access.elapsed() > BUCKET_TTL
    }
}

// ── Rate Limiter State ──────────────────────────────────────

/// Shared rate limiter state keyed by (IP, route category).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RouteCategory {
    Auth,
    Api,
}

/// Composite key for rate limit buckets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BucketKey {
    ip: IpAddr,
    category: RouteCategory,
}

/// Shared rate limiter across all requests.
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<BucketKey, Bucket>>>,
}

impl RateLimiter {
    /// Create a new rate limiter and spawn the background cleanup task.
    pub fn new() -> Self {
        let limiter = Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        };

        // Spawn background cleanup task to prevent memory leaks.
        let cleanup_buckets = limiter.buckets.clone();
        tokio::spawn(async move {
            cleanup_task(cleanup_buckets).await;
        });

        limiter
    }

    /// Check whether a request from `ip` to the given route category is allowed.
    async fn check(&self, ip: IpAddr, category: RouteCategory) -> Result<(), u64> {
        let mut buckets = self.buckets.lock().await;

        let rate = match category {
            RouteCategory::Auth => AUTH_RATE_LIMIT,
            RouteCategory::Api => API_RATE_LIMIT,
        };

        let key = BucketKey { ip, category };
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| Bucket::new(rate));

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(bucket.retry_after_secs())
        }
    }
}

/// Background task that periodically removes expired buckets.
async fn cleanup_task(buckets: Arc<Mutex<HashMap<BucketKey, Bucket>>>) {
    loop {
        tokio::time::sleep(CLEANUP_INTERVAL).await;

        let mut map = buckets.lock().await;
        let before = map.len();
        map.retain(|_, bucket| !bucket.is_expired());
        let removed = before - map.len();

        if removed > 0 {
            tracing::debug!(
                removed = removed,
                remaining = map.len(),
                "Rate limiter cleanup: removed expired buckets"
            );
        }
    }
}

// ── Axum Middleware ─────────────────────────────────────────

/// Extract the client IP address from the request.
///
/// Priority:
/// 1. `X-Real-IP` header (set by reverse proxy)
/// 2. `X-Forwarded-For` header (first entry)
/// 3. Socket peer address (direct connection)
fn extract_client_ip<B>(req: &Request<B>) -> IpAddr {
    // Try X-Real-IP header first (set by nginx/Caddy)
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // Try X-Forwarded-For header (first IP in chain)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first) = val.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Fall back to socket peer address
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]))
}

/// Determine the route category from the request path.
fn classify_route(path: &str) -> RouteCategory {
    if path.starts_with("/api/v1/auth/") || path == "/api/v1/auth" {
        RouteCategory::Auth
    } else {
        RouteCategory::Api
    }
}

/// Axum middleware function for rate limiting.
///
/// Apply this as a global layer on the router. Auth routes are limited to
/// 10 req/min per IP; all other routes to 100 req/min per IP.
///
/// Returns `429 Too Many Requests` with a `Retry-After` header when the
/// limit is exceeded.
pub async fn rate_limit_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Get the rate limiter from request extensions.
    let limiter = req
        .extensions()
        .get::<RateLimiter>()
        .cloned();

    let Some(limiter) = limiter else {
        // If no rate limiter is configured, pass through.
        return next.run(req).await;
    };

    let ip = extract_client_ip(&req);
    let category = classify_route(req.uri().path());

    match limiter.check(ip, category).await {
        Ok(()) => next.run(req).await,
        Err(retry_after) => {
            warn!(
                client_ip = %ip,
                category = ?category,
                retry_after = retry_after,
                "Rate limit exceeded"
            );

            Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Retry-After", retry_after.to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"error":"Too many requests","code":429,"retry_after":{}}}"#,
                    retry_after
                )))
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .body(Body::empty())
                        .unwrap()
                })
        }
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_initial_state() {
        let bucket = Bucket::new(10.0);
        assert_eq!(bucket.capacity, 10.0);
        assert!(bucket.tokens >= 9.99);
    }

    #[test]
    fn test_bucket_consume() {
        let mut bucket = Bucket::new(2.0);
        assert!(bucket.try_consume()); // 1 left
        assert!(bucket.try_consume()); // 0 left
        assert!(!bucket.try_consume()); // denied
    }

    #[test]
    fn test_bucket_retry_after() {
        let mut bucket = Bucket::new(60.0); // 1 per second
        // Drain all tokens
        for _ in 0..60 {
            bucket.try_consume();
        }
        let retry = bucket.retry_after_secs();
        assert!(retry >= 1, "retry_after should be at least 1 second");
    }

    #[test]
    fn test_bucket_expired() {
        let mut bucket = Bucket::new(10.0);
        // Manually set last_access to the past
        bucket.last_access = Instant::now() - Duration::from_secs(600);
        assert!(bucket.is_expired());
    }

    #[test]
    fn test_bucket_not_expired() {
        let bucket = Bucket::new(10.0);
        assert!(!bucket.is_expired());
    }

    #[test]
    fn test_classify_route_auth() {
        assert_eq!(classify_route("/api/v1/auth/register"), RouteCategory::Auth);
        assert_eq!(classify_route("/api/v1/auth/login"), RouteCategory::Auth);
        assert_eq!(classify_route("/api/v1/auth/refresh"), RouteCategory::Auth);
    }

    #[test]
    fn test_classify_route_api() {
        assert_eq!(classify_route("/api/v1/plans"), RouteCategory::Api);
        assert_eq!(classify_route("/api/v1/sync/push"), RouteCategory::Api);
        assert_eq!(classify_route("/api/v1/family"), RouteCategory::Api);
        assert_eq!(classify_route("/health"), RouteCategory::Api);
    }
}
