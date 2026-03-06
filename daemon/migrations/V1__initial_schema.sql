-- FocusMe Initial Schema Migration V1
-- Applied by refinery crate at startup

CREATE TABLE IF NOT EXISTS plans (
    plan_id         TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    schema_version  TEXT NOT NULL DEFAULT '1.0.0',
    enabled         INTEGER NOT NULL DEFAULT 1,
    forced_mode     INTEGER NOT NULL DEFAULT 0,
    forced_mode_max_duration_s INTEGER NOT NULL DEFAULT 86400,
    protection_type TEXT NOT NULL DEFAULT 'none',
    protection_hash TEXT,
    challenge_required INTEGER NOT NULL DEFAULT 0,
    plan_json       TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    modified_at     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_plans_enabled ON plans(enabled);

CREATE TABLE IF NOT EXISTS schedules (
    schedule_id     TEXT PRIMARY KEY NOT NULL,
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    days            TEXT NOT NULL,
    start_time      TEXT NOT NULL,
    end_time        TEXT NOT NULL,
    timezone        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_schedules_plan ON schedules(plan_id);

CREATE TABLE IF NOT EXISTS app_rules (
    rule_id         TEXT PRIMARY KEY NOT NULL,
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    rule_type       TEXT NOT NULL,
    match_type      TEXT NOT NULL,
    value           TEXT NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_app_rules_plan ON app_rules(plan_id);

CREATE TABLE IF NOT EXISTS url_rules (
    rule_id         TEXT PRIMARY KEY NOT NULL,
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    rule_type       TEXT NOT NULL,
    match_type      TEXT NOT NULL,
    value           TEXT NOT NULL,
    sort_order      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_url_rules_plan ON url_rules(plan_id);

CREATE TABLE IF NOT EXISTS quotas (
    quota_id        TEXT PRIMARY KEY NOT NULL,
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    target_type     TEXT NOT NULL,
    target          TEXT NOT NULL,
    daily_limit_s   INTEGER NOT NULL,
    launches_per_day INTEGER,
    min_break_s     INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_quotas_plan ON quotas(plan_id);

CREATE TABLE IF NOT EXISTS quota_ledger (
    ledger_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    quota_id        TEXT NOT NULL REFERENCES quotas(quota_id) ON DELETE CASCADE,
    date            TEXT NOT NULL,
    used_s          INTEGER NOT NULL DEFAULT 0,
    launch_count    INTEGER NOT NULL DEFAULT 0,
    last_launch_at  TEXT,
    updated_at      TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_quota_ledger_daily ON quota_ledger(quota_id, date);

CREATE TABLE IF NOT EXISTS sessions (
    session_id      TEXT PRIMARY KEY NOT NULL,
    plan_id         TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    forced_mode     INTEGER NOT NULL DEFAULT 0,
    end_reason      TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_plan ON sessions(plan_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(ended_at) WHERE ended_at IS NULL;

CREATE TABLE IF NOT EXISTS events (
    event_id        TEXT PRIMARY KEY NOT NULL,
    event_type      TEXT NOT NULL,
    plan_id         TEXT REFERENCES plans(plan_id) ON DELETE SET NULL,
    rule_id         TEXT,
    session_id      TEXT REFERENCES sessions(session_id) ON DELETE SET NULL,
    subject_hash    TEXT,
    duration_ms     INTEGER,
    quota_used_s    INTEGER,
    quota_limit_s   INTEGER,
    timestamp_utc   TEXT NOT NULL,
    synced          INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_plan ON events(plan_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_utc);
CREATE INDEX IF NOT EXISTS idx_events_unsynced ON events(synced) WHERE synced = 0;

CREATE TABLE IF NOT EXISTS forced_mode_state (
    plan_id         TEXT PRIMARY KEY NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    started_at_utc  TEXT NOT NULL,
    expires_at_utc  TEXT NOT NULL,
    monotonic_start INTEGER NOT NULL,
    monotonic_duration_s INTEGER NOT NULL,
    emergency_code_hash TEXT,
    active          INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY NOT NULL,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('schema_version', '1.0.0', datetime('now')),
    ('telemetry_consent', 'essential_only', datetime('now')),
    ('log_level', 'INFO', datetime('now')),
    ('debug_ipc_json', '0', datetime('now')),
    ('forced_mode_max_cap_s', '86400', datetime('now')),
    ('process_poll_interval_ms', '500', datetime('now')),
    ('hosts_restore_timeout_ms', '2000', datetime('now'));
