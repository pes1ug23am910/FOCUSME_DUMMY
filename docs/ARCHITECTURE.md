# FocusMe — Architecture Reference

> **FILE:** docs/ARCHITECTURE.md
> **TASK:** Track A8 (Session 6 — Polish & Hardening)
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **LAST UPDATED:** Session 8

---

## 1. System Overview

FocusMe is a cross-platform productivity enforcement application that blocks distracting websites and applications at the **operating system level**. It operates as a privileged daemon/service that intercepts DNS queries, filters network traffic, and controls process execution, coordinated through an encrypted local database and multiple user interfaces (desktop GUI, browser extension, Android app).

The system is designed around five logical layers that communicate via IPC (MessagePack primary, JSON debug), with each platform implementing its own enforcement engine using the strongest available kernel API.

```
┌─────────────────────────────────────────────────────────────┐
│                    LAYER 5: ANALYTICS                        │
│             PostHog (Phase 5 — post-MVP, optional)           │
├─────────────────────────────────────────────────────────────┤
│                    LAYER 4: BROWSER CONNECTOR                │
│  WebExtension (MV3 Chrome / MV2 Firefox) + NMH              │
├─────────────────────────────────────────────────────────────┤
│                    LAYER 3: UI SHELL                         │
│  Tauri v2 (React + TypeScript) → IPC → Daemon               │
├─────────────────────────────────────────────────────────────┤
│                    LAYER 2: POLICY STORE                     │
│  SQLCipher (WAL mode) — 10 tables, RwLock<Connection>        │
├─────────────────────────────────────────────────────────────┤
│                  LAYER 1: ENFORCEMENT ENGINE                 │
│  Windows: WFP + HOSTS      │  Linux: eBPF/Fanotify + DNS   │
│  macOS: ESF + DNS Proxy    │  Android: a11y + VPN           │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Data Flow — Block Decision Path

This section traces how a block decision travels from plan creation to enforcement.

### 2.1 Plan Creation Flow

```
User creates plan in UI
        │
        ▼
PlanWizard.tsx ──invoke("create_plan")──▶ Tauri lib.rs
        │                                      │
        │                              IPC (MessagePack)
        │                                      │
        ▼                                      ▼
Plan saved to SQLCipher ◄────── ipc_server.rs::handle_plan_create()
        │                              │
        ▼                              ▼
scheduler.rs loads plan          HOSTS entries written
        │                              │
        ▼                              ▼
Plan becomes active at           WFP/eBPF/ESF rules updated
scheduled time window
```

### 2.2 URL Block Decision (Browser)

```
User navigates to blocked URL
        │
        ▼
extension/background.ts
  ├─ MV3: chrome.declarativeNetRequest (pre-loaded rules)
  │   └─ Rule match → redirect to blocked.html
  │
  └─ MV2: chrome.webRequest.onBeforeRequest
      └─ Pattern match → cancel request → redirect to blocked.html
        │
        ▼
  (Parallel) NMH sync every 30s:
    background.ts ──NMH framing──▶ native_messaging_host/main.rs
                                          │
                                    IPC (MsgPack)
                                          │
                                          ▼
                                   ipc_server.rs::handle_url_check()
                                          │
                                    db.get_url_rules(plan_id)
                                          │
                                    domain_matches() check
                                          │
                                     BLOCK / ALLOW
```

### 2.3 App Block Decision (Windows)

```
process_monitor.rs (500ms polling loop)
        │
   CreateToolhelp32Snapshot()
        │
        ▼
For each running process:
   ipc_server.rs::handle_app_check()
        │
   db.get_app_rules(plan_id)
        │
   match_type check (process_name / path_prefix / path_exact)
        │
        ▼
   BLOCK → TerminateProcess()
   ALLOW → skip
```

### 2.4 App Block Decision (Linux — Fanotify)

```
loader.rs::spawn_fanotify_event_loop()
        │
   fanotify_init(FAN_CLOEXEC | FAN_CLASS_CONTENT)
   fanotify_mark(FAN_MARK_ADD | FAN_MARK_MOUNT, FAN_OPEN_EXEC_PERM, "/")
        │
        ▼
Kernel sends FanotifyEvent for every exec attempt
        │
   resolve fd → /proc/self/fd/{fd} readlink → path
        │
   Check path against blocked_paths HashSet
        │
        ▼
   FAN_DENY → kernel blocks exec
   FAN_ALLOW → kernel permits exec
```

### 2.5 DNS Block Decision

```
DNS query for blocked domain
        │
   ┌────┴────────────────────────────────────┐
   │              Platform-specific          │
   ├─ Windows: WFP intercepts → HOSTS file   │
   │           returns 0.0.0.0               │
   ├─ macOS: NEDNSProxyProvider intercepts    │
   │         → synthesizes NXDOMAIN response  │
   ├─ Linux: Unbound RPZ zone → NXDOMAIN     │
   │         + HOSTS file (fallback)          │
   └─ Android: VPN tunnel intercepts DNS      │
              → NXDOMAIN synthesis            │
```

---

## 3. IPC Protocol

Full specification: [ipc_protocol_v1.md](ipc_protocol_v1.md)

### 3.1 Framing

```
┌───────────┬──────────────────────┐
│ 4 bytes   │ N bytes              │
│ LE uint32 │ MessagePack payload  │
│ (length)  │ (or JSON in debug)   │
└───────────┴──────────────────────┘
```

### 3.2 Message Types

| Message | Direction | Handler | DB Access |
|---------|-----------|---------|-----------|
| `PING` | Client → Daemon | `handle_ping()` | None |
| `CONNECT` | Client → Daemon | `handle_connect()` | Read (active plans) |
| `PLAN_LIST` | Client → Daemon | `handle_plan_list()` | Read |
| `PLAN_GET` | Client → Daemon | `handle_plan_get()` | Read |
| `PLAN_CREATE` | Client → Daemon | `handle_plan_create()` | Write |
| `PLAN_UPDATE` | Client → Daemon | `handle_plan_update()` | Read + Write |
| `PLAN_DELETE` | Client → Daemon | `handle_plan_delete()` | Read + Write |
| `URL_CHECK` | Client → Daemon | `handle_url_check()` | Read |
| `APP_CHECK` | Client → Daemon | `handle_app_check()` | Read |
| `STATUS_REQUEST` | Client → Daemon | `handle_status_request()` | None (in-memory) |
| `UNLOCK_REQUEST` | Client → Daemon | `handle_unlock_request()` | Read |
| `STATS_REQUEST` | Client → Daemon | `handle_stats_request()` | Read |

### 3.3 IPC Sequence Diagram

```
  UI Shell (Tauri)              Daemon (ipc_server.rs)          DB (db.rs)
       │                              │                            │
       │──── CONNECT ─────────────▶   │                            │
       │                              │── scheduler.get_active() ─▶│
       │                              │◀── plan IDs ──────────────│
       │◀── CONNECT_RESPONSE ──────   │                            │
       │    {version, plan_count}     │                            │
       │                              │                            │
       │──── PLAN_CREATE ─────────▶   │                            │
       │    {name, rules, schedule}   │── db.create_plan() ──────▶│
       │                              │◀── Ok ────────────────────│
       │◀── PLAN_CREATE_RESPONSE ──   │                            │
       │    {plan_id, success}        │                            │
       │                              │                            │
       │──── URL_CHECK ───────────▶   │                            │
       │    {url, domain}             │── db.get_url_rules() ────▶│
       │                              │◀── rules ────────────────│
       │                              │── domain_matches() ──┐    │
       │                              │◀─────────────────────┘    │
       │◀── URL_CHECK_RESPONSE ────   │                            │
       │    {decision: BLOCK/ALLOW}   │                            │
```

---

## 4. Database Schema

**Engine:** SQLCipher (SQLite + AES-256 CBC encryption)
**Mode:** WAL (Write-Ahead Logging) for concurrent read performance
**Lock:** `tokio::sync::RwLock<Connection>` (D-013) — 11 read / 13 write methods
**Key derivation:** SHA-256(machine-id + "FocusMe-Policy-Store-v1")

### 4.1 Entity Relationships

```
                    ┌──────────────────┐
                    │     settings     │
                    │ key | value      │
                    └──────────────────┘

┌────────────┐     1:N     ┌──────────────┐
│   plans    │────────────▶│  schedules   │
│            │             │ days, times  │
│ plan_id PK │             │ timezone     │
│ name       │             └──────────────┘
│ enabled    │
│ forced_mode│     1:N     ┌──────────────┐
│ protection │────────────▶│  app_rules   │
│ plan_json  │             │ type, match  │
│            │             │ value, order │
│            │             └──────────────┘
│            │
│            │     1:N     ┌──────────────┐
│            │────────────▶│  url_rules   │
│            │             │ type, match  │
│            │             │ value, order │
│            │             └──────────────┘
│            │
│            │     1:N     ┌──────────────┐     1:N     ┌────────────────┐
│            │────────────▶│   quotas     │────────────▶│ quota_ledger   │
│            │             │ target, limit│             │ date, used_s   │
│            │             └──────────────┘             │ launch_count   │
│            │                                          └────────────────┘
│            │     1:N     ┌──────────────┐
│            │────────────▶│  sessions    │
│            │             │ started/ended│
│            │             │ forced_mode  │
│            │             └──────────────┘
│            │
│            │     1:N     ┌──────────────┐
│            │────────────▶│   events     │
│            │             │ type, hash   │
│            │             │ timestamp    │
│            │             └──────────────┘
└────────────┘
                    ┌──────────────────────┐
                    │ forced_mode_state    │
                    │ plan_id, timestamps  │
                    │ monotonic durations  │
                    │ emergency_code_hash  │
                    └──────────────────────┘
```

### 4.2 Tables Summary

| Table | Rows | Purpose |
|-------|------|---------|
| `plans` | Low (1-20) | User-created blocking plans |
| `schedules` | Low | Day/time activation windows per plan |
| `app_rules` | Medium (10-100 per plan) | Process blocking rules |
| `url_rules` | Medium (10-500 per plan) | Domain/URL blocking rules |
| `quotas` | Low | Per-app/site time limits |
| `quota_ledger` | Medium (grows daily) | Daily usage accumulation |
| `sessions` | Medium | Plan activation history |
| `events` | High (grows continuously) | Block/quota/system event log |
| `forced_mode_state` | Low (0-5) | Active forced mode sessions |
| `settings` | Low (5-20) | Key-value daemon configuration |

---

## 5. Platform Enforcement Matrix

| Enforcement Method | Platform | Privilege Required | What It Blocks | Bypass Resistance |
|-------------------|----------|-------------------|----------------|-------------------|
| **WFP Callout** | Windows | Admin (service) | DNS queries, DoH IPs, HTTPS by IP | HIGH — kernel-level network filter |
| **HOSTS File** | Windows/Linux | Admin/root | DNS resolution (domain → 0.0.0.0) | LOW — user can edit /etc/hosts |
| **WFP + HOSTS** | Windows | Admin | DNS + DoH + direct IP | HIGH — belt-and-suspenders (D-003) |
| **ESF** | macOS | System Extension entitlement | Process execution (AUTH_EXEC) | VERY HIGH — kernel event stream |
| **NEDNSProxyProvider** | macOS | Network Extension entitlement | DNS queries (NXDOMAIN synthesis) | HIGH — system DNS proxy |
| **eBPF LSM** | Linux 5.7+ | root + CONFIG_BPF_LSM | Process execution (bprm_check) | VERY HIGH — in-kernel hook |
| **Fanotify** | Linux 3.8+ | CAP_SYS_ADMIN | Process execution (OPEN_EXEC_PERM) | HIGH — kernel permission check |
| **Unbound RPZ** | Linux | root | DNS queries (NXDOMAIN via RPZ) | MEDIUM — requires Unbound |
| **AccessibilityService** | Android | User grant | Foreground app detection + overlay | MEDIUM — user can disable in settings |
| **VpnService** | Android | User grant | DNS queries (NXDOMAIN synthesis) | MEDIUM — user can disconnect VPN |
| **declarativeNetRequest** | Chrome | Extension permission | URL requests (redirect to blocked) | LOW — user can uninstall extension |
| **webRequest** | Firefox | Extension permission | URL requests (cancel + redirect) | LOW — user can uninstall extension |

---

## 6. Forced Mode Design

Forced Mode prevents the user from disabling a blocking plan until a timer expires. It is the core anti-circumvention feature.

### 6.1 Dual-Clock Architecture

```
┌─────────────────────────┐     ┌──────────────────────────┐
│    MONOTONIC CLOCK       │     │    WALL CLOCK             │
│  (Instant::now)          │     │  (Utc::now)               │
│                          │     │                            │
│  Used for: correctness   │     │  Used for: display only    │
│  - Immune to system      │     │  - Shown to user as        │
│    clock changes          │     │    "expires at HH:MM"      │
│  - Immune to timezone    │     │  - Stored in DB for        │
│    changes                │     │    recovery after reboot   │
│  - Survives NTP sync     │     │                            │
│                          │     │                            │
│  remaining_s =           │     │  expires_at_utc =          │
│    duration - elapsed    │     │    started_at + duration   │
└─────────────────────────┘     └──────────────────────────┘
```

### 6.2 Emergency Unlock

1. User requests emergency unlock during active Forced Mode
2. System generates random 8-character challenge string
3. User must compute 8-digit response code (paper-based, no software assist)
4. Argon2id verifies the emergency code hash stored at session start
5. If correct → Forced Mode ends immediately, DB state cleared
6. If incorrect → attempt logged, max 5 attempts before lockout (300s)

### 6.3 Reboot Persistence

```
Daemon starts → check forced_mode_state table
    │
    ├─ active=1, expires_at > now → restore session
    │   monotonic_start = current monotonic time
    │   remaining = expires_at_utc - utc_now
    │
    └─ active=1, expires_at <= now → clear state
        session expired during downtime
```

---

## 7. Security Model

Full threat analysis: [security_review.md](security_review.md)

### 7.1 Threat Actors

| Actor | Capability | Primary Mitigation |
|-------|-----------|-------------------|
| **Casual user** | Browser settings, app removal | Extension + daemon dual enforcement |
| **Technical user** | Process manager, DNS config, VPN | WFP/eBPF kernel hooks, HOSTS tamper detection |
| **Admin user** | Service control, system config | Forced Mode, plan protection (Argon2id) |
| **Sophisticated** | Kernel modules, Secure Boot bypass | eBPF LSM, ESF system extension, bypass logging |

### 7.2 Key Mitigations

- **HOSTS tamper detection** (D-010): SHA-256 hash comparison every 2 seconds, auto-restore on change
- **DoH bypass prevention** (S-001): WFP blocks 14 known DoH provider IPs
- **Database encryption** (D-006): SQLCipher with machine-ID-derived key
- **Password protection**: Argon2id with per-plan hash storage
- **Forced Mode dual-clock**: Immune to system clock manipulation
- **IPC channel permissions**: Unix socket 0660, Named Pipe DACL

---

## 8. Extension ↔ Daemon Protocol

### 8.1 Communication Path

```
Browser Extension                NMH Binary              Daemon
     │                              │                      │
     │── chrome.runtime.           │                      │
     │   connectNative()           │                      │
     │   ("com.focusme.nmh")       │                      │
     │                              │                      │
     │◀─── stdin/stdout ──────────▶│                      │
     │     4-byte LE + MsgPack     │                      │
     │                              │── Named Pipe / UDS ─▶│
     │                              │   4-byte LE + MsgPack│
     │                              │◀─ response ──────────│
     │◀─── response ───────────────│                      │
```

### 8.2 Reconnection Strategy

```
connectNative() fails
    │
    ▼
Exponential backoff: 2s → 4s → 8s → 16s → 32s (5 retries)
    │
    ▼
If all retries fail:
    Extension falls back to local storage rules
    30-second chrome.alarms sync continues
    Next alarm triggers reconnect attempt
```

### 8.3 Rule Sync Protocol

1. Extension wakes on chrome.alarms (30s interval)
2. Sends `URL_CHECK` or `PLAN_LIST` via NMH
3. Daemon queries scheduler for active plans
4. Daemon returns active URL rules
5. Extension converts to DNR rules (MV3) or webRequest patterns (MV2)
6. `MAX_DNR = 5000` rules (Chrome limit)
7. Allow rules get priority=2 (higher than block priority=1)

---

## 9. Android Architecture

### 9.1 Four-Service Coordination

```
┌───────────────────────────────────────────────────┐
│                  FocusMeDaemonService               │
│  - Foreground service (persistent notification)     │
│  - 30-second Handler loop for plan evaluation       │
│  - SharedPreferences JSON plan storage (D-012)      │
│  - Coordinates VPN + Accessibility services         │
├────────────────┬──────────────────┬─────────────────┤
│ VpnService     │ Accessibility    │ WorkManager     │
│                │   Service        │                 │
│ - Tun device   │ - onAccessibilityEvent()          │
│ - DNS intercept│ - foreground app │ - Periodic      │
│ - NXDOMAIN     │   detection      │   quota reset   │
│   synthesis    │ - overlay_blocked│ - Backup /      │
│ - IP checksum  │   XML inflation  │   restore       │
│   recalculation│ - ConstraintLayout                 │
└────────────────┴──────────────────┴─────────────────┘
```

### 9.2 Inter-Service Communication

- **DaemonService → VpnService**: `Intent` extras with blocked domain list
- **DaemonService → AccessibilityService**: `SharedPreferences` JSON — blocked app package names
- **QuotaTracker**: `UsageStatsManager.queryUsageStats(INTERVAL_DAILY)` — reconciles every 30s cycle
- **Plan storage**: `SharedPreferences` as JSON strings (D-012 — no SQLite on Android)

### 9.3 Permission Requirements

| Permission | API | Justification |
|------------|-----|--------------|
| `BIND_ACCESSIBILITY_SERVICE` | AccessibilityService | Core functionality: foreground app detection and blocking |
| `BIND_VPN_SERVICE` | VpnService | DNS query interception for domain blocking |
| `FOREGROUND_SERVICE` | DaemonService | Persistent monitoring service |
| `PACKAGE_USAGE_STATS` | UsageStatsManager | App usage time tracking for quotas |
| `SYSTEM_ALERT_WINDOW` | WindowManager | Blocking overlay display |
| `RECEIVE_BOOT_COMPLETED` | BroadcastReceiver | Auto-start after device reboot |
| `INTERNET` | VpnService | Required for VPN tunnel establishment |

---

## 10. Known Limitations

| Platform | Limitation | Impact | Mitigation |
|----------|-----------|--------|-----------|
| **All** | Admin/root can disable the service | Full bypass | Forced Mode + detection logging |
| **Windows** | Safe Mode bypasses WFP + service | Full bypass | Not mitigatable without kernel driver |
| **macOS** | Recovery Mode can remove System Extension | Full bypass | Detection on next boot |
| **macOS** | ESF requires Apple entitlement | No exec blocking | DNS-only enforcement until approved |
| **Linux** | Safe Mode / init=/bin/sh | Full bypass | Not mitigatable |
| **Linux** | eBPF requires CONFIG_BPF_LSM=y | Falls back to Fanotify | Fanotify provides equivalent protection |
| **Android** | Safe Mode disables third-party services | Full bypass | Detection on next normal boot |
| **Android** | User can revoke Accessibility permission | App blocking disabled | Persistent notification warning |
| **Browser** | User can uninstall extension | URL blocking weaker | Daemon HOSTS/WFP still enforces |
| **Browser** | MV3 static DNR rules (max 5000) | Large rule sets truncated | Priority sorting keeps most important |
| **All** | VPN/proxy can tunnel past DNS blocking | DNS bypass | WFP blocks known VPN ports (Windows) |
| **All** | Tor Browser uses its own DNS | DNS bypass | App blocking catches Tor process |

---

## 11. Cloud Backend Architecture (Phase 5)

### 11.1 Overview

The cloud backend is an Axum 0.7 REST API running on PostgreSQL 16 (D-014). It provides user authentication, cross-device plan synchronization, and a family dashboard.

**Request Lifecycle:**

```
Client Request
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│ Tower Middleware Stack (applied bottom-up)                    │
│                                                              │
│  ┌─ Extension Layer ── inject RateLimiter into extensions    │
│  │                                                           │
│  ├─ Rate Limiter ── token bucket per IP (D-015)              │
│  │   Auth: 10 req/min  │  API: 100 req/min                  │
│  │   Exceeded → 429 + Retry-After header                    │
│  │                                                           │
│  ├─ Security Headers ── X-Frame-Options, CSP, etc.           │
│  │                                                           │
│  ├─ CORS ── allow origins (restrict in prod via FRONTEND_URL)│
│  │                                                           │
│  └─ TraceLayer ── structured logging of all requests          │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ Router                                                       │
│  GET  /health ─────────── health_handler() [public]          │
│  /api/v1/auth/* ──────── auth routes [public]                │
│  /api/v1/* ───────────── protected routes [Bearer JWT]       │
│    ├─ /plans/*            plan CRUD + delta sync             │
│    ├─ /sync/*             push/pull batch sync               │
│    └─ /family/*           family groups + dashboard          │
├──────────────────────────────────────────────────────────────┤
│ Handler → DB Query → JSON Response                           │
└─────────────────────────────────────────────────────────────┘
```

### 11.2 Authentication Flow

The backend uses **Argon2id** (OWASP-recommended parameters: m=64MiB, t=3, p=4) for password hashing and **JWT** for session management.

**Token Architecture:**
- **Access token** (15 min TTL): HS256-signed JWT carrying `{sub, email, token_type: "access", iat, exp}`. Validated by `auth_middleware` on every protected request.
- **Refresh token** (30 day TTL): HS256-signed JWT. The raw token is sent to the client; a **SHA-256 hash** is stored server-side in `refresh_tokens` table.

**Refresh Rotation with Reuse Detection:**

```
Client sends refresh_token
    │
    ▼
Backend: SHA-256(refresh_token) → lookup in DB
    │
    ├── Found + not revoked:
    │     1. Revoke current refresh token (mark revoked_at)
    │     2. Issue new access + refresh token pair
    │     3. Store SHA-256(new_refresh) in DB
    │     4. Return new pair to client
    │
    ├── Found + already revoked (REUSE DETECTED):
    │     ⚠️ Token theft suspected!
    │     1. Revoke ALL refresh tokens for this user
    │     2. Return 401 — forces re-login on all devices
    │
    └── Not found:
          Return 401 — invalid token
```

### 11.3 Sync Protocol

FocusMe uses a **push-first, pull-second delta sync** model with optimistic concurrency control.

```
┌────────────┐                         ┌─────────────┐
│   Client   │                         │   Server    │
└─────┬──────┘                         └──────┬──────┘
      │  POST /sync/push                      │
      │  {device_id, events: [...]}           │
      │──────────────────────────────────────►│
      │                                       │── Store events
      │                                       │── Update plan versions
      │  {processed: N, server_time}          │
      │◄──────────────────────────────────────│
      │                                       │
      │  GET /sync/pull?since=last_sync_ts    │
      │──────────────────────────────────────►│
      │                                       │── Query plans WHERE
      │                                       │   updated_at > since
      │  {plans: [...], events: [...],        │── Query events WHERE
      │   server_time: "..."}                 │   created_at > since
      │◄──────────────────────────────────────│
      │                                       │
      │  POST /plans {local_id, plan_json,    │
      │   expected_version: 3}                │
      │──────────────────────────────────────►│
      │                                       │── Check: server version == 3?
      │                                       │
      │  ┌─ Yes: upsert, version=4, 201      │
      │  └─ No:  409 Conflict + server data   │
      │◄──────────────────────────────────────│
```

**Conflict Resolution:** Last-write-wins using `updated_at` timestamp as tiebreaker. The `expected_version` field provides optimistic locking — if the server's version is ahead, the client receives the server's data and must merge locally.

### 11.4 Family Model

```
┌──────────────────┐         ┌──────────────────────┐
│  family_groups    │    1:N  │   family_members     │
│──────────────────│─────────│──────────────────────│
│ id (UUID PK)     │         │ id (UUID PK)         │
│ name             │         │ group_id (FK)        │
│ owner_id (FK)    │         │ user_id (FK)         │
│ created_at       │         │ role (owner/member)  │
│                  │         │ status (pending/     │
│                  │         │         accepted)    │
│                  │         │ invite_token         │
│                  │         │ invite_expires_at    │
└──────────────────┘         └──────────────────────┘
                                       │
                                  1:N  │
                             ┌─────────┴──────────┐
                             │   shared_plans     │
                             │────────────────────│
                             │ id (UUID PK)       │
                             │ plan_id (FK)       │
                             │ shared_by (FK)     │
                             │ shared_with_group  │
                             │   (FK)             │
                             │ created_at         │
                             └────────────────────┘
```

- **Owner-only invites:** Only the group owner can invite members.
- **UUID invite tokens:** 7-day expiry, single-use.
- **Shared plans are copies:** To avoid cross-user conflicts, shared plans create independent copies rather than references.

### 11.5 Database Schema (ER Diagram)

```
┌──────────────┐     1:N     ┌──────────────────┐
│    users     │────────────▶│ refresh_tokens   │
│──────────────│             │──────────────────│
│ id UUID PK   │             │ id UUID PK       │
│ email UNIQUE │             │ user_id FK       │
│ password_hash│             │ token_hash       │
│ display_name │             │ expires_at       │
│ created_at   │             │ revoked_at       │
│ updated_at   │             └──────────────────┘
└──────┬───────┘
       │
       │ 1:N      ┌──────────────────┐
       ├─────────▶│    devices       │
       │          │──────────────────│
       │          │ id UUID PK       │
       │          │ user_id FK       │
       │          │ device_name      │
       │          │ platform         │
       │          │ push_token       │
       │          │ last_seen_at     │
       │          └──────────────────┘
       │
       │ 1:N      ┌──────────────────┐
       ├─────────▶│  cloud_plans     │
       │          │──────────────────│
       │          │ id UUID PK       │
       │          │ user_id FK       │
       │          │ local_id         │
       │          │ plan_json JSONB  │
       │          │ version INT      │
       │          │ deleted_at       │
       │          │ created_at       │
       │          │ updated_at       │
       │          └──────────────────┘
       │
       │ 1:N      ┌──────────────────┐
       └─────────▶│  sync_events     │
                  │──────────────────│
                  │ id UUID PK       │
                  │ user_id FK       │
                  │ device_id        │
                  │ event_type       │
                  │ payload JSONB    │
                  │ created_at       │
                  └──────────────────┘
```

**8 tables total.** PostgreSQL extensions: `pgcrypto` (gen_random_uuid()), `JSONB` for flexible plan storage.

### 11.6 Rate Limiting (D-015)

In-process **token bucket** algorithm per client IP:

| Route Category | Limit | Bucket Capacity |
|---------------|-------|-----------------|
| Auth routes (`/api/v1/auth/*`) | 10 req/min | 10 tokens |
| All other API routes | 100 req/min | 100 tokens |

- **Implementation:** `tokio::sync::Mutex<HashMap<(IpAddr, Category), Bucket>>` — lock-per-check, O(1) lookup.
- **Token refill:** Continuous (fractional tokens added based on elapsed time since last refill).
- **Cleanup:** Background `tokio::spawn` task removes buckets unused for >5 minutes every 60 seconds.
- **IP extraction priority:** `X-Real-IP` header → `X-Forwarded-For` first entry → socket peer address.
- **Exceeded → 429 Too Many Requests** with `Retry-After` header (seconds).

*Chosen over `tower-governor` for zero external dependency and full control over burst behavior. Revisit with Redis if horizontal scaling required.*

### 11.7 Deployment

**Development (local):**
```bash
cd backend
docker compose up -d postgres    # PostgreSQL 16 only
cp .env.example .env             # Configure
cargo run                        # Direct binary
```

**Production (containerized):**
```bash
docker compose up -d             # PostgreSQL + backend
```

**Required Environment Variables:**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `JWT_SECRET` | Yes | — | HS256 signing key (≥32 chars) |
| `PORT` | No | 8080 | HTTP listen port |
| `RUST_LOG` | No | `info` | Tracing filter |
| `POSTHOG_API_KEY` | No | — | PostHog project key (empty = disabled) |
| `FRONTEND_URL` | No | `*` | CORS allowed origin (restrict in prod) |

**PostgreSQL connection pool:** sqlx default (10 connections). Tune via `PgPoolOptions::max_connections()` based on expected load. Rule of thumb: connections = 2× CPU cores.

**TLS termination:** The backend does NOT handle TLS. Deploy behind nginx, Caddy, or a cloud load balancer for HTTPS.

---

*Architecture documented as of Session 8. 42/49 tasks complete (86%). 125+ files.*
*See [decisions.md](decisions.md) for D-001 through D-015. See [security_review.md](security_review.md) for full threat model.*
