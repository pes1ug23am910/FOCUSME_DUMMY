-- ============================================================
-- FILE:        db_schema.sql
-- MODULE:      Layer 2 — Policy Store
-- TASK:        T-018
-- PLATFORM:    cross
-- AUTHOR:      FocusMe Co-Pilot (Claude Opus)
-- GENERATED:   Phase 1, SQLite policy store
-- DEPENDENCIES: SQLite 3.35+, SQLCipher, refinery (migrations)
-- TEST COVERAGE: UT-07 (concurrent write safety)
-- KNOWN LIMITATIONS: SQLCipher key derived from machine ID + user salt;
--                    does not protect against admin-level disk access
-- ============================================================

-- Enable WAL mode for concurrent read/write performance
-- Applied pragmatically at connection open time, not in schema
-- PRAGMA journal_mode=WAL;
-- PRAGMA foreign_keys=ON;

-- ============================================================
-- PLANS TABLE
-- Stores Focus Plan metadata and configuration
-- ============================================================
CREATE TABLE IF NOT EXISTS plans (
    plan_id         TEXT PRIMARY KEY NOT NULL,   -- UUIDv4
    name            TEXT NOT NULL,
    schema_version  TEXT NOT NULL DEFAULT '1.0.0',
    enabled         INTEGER NOT NULL DEFAULT 1,  -- boolean: 0/1
    forced_mode     INTEGER NOT NULL DEFAULT 0,
    forced_mode_max_duration_s INTEGER NOT NULL DEFAULT 86400,
    protection_type TEXT NOT NULL DEFAULT 'none', -- 'none' | 'argon2id_password' | 'random_challenge'
    protection_hash TEXT,                         -- Argon2id hash (NULL if type=none)
    challenge_required INTEGER NOT NULL DEFAULT 0,
    plan_json       TEXT NOT NULL,                -- Full policy JSON (source of truth)
    created_at      TEXT NOT NULL,                -- ISO-8601 UTC
    modified_at     TEXT NOT NULL                 -- ISO-8601 UTC
);

CREATE INDEX IF NOT EXISTS idx_plans_enabled ON plans(enabled);

-- ============================================================
-- SCHEDULES TABLE
-- Time windows for plan activation
-- ============================================================
CREATE TABLE IF NOT EXISTS schedules (
    schedule_id     TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    days            TEXT NOT NULL,                -- JSON array: ["mon","tue",...]
    start_time      TEXT NOT NULL,                -- HH:MM (24h)
    end_time        TEXT NOT NULL,                -- HH:MM (24h)
    timezone        TEXT NOT NULL                 -- IANA timezone
);

CREATE INDEX IF NOT EXISTS idx_schedules_plan ON schedules(plan_id);

-- ============================================================
-- APP RULES TABLE
-- Application block/allow rules
-- ============================================================
CREATE TABLE IF NOT EXISTS app_rules (
    rule_id         TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    rule_type       TEXT NOT NULL,                -- 'block' | 'allow'
    match_type      TEXT NOT NULL,                -- 'process_name' | 'path_prefix' | 'path_exact' | 'bundle_id'
    value           TEXT NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_app_rules_plan ON app_rules(plan_id);

-- ============================================================
-- URL RULES TABLE
-- Domain/URL block/allow rules
-- ============================================================
CREATE TABLE IF NOT EXISTS url_rules (
    rule_id         TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    rule_type       TEXT NOT NULL,                -- 'block' | 'allow'
    match_type      TEXT NOT NULL,                -- 'domain' | 'wildcard' | 'path' | 'regex'
    value           TEXT NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_url_rules_plan ON url_rules(plan_id);

-- ============================================================
-- QUOTAS TABLE
-- Daily/weekly usage quotas per target
-- ============================================================
CREATE TABLE IF NOT EXISTS quotas (
    quota_id        TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    target_type     TEXT NOT NULL,                -- 'domain' | 'app' | 'category'
    target          TEXT NOT NULL,
    daily_limit_s   INTEGER NOT NULL,
    launches_per_day INTEGER,                     -- NULL = unlimited
    min_break_s     INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_quotas_plan ON quotas(plan_id);

-- ============================================================
-- QUOTA LEDGER TABLE
-- Tracks daily quota consumption (UT-07: concurrent write safety)
-- ============================================================
CREATE TABLE IF NOT EXISTS quota_ledger (
    ledger_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    quota_id        TEXT NOT NULL REFERENCES quotas(quota_id) ON DELETE CASCADE,
    date            TEXT NOT NULL,                -- YYYY-MM-DD (local date)
    used_s          INTEGER NOT NULL DEFAULT 0,
    launch_count    INTEGER NOT NULL DEFAULT 0,
    last_launch_at  TEXT,                         -- ISO-8601 UTC
    updated_at      TEXT NOT NULL                 -- ISO-8601 UTC
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_quota_ledger_daily ON quota_ledger(quota_id, date);

-- ============================================================
-- SESSIONS TABLE
-- Tracks plan activation sessions (start/stop events)
-- ============================================================
CREATE TABLE IF NOT EXISTS sessions (
    session_id      TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    started_at      TEXT NOT NULL,                -- ISO-8601 UTC
    ended_at        TEXT,                         -- ISO-8601 UTC (NULL if active)
    forced_mode     INTEGER NOT NULL DEFAULT 0,
    end_reason      TEXT                          -- 'scheduled' | 'manual' | 'forced_expired' | 'emergency_unlock'
);

CREATE INDEX IF NOT EXISTS idx_sessions_plan ON sessions(plan_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(ended_at) WHERE ended_at IS NULL;

-- ============================================================
-- EVENTS TABLE
-- Block events, quota events, system events for stats display
-- Partitioned by date for efficient cleanup per retention policy
-- ============================================================
CREATE TABLE IF NOT EXISTS events (
    event_id        TEXT PRIMARY KEY NOT NULL,    -- UUIDv4
    event_type      TEXT NOT NULL,                -- See Appendix A event_type enum
    plan_id         TEXT REFERENCES plans(plan_id) ON DELETE SET NULL,
    rule_id         TEXT,
    session_id      TEXT REFERENCES sessions(session_id) ON DELETE SET NULL,
    subject_hash    TEXT,                         -- SHA-256 of blocked process/domain
    -- PRIVACY: Only hashed identifiers stored. No plaintext URLs or app names.
    duration_ms     INTEGER,
    quota_used_s    INTEGER,
    quota_limit_s   INTEGER,
    timestamp_utc   TEXT NOT NULL,                -- ISO-8601 UTC
    synced          INTEGER NOT NULL DEFAULT 0    -- 0 = not synced to cloud, 1 = synced
);

CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_plan ON events(plan_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_utc);
CREATE INDEX IF NOT EXISTS idx_events_unsynced ON events(synced) WHERE synced = 0;

-- ============================================================
-- FORCED MODE STATE TABLE
-- Persists Forced Mode timer state across restarts
-- Survives reboots; dualtracks wall clock + monotonic offset
-- ============================================================
CREATE TABLE IF NOT EXISTS forced_mode_state (
    plan_id         TEXT PRIMARY KEY NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    started_at_utc  TEXT NOT NULL,                -- ISO-8601 UTC wall clock
    expires_at_utc  TEXT NOT NULL,                -- ISO-8601 UTC wall clock
    monotonic_start INTEGER NOT NULL,             -- Monotonic clock ticks at start
    monotonic_duration_s INTEGER NOT NULL,         -- Duration in seconds
    emergency_code_hash TEXT,                      -- Argon2id hash of TOTP emergency code
    active          INTEGER NOT NULL DEFAULT 1
);

-- ============================================================
-- SETTINGS TABLE
-- Key-value store for daemon configuration
-- ============================================================
CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY NOT NULL,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL                 -- ISO-8601 UTC
);

-- Default settings
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('schema_version', '1.0.0', datetime('now')),
    ('telemetry_consent', 'essential_only', datetime('now')),
    ('log_level', 'INFO', datetime('now')),
    ('debug_ipc_json', '0', datetime('now')),
    ('forced_mode_max_cap_s', '86400', datetime('now')),
    ('process_poll_interval_ms', '500', datetime('now')),
    ('hosts_restore_timeout_ms', '2000', datetime('now'));

-- ============================================================
-- MIGRATIONS TRACKING TABLE
-- Used by refinery crate to track applied migrations
-- ============================================================
CREATE TABLE IF NOT EXISTS refinery_schema_history (
    version         INTEGER PRIMARY KEY,
    name            TEXT,
    applied_on      TEXT NOT NULL,
    checksum        TEXT NOT NULL
);
