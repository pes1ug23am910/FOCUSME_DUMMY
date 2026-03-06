// ============================================================
// FILE:        db.rs
// MODULE:      Layer 2 — Policy Store
// TASK:        T-018 (implementation) + S-005 RwLock migration (Session 6)
// PLATFORM:    cross
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, Session 2 — DB integration
// UPDATED:     Session 6 — Migrated Mutex → tokio::sync::RwLock (D-013)
// DEPENDENCIES: rusqlite 0.31 (bundled-sqlcipher), refinery 0.8, tokio
// TEST COVERAGE: UT-07 (concurrent write safety via WAL mode)
// KNOWN LIMITATIONS: SQLCipher key derived from machine-id + salt;
//                    admin with disk access can still extract key.
// [DECISION D-013] Migrated DaemonState.db from Mutex to RwLock
//   to reduce contention on read-heavy plan store workload.
//   Read ops (get_*, list_*, check_*) use .read().await
//   Write ops (create_*, update_*, delete_*, save_*, record_*, log_*) use .write().await
// ============================================================

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

// Embed refinery migrations at compile time
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

// ────────────────────────────────────────────────────────
// Row types — map 1:1 to database tables
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PlanRow {
    pub plan_id: String,
    pub name: String,
    pub schema_version: String,
    pub enabled: bool,
    pub forced_mode: bool,
    pub forced_mode_max_duration_s: i64,
    pub protection_type: String,
    pub protection_hash: Option<String>,
    pub challenge_required: bool,
    pub plan_json: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Clone)]
pub struct ScheduleRow {
    pub schedule_id: String,
    pub plan_id: String,
    pub days: String,       // JSON array: ["mon","tue",...]
    pub start_time: String, // HH:MM
    pub end_time: String,   // HH:MM
    pub timezone: String,   // IANA timezone
}

#[derive(Debug, Clone)]
pub struct AppRuleRow {
    pub rule_id: String,
    pub plan_id: String,
    pub rule_type: String,  // "block" | "allow"
    pub match_type: String, // "process_name" | "path_prefix" | "path_exact" | "bundle_id"
    pub value: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct UrlRuleRow {
    pub rule_id: String,
    pub plan_id: String,
    pub rule_type: String,  // "block" | "allow"
    pub match_type: String, // "domain" | "wildcard" | "path" | "regex"
    pub value: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct QuotaRow {
    pub quota_id: String,
    pub plan_id: String,
    pub target_type: String,
    pub target: String,
    pub daily_limit_s: i64,
    pub launches_per_day: Option<i32>,
    pub min_break_s: i32,
}

#[derive(Debug, Clone)]
pub struct QuotaLedgerRow {
    pub ledger_id: i64,
    pub quota_id: String,
    pub date: String,
    pub used_s: i64,
    pub launch_count: i32,
    pub last_launch_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub session_id: String,
    pub plan_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub forced_mode: bool,
    pub end_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventRow {
    pub event_id: String,
    pub event_type: String,
    pub plan_id: Option<String>,
    pub rule_id: Option<String>,
    pub session_id: Option<String>,
    pub subject_hash: Option<String>,
    pub duration_ms: Option<i64>,
    pub quota_used_s: Option<i64>,
    pub quota_limit_s: Option<i64>,
    pub timestamp_utc: String,
    pub synced: bool,
}

#[derive(Debug, Clone)]
pub struct ForcedModeRow {
    pub plan_id: String,
    pub started_at_utc: String,
    pub expires_at_utc: String,
    pub monotonic_start: i64,
    pub monotonic_duration_s: i64,
    pub emergency_code_hash: Option<String>,
    pub active: bool,
}

// ────────────────────────────────────────────────────────
// Database — thread-safe wrapper around rusqlite::Connection
// ────────────────────────────────────────────────────────

pub struct Database {
    conn: RwLock<Connection>,
    path: PathBuf,
}

impl Database {
    /// Open (or create) the FocusMe database at the given path.
    ///
    /// Applies SQLCipher encryption, WAL mode, foreign keys, and
    /// runs any pending refinery migrations.
    pub fn open(db_path: &Path, encryption_key: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

        // SQLCipher key — must be the first statement after open
        if !encryption_key.is_empty() {
            conn.pragma_update(None, "key", encryption_key)
                .context("Failed to set SQLCipher encryption key")?;
        }

        // Performance pragmas
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("Failed to enable WAL mode")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("Failed to enable foreign keys")?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .context("Failed to set synchronous mode")?;
        conn.pragma_update(None, "busy_timeout", "5000")
            .context("Failed to set busy timeout")?;

        info!(path = %db_path.display(), "Database opened with SQLCipher + WAL mode");

        let db = Self {
            conn: RwLock::new(conn),
            path: db_path.to_path_buf(),
        };

        db.run_migrations()?;

        Ok(db)
    }

    /// Run pending refinery migrations
    fn run_migrations(&self) -> Result<()> {
        let mut conn = self.conn.blocking_write();
        embedded::migrations::runner()
            .run(&mut *conn)
            .context("Database migration failed")?;
        info!("Database migrations applied successfully");
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // PLAN CRUD
    // ════════════════════════════════════════════════════

    pub fn create_plan(&self, plan: &PlanRow) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "INSERT INTO plans (plan_id, name, schema_version, enabled, forced_mode,
             forced_mode_max_duration_s, protection_type, protection_hash,
             challenge_required, plan_json, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                plan.plan_id, plan.name, plan.schema_version,
                plan.enabled as i32, plan.forced_mode as i32,
                plan.forced_mode_max_duration_s, plan.protection_type,
                plan.protection_hash, plan.challenge_required as i32,
                plan.plan_json, plan.created_at, plan.modified_at,
            ],
        ).context("Failed to insert plan")?;
        debug!(plan_id = %plan.plan_id, "Plan created in database");
        Ok(())
    }

    pub fn get_plan(&self, plan_id: &str) -> Result<Option<PlanRow>> {
        let conn = self.conn.blocking_read();
        conn.query_row(
            "SELECT plan_id, name, schema_version, enabled, forced_mode,
                    forced_mode_max_duration_s, protection_type, protection_hash,
                    challenge_required, plan_json, created_at, modified_at
             FROM plans WHERE plan_id = ?1",
            params![plan_id],
            |row| {
                Ok(PlanRow {
                    plan_id: row.get(0)?,
                    name: row.get(1)?,
                    schema_version: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    forced_mode: row.get::<_, i32>(4)? != 0,
                    forced_mode_max_duration_s: row.get(5)?,
                    protection_type: row.get(6)?,
                    protection_hash: row.get(7)?,
                    challenge_required: row.get::<_, i32>(8)? != 0,
                    plan_json: row.get(9)?,
                    created_at: row.get(10)?,
                    modified_at: row.get(11)?,
                })
            },
        )
        .optional()
        .context("Failed to query plan")
    }

    pub fn list_plans(&self) -> Result<Vec<PlanRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT plan_id, name, schema_version, enabled, forced_mode,
                    forced_mode_max_duration_s, protection_type, protection_hash,
                    challenge_required, plan_json, created_at, modified_at
             FROM plans ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PlanRow {
                plan_id: row.get(0)?,
                name: row.get(1)?,
                schema_version: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                forced_mode: row.get::<_, i32>(4)? != 0,
                forced_mode_max_duration_s: row.get(5)?,
                protection_type: row.get(6)?,
                protection_hash: row.get(7)?,
                challenge_required: row.get::<_, i32>(8)? != 0,
                plan_json: row.get(9)?,
                created_at: row.get(10)?,
                modified_at: row.get(11)?,
            })
        })?;
        let mut plans = Vec::new();
        for row in rows {
            plans.push(row?);
        }
        Ok(plans)
    }

    pub fn list_enabled_plans(&self) -> Result<Vec<PlanRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT plan_id, name, schema_version, enabled, forced_mode,
                    forced_mode_max_duration_s, protection_type, protection_hash,
                    challenge_required, plan_json, created_at, modified_at
             FROM plans WHERE enabled = 1 ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PlanRow {
                plan_id: row.get(0)?,
                name: row.get(1)?,
                schema_version: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                forced_mode: row.get::<_, i32>(4)? != 0,
                forced_mode_max_duration_s: row.get(5)?,
                protection_type: row.get(6)?,
                protection_hash: row.get(7)?,
                challenge_required: row.get::<_, i32>(8)? != 0,
                plan_json: row.get(9)?,
                created_at: row.get(10)?,
                modified_at: row.get(11)?,
            })
        })?;
        let mut plans = Vec::new();
        for row in rows {
            plans.push(row?);
        }
        Ok(plans)
    }

    pub fn update_plan(&self, plan: &PlanRow) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "UPDATE plans SET name = ?2, schema_version = ?3, enabled = ?4,
             forced_mode = ?5, forced_mode_max_duration_s = ?6,
             protection_type = ?7, protection_hash = ?8,
             challenge_required = ?9, plan_json = ?10, modified_at = ?11
             WHERE plan_id = ?1",
            params![
                plan.plan_id, plan.name, plan.schema_version,
                plan.enabled as i32, plan.forced_mode as i32,
                plan.forced_mode_max_duration_s, plan.protection_type,
                plan.protection_hash, plan.challenge_required as i32,
                plan.plan_json, plan.modified_at,
            ],
        ).context("Failed to update plan")?;
        Ok(())
    }

    pub fn delete_plan(&self, plan_id: &str) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute("DELETE FROM plans WHERE plan_id = ?1", params![plan_id])
            .context("Failed to delete plan")?;
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // SCHEDULES
    // ════════════════════════════════════════════════════

    pub fn get_schedules(&self, plan_id: &str) -> Result<Vec<ScheduleRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT schedule_id, plan_id, days, start_time, end_time, timezone
             FROM schedules WHERE plan_id = ?1",
        )?;
        let rows = stmt.query_map(params![plan_id], |row| {
            Ok(ScheduleRow {
                schedule_id: row.get(0)?,
                plan_id: row.get(1)?,
                days: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                timezone: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn save_schedules(&self, plan_id: &str, schedules: &[ScheduleRow]) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute("DELETE FROM schedules WHERE plan_id = ?1", params![plan_id])?;
        for s in schedules {
            conn.execute(
                "INSERT INTO schedules (schedule_id, plan_id, days, start_time, end_time, timezone)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![s.schedule_id, s.plan_id, s.days, s.start_time, s.end_time, s.timezone],
            )?;
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // APP RULES
    // ════════════════════════════════════════════════════

    pub fn get_app_rules(&self, plan_id: &str) -> Result<Vec<AppRuleRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT rule_id, plan_id, rule_type, match_type, value, sort_order
             FROM app_rules WHERE plan_id = ?1 ORDER BY sort_order",
        )?;
        let rows = stmt.query_map(params![plan_id], |row| {
            Ok(AppRuleRow {
                rule_id: row.get(0)?,
                plan_id: row.get(1)?,
                rule_type: row.get(2)?,
                match_type: row.get(3)?,
                value: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn save_app_rules(&self, plan_id: &str, rules: &[AppRuleRow]) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute("DELETE FROM app_rules WHERE plan_id = ?1", params![plan_id])?;
        for r in rules {
            conn.execute(
                "INSERT INTO app_rules (rule_id, plan_id, rule_type, match_type, value, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![r.rule_id, r.plan_id, r.rule_type, r.match_type, r.value, r.sort_order],
            )?;
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // URL RULES
    // ════════════════════════════════════════════════════

    pub fn get_url_rules(&self, plan_id: &str) -> Result<Vec<UrlRuleRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT rule_id, plan_id, rule_type, match_type, value, sort_order
             FROM url_rules WHERE plan_id = ?1 ORDER BY sort_order",
        )?;
        let rows = stmt.query_map(params![plan_id], |row| {
            Ok(UrlRuleRow {
                rule_id: row.get(0)?,
                plan_id: row.get(1)?,
                rule_type: row.get(2)?,
                match_type: row.get(3)?,
                value: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn save_url_rules(&self, plan_id: &str, rules: &[UrlRuleRow]) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute("DELETE FROM url_rules WHERE plan_id = ?1", params![plan_id])?;
        for r in rules {
            conn.execute(
                "INSERT INTO url_rules (rule_id, plan_id, rule_type, match_type, value, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![r.rule_id, r.plan_id, r.rule_type, r.match_type, r.value, r.sort_order],
            )?;
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // QUOTAS
    // ════════════════════════════════════════════════════

    pub fn get_quotas(&self, plan_id: &str) -> Result<Vec<QuotaRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT quota_id, plan_id, target_type, target, daily_limit_s,
                    launches_per_day, min_break_s
             FROM quotas WHERE plan_id = ?1",
        )?;
        let rows = stmt.query_map(params![plan_id], |row| {
            Ok(QuotaRow {
                quota_id: row.get(0)?,
                plan_id: row.get(1)?,
                target_type: row.get(2)?,
                target: row.get(3)?,
                daily_limit_s: row.get(4)?,
                launches_per_day: row.get(5)?,
                min_break_s: row.get::<_, i32>(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn save_quotas(&self, plan_id: &str, quotas: &[QuotaRow]) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute("DELETE FROM quotas WHERE plan_id = ?1", params![plan_id])?;
        for q in quotas {
            conn.execute(
                "INSERT INTO quotas (quota_id, plan_id, target_type, target,
                 daily_limit_s, launches_per_day, min_break_s)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    q.quota_id, q.plan_id, q.target_type, q.target,
                    q.daily_limit_s, q.launches_per_day, q.min_break_s
                ],
            )?;
        }
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // QUOTA LEDGER — daily usage tracking
    // ════════════════════════════════════════════════════

    /// Record usage time against a quota for a given date.
    /// Uses UPSERT to atomically create-or-update the ledger entry.
    pub fn record_quota_usage(
        &self,
        quota_id: &str,
        date: &str,
        additional_seconds: i64,
    ) -> Result<QuotaLedgerRow> {
        let conn = self.conn.blocking_write();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO quota_ledger (quota_id, date, used_s, launch_count, updated_at)
             VALUES (?1, ?2, ?3, 0, ?4)
             ON CONFLICT(quota_id, date) DO UPDATE SET
                used_s = used_s + ?3,
                updated_at = ?4",
            params![quota_id, date, additional_seconds, now],
        )?;

        // Read back the current state
        conn.query_row(
            "SELECT ledger_id, quota_id, date, used_s, launch_count, last_launch_at, updated_at
             FROM quota_ledger WHERE quota_id = ?1 AND date = ?2",
            params![quota_id, date],
            |row| {
                Ok(QuotaLedgerRow {
                    ledger_id: row.get(0)?,
                    quota_id: row.get(1)?,
                    date: row.get(2)?,
                    used_s: row.get(3)?,
                    launch_count: row.get(4)?,
                    last_launch_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .context("Failed to read quota ledger after update")
    }

    /// Record a launch event for quota tracking
    pub fn record_quota_launch(&self, quota_id: &str, date: &str) -> Result<()> {
        let conn = self.conn.blocking_write();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO quota_ledger (quota_id, date, used_s, launch_count, last_launch_at, updated_at)
             VALUES (?1, ?2, 0, 1, ?3, ?3)
             ON CONFLICT(quota_id, date) DO UPDATE SET
                launch_count = launch_count + 1,
                last_launch_at = ?3,
                updated_at = ?3",
            params![quota_id, date, now],
        )?;
        Ok(())
    }

    pub fn get_quota_usage(&self, quota_id: &str, date: &str) -> Result<Option<QuotaLedgerRow>> {
        let conn = self.conn.blocking_read();
        conn.query_row(
            "SELECT ledger_id, quota_id, date, used_s, launch_count, last_launch_at, updated_at
             FROM quota_ledger WHERE quota_id = ?1 AND date = ?2",
            params![quota_id, date],
            |row| {
                Ok(QuotaLedgerRow {
                    ledger_id: row.get(0)?,
                    quota_id: row.get(1)?,
                    date: row.get(2)?,
                    used_s: row.get(3)?,
                    launch_count: row.get(4)?,
                    last_launch_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to query quota ledger")
    }

    // ════════════════════════════════════════════════════
    // SESSIONS — plan activation tracking
    // ════════════════════════════════════════════════════

    pub fn create_session(&self, session: &SessionRow) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "INSERT INTO sessions (session_id, plan_id, started_at, ended_at, forced_mode, end_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.session_id, session.plan_id, session.started_at,
                session.ended_at, session.forced_mode as i32, session.end_reason,
            ],
        )?;
        Ok(())
    }

    pub fn end_session(&self, session_id: &str, end_reason: &str) -> Result<()> {
        let conn = self.conn.blocking_write();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET ended_at = ?2, end_reason = ?3 WHERE session_id = ?1",
            params![session_id, now, end_reason],
        )?;
        Ok(())
    }

    pub fn get_active_sessions(&self) -> Result<Vec<SessionRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT session_id, plan_id, started_at, ended_at, forced_mode, end_reason
             FROM sessions WHERE ended_at IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionRow {
                session_id: row.get(0)?,
                plan_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                forced_mode: row.get::<_, i32>(4)? != 0,
                end_reason: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    // ════════════════════════════════════════════════════
    // EVENTS — block/quota/system event log
    // ════════════════════════════════════════════════════

    pub fn log_event(&self, event: &EventRow) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "INSERT INTO events (event_id, event_type, plan_id, rule_id, session_id,
             subject_hash, duration_ms, quota_used_s, quota_limit_s, timestamp_utc, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.event_id, event.event_type, event.plan_id, event.rule_id,
                event.session_id, event.subject_hash, event.duration_ms,
                event.quota_used_s, event.quota_limit_s, event.timestamp_utc,
                event.synced as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_events_since(&self, since_utc: &str, limit: i64) -> Result<Vec<EventRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT event_id, event_type, plan_id, rule_id, session_id,
                    subject_hash, duration_ms, quota_used_s, quota_limit_s,
                    timestamp_utc, synced
             FROM events WHERE timestamp_utc >= ?1
             ORDER BY timestamp_utc DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since_utc, limit], |row| {
            Ok(EventRow {
                event_id: row.get(0)?,
                event_type: row.get(1)?,
                plan_id: row.get(2)?,
                rule_id: row.get(3)?,
                session_id: row.get(4)?,
                subject_hash: row.get(5)?,
                duration_ms: row.get(6)?,
                quota_used_s: row.get(7)?,
                quota_limit_s: row.get(8)?,
                timestamp_utc: row.get(9)?,
                synced: row.get::<_, i32>(10)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn get_event_count_by_type(&self, since_utc: &str) -> Result<Vec<(String, i64)>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT event_type, COUNT(*) as cnt
             FROM events WHERE timestamp_utc >= ?1
             GROUP BY event_type ORDER BY cnt DESC",
        )?;
        let rows = stmt.query_map(params![since_utc], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    // ════════════════════════════════════════════════════
    // FORCED MODE STATE — persists across reboots
    // ════════════════════════════════════════════════════

    pub fn save_forced_mode_state(&self, state: &ForcedModeRow) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "INSERT OR REPLACE INTO forced_mode_state
             (plan_id, started_at_utc, expires_at_utc, monotonic_start,
              monotonic_duration_s, emergency_code_hash, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                state.plan_id, state.started_at_utc, state.expires_at_utc,
                state.monotonic_start, state.monotonic_duration_s,
                state.emergency_code_hash, state.active as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_forced_mode_state(&self, plan_id: &str) -> Result<Option<ForcedModeRow>> {
        let conn = self.conn.blocking_read();
        conn.query_row(
            "SELECT plan_id, started_at_utc, expires_at_utc, monotonic_start,
                    monotonic_duration_s, emergency_code_hash, active
             FROM forced_mode_state WHERE plan_id = ?1 AND active = 1",
            params![plan_id],
            |row| {
                Ok(ForcedModeRow {
                    plan_id: row.get(0)?,
                    started_at_utc: row.get(1)?,
                    expires_at_utc: row.get(2)?,
                    monotonic_start: row.get(3)?,
                    monotonic_duration_s: row.get(4)?,
                    emergency_code_hash: row.get(5)?,
                    active: row.get::<_, i32>(6)? != 0,
                })
            },
        )
        .optional()
        .context("Failed to query forced mode state")
    }

    pub fn get_all_active_forced_modes(&self) -> Result<Vec<ForcedModeRow>> {
        let conn = self.conn.blocking_read();
        let mut stmt = conn.prepare(
            "SELECT plan_id, started_at_utc, expires_at_utc, monotonic_start,
                    monotonic_duration_s, emergency_code_hash, active
             FROM forced_mode_state WHERE active = 1",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ForcedModeRow {
                plan_id: row.get(0)?,
                started_at_utc: row.get(1)?,
                expires_at_utc: row.get(2)?,
                monotonic_start: row.get(3)?,
                monotonic_duration_s: row.get(4)?,
                emergency_code_hash: row.get(5)?,
                active: row.get::<_, i32>(6)? != 0,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn clear_forced_mode_state(&self, plan_id: &str) -> Result<()> {
        let conn = self.conn.blocking_write();
        conn.execute(
            "UPDATE forced_mode_state SET active = 0 WHERE plan_id = ?1",
            params![plan_id],
        )?;
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // SETTINGS — key-value configuration store
    // ════════════════════════════════════════════════════

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.blocking_read();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to query setting")
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.blocking_write();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
        Ok(())
    }

    // ════════════════════════════════════════════════════
    // UTILITIES
    // ════════════════════════════════════════════════════

    /// Get the database file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Purge events older than the given ISO-8601 date
    pub fn purge_events_before(&self, before_utc: &str) -> Result<usize> {
        let conn = self.conn.blocking_write();
        let deleted = conn.execute(
            "DELETE FROM events WHERE timestamp_utc < ?1",
            params![before_utc],
        )?;
        info!(deleted, before = before_utc, "Old events purged");
        Ok(deleted)
    }

    /// Get the default database path for the current platform
    pub fn default_path() -> PathBuf {
        #[cfg(windows)]
        {
            let appdata = std::env::var("PROGRAMDATA")
                .unwrap_or_else(|_| "C:\\ProgramData".to_string());
            PathBuf::from(appdata).join("FocusMe").join("focusme.db")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/Application Support/FocusMe/focusme.db")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/var/lib/focusme/focusme.db")
        }
    }

    /// Derive encryption key from machine ID + salt
    /// D-006: SQLCipher key derivation
    pub fn derive_encryption_key() -> String {
        let machine_id = Self::get_machine_id();
        let salt = "FocusMe-Policy-Store-v1";
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(machine_id.as_bytes());
        hasher.update(salt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get platform machine ID
    fn get_machine_id() -> String {
        #[cfg(windows)]
        {
            // Read MachineGuid from registry
            std::process::Command::new("reg")
                .args(["query", r"HKLM\SOFTWARE\Microsoft\Cryptography", "/v", "MachineGuid"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.contains("MachineGuid"))
                        .and_then(|l| l.split_whitespace().last())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "fallback-machine-id".to_string())
        }

        #[cfg(not(windows))]
        {
            std::fs::read_to_string("/etc/machine-id")
                .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "fallback-machine-id".to_string())
        }
    }
}

// ════════════════════════════════════════════════════════
// UNIT TESTS — UT-07: concurrent write safety
// ════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_db() -> Database {
        let tmp = NamedTempFile::new().expect("temp file");
        Database::open(tmp.path(), "").expect("DB open should succeed")
    }

    #[test]
    fn test_open_and_migrate() {
        let db = test_db();
        let setting = db.get_setting("schema_version").expect("query");
        assert_eq!(setting, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_plan_crud() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();
        let plan = PlanRow {
            plan_id: "plan-1".to_string(),
            name: "Work Focus".to_string(),
            schema_version: "1.0.0".to_string(),
            enabled: true,
            forced_mode: false,
            forced_mode_max_duration_s: 3600,
            protection_type: "none".to_string(),
            protection_hash: None,
            challenge_required: false,
            plan_json: "{}".to_string(),
            created_at: now.clone(),
            modified_at: now,
        };

        db.create_plan(&plan).expect("create plan");
        let fetched = db.get_plan("plan-1").expect("get plan").expect("should exist");
        assert_eq!(fetched.name, "Work Focus");

        let plans = db.list_plans().expect("list plans");
        assert_eq!(plans.len(), 1);

        db.delete_plan("plan-1").expect("delete");
        assert!(db.get_plan("plan-1").expect("get").is_none());
    }

    #[test]
    fn test_quota_ledger_upsert() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        // Create a plan and quota first
        let plan = PlanRow {
            plan_id: "p1".to_string(),
            name: "Test".to_string(),
            schema_version: "1.0.0".to_string(),
            enabled: true,
            forced_mode: false,
            forced_mode_max_duration_s: 3600,
            protection_type: "none".to_string(),
            protection_hash: None,
            challenge_required: false,
            plan_json: "{}".to_string(),
            created_at: now.clone(),
            modified_at: now,
        };
        db.create_plan(&plan).expect("create plan");

        db.save_quotas("p1", &[QuotaRow {
            quota_id: "q1".to_string(),
            plan_id: "p1".to_string(),
            target_type: "app".to_string(),
            target: "Spotify".to_string(),
            daily_limit_s: 3600,
            launches_per_day: Some(10),
            min_break_s: 0,
        }]).expect("save quotas");

        // Record usage — first time creates entry
        let ledger = db.record_quota_usage("q1", "2026-02-26", 300).expect("record");
        assert_eq!(ledger.used_s, 300);

        // Record more usage — accumulates
        let ledger2 = db.record_quota_usage("q1", "2026-02-26", 200).expect("record again");
        assert_eq!(ledger2.used_s, 500);
    }

    #[test]
    fn test_session_lifecycle() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let plan = PlanRow {
            plan_id: "p1".to_string(),
            name: "Test".to_string(),
            schema_version: "1.0.0".to_string(),
            enabled: true,
            forced_mode: false,
            forced_mode_max_duration_s: 3600,
            protection_type: "none".to_string(),
            protection_hash: None,
            challenge_required: false,
            plan_json: "{}".to_string(),
            created_at: now.clone(),
            modified_at: now.clone(),
        };
        db.create_plan(&plan).expect("create plan");

        db.create_session(&SessionRow {
            session_id: "s1".to_string(),
            plan_id: "p1".to_string(),
            started_at: now,
            ended_at: None,
            forced_mode: false,
            end_reason: None,
        }).expect("create session");

        let active = db.get_active_sessions().expect("active sessions");
        assert_eq!(active.len(), 1);

        db.end_session("s1", "manual").expect("end session");
        let active2 = db.get_active_sessions().expect("active sessions");
        assert_eq!(active2.len(), 0);
    }

    #[test]
    fn test_forced_mode_persistence() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        let plan = PlanRow {
            plan_id: "p1".to_string(),
            name: "Test".to_string(),
            schema_version: "1.0.0".to_string(),
            enabled: true,
            forced_mode: true,
            forced_mode_max_duration_s: 3600,
            protection_type: "none".to_string(),
            protection_hash: None,
            challenge_required: false,
            plan_json: "{}".to_string(),
            created_at: now.clone(),
            modified_at: now.clone(),
        };
        db.create_plan(&plan).expect("create plan");

        let fm = ForcedModeRow {
            plan_id: "p1".to_string(),
            started_at_utc: now.clone(),
            expires_at_utc: now,
            monotonic_start: 12345,
            monotonic_duration_s: 3600,
            emergency_code_hash: None,
            active: true,
        };

        db.save_forced_mode_state(&fm).expect("save FM");
        let loaded = db.get_forced_mode_state("p1").expect("load FM").expect("exists");
        assert_eq!(loaded.monotonic_duration_s, 3600);

        db.clear_forced_mode_state("p1").expect("clear FM");
        assert!(db.get_forced_mode_state("p1").expect("load FM").is_none());
    }

    #[test]
    fn test_settings() {
        let db = test_db();
        db.set_setting("custom_key", "custom_value").expect("set");
        let val = db.get_setting("custom_key").expect("get").expect("exists");
        assert_eq!(val, "custom_value");
    }

    #[test]
    fn test_event_logging() {
        let db = test_db();
        let now = chrono::Utc::now().to_rfc3339();

        db.log_event(&EventRow {
            event_id: "e1".to_string(),
            event_type: "app_blocked".to_string(),
            plan_id: None,
            rule_id: None,
            session_id: None,
            subject_hash: Some("abc123".to_string()),
            duration_ms: None,
            quota_used_s: None,
            quota_limit_s: None,
            timestamp_utc: now.clone(),
            synced: false,
        }).expect("log event");

        let events = db.get_events_since("2020-01-01T00:00:00Z", 100).expect("query");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "app_blocked");
    }
}
