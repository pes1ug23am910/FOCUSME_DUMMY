# FocusMe — Decision Log

All architecture and implementation decisions are logged here for audit and reversal reference.

---

[DECISION D-001] Context: Project repository structure
  Options considered:
    A) Polyrepo — separate repos for daemon, extension, android, UI
    B) Monorepo — single repo with top-level module directories
  Chosen: B — Monorepo
  Rationale: Build plan T-004 recommends monorepo. Simplifies CI/CD, cross-module refactoring, and version coordination across 5 layers.
  Reversibility: moderate — would require splitting history and updating CI

[DECISION D-002] Context: IPC serialization format
  Options considered:
    A) JSON only — human-readable, larger payloads
    B) MessagePack only — compact, typed, less debuggable
    C) MessagePack primary + JSON debug mode
  Chosen: C — MessagePack primary with JSON fallback for debugging
  Rationale: Build plan Section 2.3 explicitly describes this dual approach.
  Reversibility: easy — format is a config flag on the IPC layer

[DECISION D-003] Context: URL blocking mechanism for Windows MVP
  Options considered:
    A) HOSTS file only — simple, no driver needed, trivially bypassed
    B) WFP callout only — more robust, more complex, requires admin
    C) HOSTS + WFP (belt-and-suspenders)
  Chosen: C — HOSTS file as primary (fast, readable) + WFP as fallback/reinforcement
  Rationale: Build plan Section 2.2 explicitly describes this dual approach (T-011/T-012).
  Reversibility: easy — WFP layer can be disabled independently via config flag

[DECISION D-004] Context: Linux process blocking mechanism
  Options considered:
    A) eBPF LSM hooks (bpf_lsm_bprm_check_security) — modern, clean, kernel 5.7+
    B) Fanotify (FAN_OPEN_EXEC_PERM) — wider kernel support, CAP_SYS_ADMIN needed
    C) eBPF primary + Fanotify fallback
  Chosen: C — eBPF LSM primary with Fanotify fallback
  Rationale: Build plan Section 2.2 strongly prefers eBPF CO-RE. Fanotify fallback for kernels without CONFIG_BPF_LSM=y (per T-001/T-016).
  Reversibility: easy — runtime detection selects mechanism

[DECISION D-005] Context: UI framework selection
  Options considered:
    A) Tauri (Rust + WebView) — lightweight, native feel, smaller binary
    B) Electron — proven, larger ecosystem, ~100MB overhead
  Chosen: A — Tauri
  Rationale: Build plan Section 2.1 and assumption A-06 specify Tauri as primary choice. Electron is documented fallback.
  Reversibility: moderate — would require rewriting Rust backend layer

[DECISION D-006] Context: Database encryption approach
  Options considered:
    A) SQLite without encryption — simple but data exposed
    B) SQLCipher — transparent encryption, Apache 2.0 license
  Chosen: B — SQLCipher with key derived from machine ID + user salt
  Rationale: Build plan Section 2.4 specifies this exact approach.
  Reversibility: moderate — would require data migration

[DECISION D-007] Context: Android DNS blocking approach
  Options considered:
    A) Root-based iptables — powerful but requires root
    B) Local VPN service (VpnService) — no root, intercepts DNS
  Chosen: B — Local VPN service
  Rationale: Build plan Section 2.2 specifies this approach. Same method used by AdGuard/Blokada.
  Reversibility: easy — VPN layer is self-contained

[DECISION D-008] Context: Browser extension manifest strategy
  Options considered:
    A) MV3 only — modern, limited Firefox support
    B) MV2 only — broader support, deprecated on Chrome
    C) Dual MV3 + MV2 builds
  Chosen: C — Dual builds with build flags
  Rationale: Build plan Section 2.5 specifies MV3 for Chrome/Edge and MV2 for Firefox.
  Reversibility: easy — build flag toggles target

[DECISION D-009] Context: Daemon shared state pattern
  Options considered:
    A) Global mutable statics — simple but unsafe, difficult to test
    B) Dependency injection via function parameters — explicit but verbose
    C) Arc<T> subsystem sharing via DaemonState struct
  Chosen: C — Arc<T> with DaemonState struct holding all subsystem handles
  Rationale: Each subsystem (scheduler, process_monitor, ipc_server, etc.) needs shared
    access to the DB, config, and other subsystems. Arc<T> provides thread-safe shared
    ownership without global state. DaemonState is constructed once in main() and cloned
    into each spawned task. Matches Tokio ecosystem conventions.
  Session: 2
  Reversibility: easy — pattern is localized to main.rs wiring

[DECISION D-010] Context: HOSTS file tamper detection mechanism
  Options considered:
    A) inotify (Linux) / ReadDirectoryChangesW (Windows) — event-driven, immediate
    B) Poll-based SHA-256 hash comparison every N seconds
  Chosen: B — Poll-based SHA-256 hash comparison (2-second interval)
  Rationale: Event-driven watchers (inotify, RDCW) are platform-specific and can miss
    changes if the file is replaced atomically (rename-over-write). Polling with SHA-256
    hash comparison is cross-platform, simple, and catches all modification types
    including atomic replacement. 2-second interval is fast enough for tamper detection
    without excessive I/O.
  Session: 2
  Reversibility: easy — swap poll loop for platform-specific watcher

[DECISION D-011] Context: Tauri version selection
  Options considered:
    A) Tauri v1 — stable, proven, larger community
    B) Tauri v2 — newer, plugin system, multi-window, mobile support roadmap
  Chosen: B — Tauri v2
  Rationale: Tauri v2 provides a better plugin architecture, improved multi-window
    support, and a path toward mobile targets (Android/iOS). The project is greenfield
    so there's no migration cost. v2 API is stable enough for production use.
  Session: 2
  Build plan ref: D-005 (Tauri chosen over Electron)
  Reversibility: moderate — API differences require refactoring lib.rs + frontend invoke calls

[DECISION D-012] Context: Android plan storage mechanism
  Options considered:
    A) Room database (SQLite ORM) — structured, type-safe, migrations
    B) SharedPreferences + JSON serialization — simple, no schema, fast reads
  Chosen: B — SharedPreferences with JSON-serialized plan objects
  Rationale: Android plans are small (typically 1-20 plans), read-heavy, and don't
    require relational queries. SharedPreferences provides instant key-value access
    without Room's boilerplate. JSON serialization via Gson/kotlinx.serialization
    keeps the data format consistent with IPC payloads. Room would be over-engineering
    for this use case.
  Session: 3
  Reversibility: easy — migrate JSON to Room entities if complexity grows

[DECISION D-013] Context: Database connection locking strategy (S-005)
  Options considered:
    A) std::sync::Mutex — exclusive lock for all operations, simpler
    B) std::sync::RwLock — read/write separation, but can't be used in async context easily
    C) tokio::sync::RwLock — async-compatible read/write separation
  Chosen: C — tokio::sync::RwLock<Connection>
  Rationale: The plan store is read-heavy (URL/app checks every poll cycle from
    process_monitor, scheduler, and IPC handlers) but writes are infrequent (plan CRUD,
    event logging). RwLock allows concurrent read-lock holders, reducing contention.
    SQLite WAL mode already supports concurrent reads at the database level.
    11 read methods use blocking_read(), 13 write methods use blocking_write().
  Session: 6
  Resolves: S-005
  Reversibility: easy — revert to Mutex wrapper with same API

[DECISION D-014] Context: Cloud backend database engine
  Options considered:
    A) SQLite — local-friendly, simpler ops, limited concurrent writes
    B) PostgreSQL — JSONB, robust indexing, TIMESTAMPTZ, pgcrypto, proven at scale
    C) MySQL / MariaDB — widely deployed, less JSONB ergonomics
  Chosen: B — PostgreSQL 16
  Rationale: Phase 5 cloud backend serves multiple concurrent users across time zones.
    PostgreSQL provides native JSONB for flexible plan storage, pgcrypto for UUID
    generation, TIMESTAMPTZ for correct global time handling, excellent indexing
    (GIN for JSONB, B-tree for FKs), and ON CONFLICT upsert semantics for sync.
    Aligns with build plan Section 4 cloud sync requirements.
  Session: 7
  Build plan ref: T-060–T-064
  Reversibility: moderate — sqlx abstracts most SQL; schema migration needed

[DECISION D-015] Context: API rate limiting strategy
  Options considered:
    A) tower-governor — community crate, wraps governor (sliding window)
    B) Redis-backed distributed rate limiter — shared state across instances
    C) In-process token bucket — HashMap<(IP, Category), Bucket>, Mutex-guarded
  Chosen: C — In-process token bucket with per-IP, per-category tracking
  Rationale: The cloud backend currently runs as a single instance. An in-process
    token bucket provides zero external dependencies (no Redis), full control over
    burst behavior (continuous token refill vs sliding window), and sub-microsecond
    check latency. Auth routes limited to 10 req/min per IP; other routes to 100
    req/min. A background cleanup task removes idle buckets every 60s to prevent
    memory leaks. Revisit with Redis if horizontal scaling requires shared state.
  Session: 8
  Build plan ref: T-060 (backend hardening)
  Reversibility: easy — swap RateLimiter implementation behind same middleware interface
