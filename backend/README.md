# FocusMe Cloud Backend

> Phase 5 — REST API for cloud sync, authentication, and family dashboard.
> **Last updated:** Session 8

## Architecture

| Component    | Technology               | Purpose                          |
|-------------|--------------------------|----------------------------------|
| Framework   | Axum 0.7                 | Async HTTP / WebSocket server    |
| Database    | PostgreSQL 16            | JSONB plans, user data, sync     |
| ORM/Query   | sqlx 0.7                 | Compile-time checked SQL queries |
| Auth        | Argon2id + JWT           | Password hashing + bearer tokens |
| Middleware  | tower / tower-http       | CORS, tracing, rate limit, security headers |
| Analytics   | PostHog (optional)       | Usage telemetry (no-op if key absent) |
| Config      | dotenvy                  | .env file loading                |

## Prerequisites

- **Rust 1.76+** with `cargo`
- **Docker** & **Docker Compose** (for PostgreSQL)
- **sqlx-cli** — `cargo install sqlx-cli --features postgres --no-default-features`

## Quick Start (First-Time Setup)

### 1. Start PostgreSQL

```bash
cd backend
docker compose up -d postgres
```

### 2. Configure Environment

```bash
cp .env.example .env
# Edit .env:
#   - Set JWT_SECRET to a strong random string (openssl rand -base64 64)
#   - Optionally set POSTHOG_API_KEY for analytics
```

### 3. Run Database Migrations

```bash
cargo sqlx migrate run
```

### 4. Start the Backend

```bash
cargo run
```

The server starts at `http://localhost:8080`.

### 5. Verify

```bash
curl http://localhost:8080/health
# → {"status":"ok","version":"0.1.0"}
```

## Running Tests

Tests require a running PostgreSQL instance:

```bash
# Start Postgres if not already running
docker compose up -d postgres

# Run with test database URL
DATABASE_URL=postgres://focusme:focusme_dev_password@localhost:5432/focusme \
JWT_SECRET=test_jwt_secret_at_least_32_chars_long \
cargo test
```

## Adding a Migration

```bash
# Create a new migration file
cargo sqlx migrate add <migration_name>

# Edit the generated SQL file in migrations/

# Apply migrations
cargo sqlx migrate run --database-url $DATABASE_URL

# Revert the last migration (if needed)
cargo sqlx migrate revert --database-url $DATABASE_URL
```

## Docker Deployment

### Development (PostgreSQL only)

```bash
docker compose up -d postgres
cargo run  # Run backend directly for hot-reload
```

### Production (Full Stack)

```bash
docker compose up -d  # Starts PostgreSQL + backend container
```

The backend Dockerfile uses a multi-stage build:
1. **Builder stage** — Rust 1.76 compiles the release binary
2. **Runtime stage** — Debian bookworm-slim with only `ca-certificates`, `libssl3`, `curl`
3. Runs as non-root user `focusme` (UID 1000)
4. HEALTHCHECK via `curl http://localhost:8080/health`

## API Reference

Full OpenAPI 3.1 specification: [`openapi.yml`](openapi.yml)

### Authentication (Public — no token required)

| Method | Endpoint                  | Description                | Rate Limit |
|--------|---------------------------|----------------------------|------------|
| POST   | `/api/v1/auth/register`   | Create account             | 10/min     |
| POST   | `/api/v1/auth/login`      | Login → JWT token pair     | 10/min     |
| POST   | `/api/v1/auth/refresh`    | Rotate refresh token       | 10/min     |

### Plans (Authenticated — Bearer JWT required)

| Method | Endpoint                       | Description                    | Rate Limit |
|--------|--------------------------------|--------------------------------|------------|
| GET    | `/api/v1/plans?since=`         | List plans (delta sync)        | 100/min    |
| POST   | `/api/v1/plans`                | Create or upsert plan          | 100/min    |
| DELETE | `/api/v1/plans/:local_id`      | Soft-delete plan               | 100/min    |

### Sync (Authenticated)

| Method | Endpoint               | Description                     | Rate Limit |
|--------|------------------------|---------------------------------|------------|
| POST   | `/api/v1/sync/push`    | Batch push local changes        | 100/min    |
| GET    | `/api/v1/sync/pull?since=` | Pull changes since timestamp | 100/min    |

### Family (Authenticated)

| Method | Endpoint                              | Description              | Rate Limit |
|--------|---------------------------------------|--------------------------|------------|
| POST   | `/api/v1/family`                      | Create family group      | 100/min    |
| GET    | `/api/v1/family`                      | List user's groups       | 100/min    |
| GET    | `/api/v1/family/members`              | List all members         | 100/min    |
| POST   | `/api/v1/family/invite`               | Send family invite       | 100/min    |
| POST   | `/api/v1/family/invite/accept`        | Accept invite            | 100/min    |
| POST   | `/api/v1/family/plans/share/:plan_id` | Share plan with family   | 100/min    |
| GET    | `/api/v1/family/dashboard`            | Family activity summary  | 100/min    |

## Authentication Flow

```
┌────────┐                    ┌─────────┐                  ┌────────┐
│ Client │                    │ Backend │                  │  DB    │
└───┬────┘                    └────┬────┘                  └───┬────┘
    │  POST /auth/register         │                           │
    │  {email, password}          │                           │
    │────────────────────────────►│                           │
    │                             │  Argon2id hash            │
    │                             │──────────────────────────►│
    │                             │  INSERT user              │
    │                             │◄──────────────────────────│
    │  {user, access_token,       │                           │
    │   refresh_token}            │  Store SHA-256(refresh)   │
    │◄────────────────────────────│──────────────────────────►│
    │                             │                           │
    │  GET /plans                 │                           │
    │  Authorization: Bearer JWT  │                           │
    │────────────────────────────►│                           │
    │                             │  Verify JWT               │
    │                             │  Extract Claims            │
    │  {plans: [...]}             │  Query user plans          │
    │◄────────────────────────────│◄──────────────────────────│
    │                             │                           │
    │  POST /auth/refresh         │                           │
    │  {refresh_token}            │                           │
    │────────────────────────────►│                           │
    │                             │  Validate + revoke old    │
    │  {new access_token,         │  Issue new pair           │
    │   new refresh_token}        │  Store new SHA-256 hash   │
    │◄────────────────────────────│──────────────────────────►│
```

## Sync Strategy

FocusMe uses **delta sync with optimistic concurrency**:

1. **Push first:** Client sends local changes via `POST /sync/push`.
2. **Pull second:** Client fetches server changes via `GET /sync/pull?since=<last_sync>`.
3. **Conflict detection:** Plans have a `version` field. If the client sends an
   `expected_version` that doesn't match the server's current version, the server
   returns `409 Conflict` with the server's data for client-side resolution.
4. **Soft deletes:** Deleted plans are marked with `deleted_at` rather than removed,
   so other devices can sync the deletion.

## Rate Limiting

In-process token bucket per client IP (D-015):

| Route Category | Limit | Burst |
|---------------|-------|-------|
| Auth routes (`/api/v1/auth/*`) | 10 req/min | 10 |
| All other API routes | 100 req/min | 100 |

**IP detection priority:** `X-Real-IP` → `X-Forwarded-For` (first) → socket peer.
**Exceeded:** `429 Too Many Requests` + `Retry-After` header.
**Cleanup:** Idle buckets (>5 min) removed every 60s by background task.

To adjust limits, edit the constants in `src/middleware/rate_limit.rs`:
```rust
const AUTH_RATE_LIMIT: f64 = 10.0;  // requests per minute
const API_RATE_LIMIT: f64 = 100.0;  // requests per minute
```

## Security Headers

Every response includes (via `security_headers_middleware`):

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `X-XSS-Protection` | `1; mode=block` | Legacy XSS filter |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Limit referrer leakage |
| `Content-Security-Policy` | `default-src 'none'; frame-ancestors 'none'` | API-only CSP |
| `Permissions-Policy` | `geolocation=(), microphone=(), camera=()` | Disable browser features |

The `Server` header is removed to avoid leaking software information.

## Database Schema

See [`migrations/V1__cloud_schema.sql`](migrations/V1__cloud_schema.sql) for the
full schema. Key tables:

- `users` — accounts with Argon2id password hashes
- `cloud_plans` — JSONB plan storage with version tracking
- `sync_events` — audit trail of all sync operations
- `family_groups` + `family_members` — family dashboard
- `shared_plans` — plan sharing within families
- `devices` — registered devices for push notifications
- `refresh_tokens` — JWT refresh token hashes (rotation)

## Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | **Yes** | — | PostgreSQL connection string (`postgres://user:pass@host:5432/db`) |
| `JWT_SECRET` | **Yes** | — | HS256 signing key (minimum 32 characters, use `openssl rand -base64 64`) |
| `PORT` | No | `8080` | HTTP listen port |
| `RUST_LOG` | No | `focusme_backend=debug,tower_http=debug` | Tracing filter (use `info` or `warn` in production) |
| `POSTHOG_API_KEY` | No | — | PostHog project key. Empty or absent = analytics disabled (no-op) |
| `POSTGRES_DB` | No | `focusme` | Database name (docker-compose) |
| `POSTGRES_USER` | No | `focusme` | Database user (docker-compose) |
| `POSTGRES_PASSWORD` | No | `focusme_dev_password` | Database password (docker-compose) |
| `FRONTEND_URL` | No | `*` (any) | CORS allowed origin. Set to actual frontend URL in production |

## Deployment Checklist

Before deploying to production:

- [ ] Set `DATABASE_URL` to production PostgreSQL instance
- [ ] Generate and set a strong `JWT_SECRET` (`openssl rand -base64 64`)
- [ ] Configure `FRONTEND_URL` for CORS restriction (do NOT use `*` in production)
- [ ] Enable TLS termination via reverse proxy (nginx/Caddy/cloud LB)
- [ ] Set `RUST_LOG=focusme_backend=warn,tower_http=warn`
- [ ] Configure Dependabot for backend crate dependencies
- [ ] Set up `/health` monitoring (Docker HEALTHCHECK or external uptime monitor)
- [ ] Configure PostgreSQL backups (pg_dump cron or managed DB snapshots)

## Project Structure

```
backend/
├── Cargo.toml                 # Dependencies
├── Dockerfile                 # Multi-stage production build
├── docker-compose.yml         # PostgreSQL + backend container
├── .env.example               # Environment variable template
├── README.md                  # This file
├── openapi.yml                # OpenAPI 3.1 specification
├── migrations/
│   └── V1__cloud_schema.sql   # Initial database schema (8 tables)
└── src/
    ├── main.rs                # Entry point, router assembly, shutdown
    ├── analytics.rs           # PostHog telemetry (fire-and-forget)
    ├── auth.rs                # AuthService (Argon2id + JWT + middleware)
    ├── db.rs                  # Database models + 25 query functions
    ├── error.rs               # AppError → HTTP response mapping
    ├── middleware/
    │   ├── mod.rs             # Middleware registry
    │   ├── rate_limit.rs      # Token bucket rate limiter
    │   └── security_headers.rs# Security header injection
    ├── routes/
    │   ├── mod.rs             # Route registry
    │   ├── auth_routes.rs     # POST /register, /login, /refresh
    │   ├── sync.rs            # Plan CRUD + push/pull sync
    │   └── family.rs          # Family groups, invites, dashboard
    └── tests/
        ├── mod.rs             # Test helpers
        ├── auth_test.rs       # 10 auth integration tests
        ├── sync_test.rs       # 8 sync integration tests
        └── family_test.rs     # 8 family integration tests
```

## Logging

Set `RUST_LOG` to control verbosity:

```bash
# Maximum verbosity (development)
RUST_LOG=focusme_backend=trace,tower_http=debug cargo run

# Standard development
RUST_LOG=focusme_backend=debug,tower_http=debug cargo run

# Production
RUST_LOG=focusme_backend=warn,tower_http=warn cargo run
```
