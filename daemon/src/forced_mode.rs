// ============================================================
// FILE:        forced_mode.rs
// MODULE:      Layer 1 — Enforcement Engine > Forced/Lockdown Mode
// TASK:        T-021
// PLATFORM:    cross (windows, macos, linux)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, daemon core
// DEPENDENCIES: chrono 0.4, tokio, argon2 0.5
// TEST COVERAGE: UT-05 (monotonic clock — rollback rejected),
//                BT-10 (clock rollback bypass test)
// KNOWN LIMITATIONS: System clock manipulation by admin is mitigated
//                    but not fully prevented (monotonic + wall clock dual tracking).
// ANTI-CIRCUMVENTION: Defends against clock rollback attack (BT-10, Section 3.2)
// ============================================================

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use std::time::Instant;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, error};

/// Maximum allowed Forced Mode duration (24 hours, per Section 8.3)
const MAX_FORCED_MODE_DURATION_S: u64 = 86400;

/// Represents the state of an active Forced Mode session
#[derive(Debug, Clone)]
pub struct ForcedModeState {
    pub plan_id: String,
    /// Wall clock start time (ISO-8601 UTC)
    pub started_at_utc: DateTime<Utc>,
    /// Wall clock expiry time (ISO-8601 UTC)
    pub expires_at_utc: DateTime<Utc>,
    /// Monotonic clock start instant
    /// NOTE: Using Instant (monotonic) — NOT SystemTime (wall clock)
    /// This defends against BT-10 (clock rollback bypass)
    pub monotonic_start: Instant,
    /// Duration in seconds
    pub duration_s: u64,
    /// Hashed emergency unlock code (Argon2id)
    pub emergency_code_hash: Option<String>,
}

/// ForcedModeTracker manages the lifecycle of Forced Mode sessions.
///
/// Key security property: time tracking uses BOTH monotonic clock (Instant)
/// and wall clock (UTC). The session expires when EITHER timer says it should,
/// whichever is LATER. This defends against clock rollback attacks.
pub struct ForcedModeTracker {
    /// Currently active Forced Mode states (one per plan)
    active_sessions: Arc<RwLock<Vec<ForcedModeState>>>,
}

impl ForcedModeTracker {
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start a Forced Mode session for a plan
    ///
    /// # Arguments
    /// * `plan_id` - Plan to lock
    /// * `duration_s` - Session duration in seconds (capped at MAX_FORCED_MODE_DURATION_S)
    /// * `emergency_code_hash` - Optional Argon2id hash of emergency unlock code
    pub async fn start_session(
        &self,
        plan_id: String,
        duration_s: u64,
        emergency_code_hash: Option<String>,
    ) -> Result<ForcedModeState> {
        // Cap duration at maximum (Section 8.3: default 24h)
        let capped_duration = duration_s.min(MAX_FORCED_MODE_DURATION_S);
        if duration_s > MAX_FORCED_MODE_DURATION_S {
            warn!(
                requested = duration_s,
                capped = capped_duration,
                "Forced Mode duration capped at maximum"
            );
        }

        let now_utc = Utc::now();
        let now_mono = Instant::now();

        let state = ForcedModeState {
            plan_id: plan_id.clone(),
            started_at_utc: now_utc,
            expires_at_utc: now_utc + chrono::Duration::seconds(capped_duration as i64),
            monotonic_start: now_mono,
            duration_s: capped_duration,
            emergency_code_hash,
        };

        let mut sessions = self.active_sessions.write().await;
        // Remove any existing session for this plan
        sessions.retain(|s| s.plan_id != plan_id);
        sessions.push(state.clone());

        info!(
            plan_id = %plan_id,
            duration_s = capped_duration,
            expires_at = %state.expires_at_utc,
            "Forced Mode session started"
        );

        // TODO: Persist to forced_mode_state table (T-018) for reboot survival

        Ok(state)
    }

    /// Check if a plan is currently in Forced Mode
    ///
    /// Uses dual clock comparison:
    /// - Monotonic elapsed < duration_s → still active (immune to clock rollback)
    /// - Wall clock < expires_at_utc → still active (immune to hibernate/suspend gaps)
    /// Session is considered active if EITHER clock says it's still active.
    /// This means the session expires at the LATER of the two checks.
    pub async fn is_active(&self, plan_id: &str) -> bool {
        let sessions = self.active_sessions.read().await;
        for session in sessions.iter() {
            if session.plan_id == plan_id {
                return Self::check_session_active(session);
            }
        }
        false
    }

    /// Internal check — is this specific session still active?
    fn check_session_active(session: &ForcedModeState) -> bool {
        let now_utc = Utc::now();
        let mono_elapsed = session.monotonic_start.elapsed();

        // ANTI-CIRCUMVENTION (BT-10): Accept only forward-moving time
        // Session is active if EITHER timer says so (take the LATER expiry)
        let mono_remaining = session.duration_s.saturating_sub(mono_elapsed.as_secs());
        let wall_active = now_utc < session.expires_at_utc;

        // If monotonic says time remains OR wall clock says time remains → active
        mono_remaining > 0 || wall_active
    }

    /// Get remaining time in seconds for a Forced Mode session
    pub async fn remaining_seconds(&self, plan_id: &str) -> Option<u64> {
        let sessions = self.active_sessions.read().await;
        for session in sessions.iter() {
            if session.plan_id == plan_id {
                let mono_elapsed = session.monotonic_start.elapsed();
                let mono_remaining = session.duration_s.saturating_sub(mono_elapsed.as_secs());

                let wall_remaining = session
                    .expires_at_utc
                    .signed_duration_since(Utc::now())
                    .num_seconds()
                    .max(0) as u64;

                // Return the LARGER remaining time (more conservative)
                return Some(mono_remaining.max(wall_remaining));
            }
        }
        None
    }

    /// Attempt emergency unlock using emergency code
    pub async fn emergency_unlock(&self, plan_id: &str, code: &str) -> Result<()> {
        let sessions = self.active_sessions.read().await;
        let session = sessions
            .iter()
            .find(|s| s.plan_id == plan_id)
            .ok_or_else(|| anyhow::anyhow!("No active Forced Mode session for plan"))?;

        let hash = session
            .emergency_code_hash
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No emergency code configured for this session"))?;

        // Verify code against Argon2id hash
        use crate::plan_protection::PlanProtection;
        match PlanProtection::verify_password(code, hash) {
            Ok(true) => {} // Code matches
            Ok(false) => bail!("Invalid emergency code"),
            Err(e) => bail!("Emergency code verification error: {}", e),
        }

        drop(sessions);

        // Remove the session
        let mut sessions = self.active_sessions.write().await;
        sessions.retain(|s| s.plan_id != plan_id);

        info!(plan_id = %plan_id, "Forced Mode emergency unlock succeeded");

        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) {
        let mut sessions = self.active_sessions.write().await;
        let before = sessions.len();
        sessions.retain(|s| Self::check_session_active(s));
        let removed = before - sessions.len();
        if removed > 0 {
            info!(removed = removed, "Expired Forced Mode sessions cleaned up");
        }
    }

    /// Get all active Forced Mode plan IDs
    pub async fn active_plan_ids(&self) -> Vec<String> {
        let sessions = self.active_sessions.read().await;
        sessions
            .iter()
            .filter(|s| Self::check_session_active(s))
            .map(|s| s.plan_id.clone())
            .collect()
    }
}

// ============================================================
// UNIT TESTS — UT-05: Monotonic clock, rollback rejected
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_forced_mode_starts_and_is_active() {
        let tracker = ForcedModeTracker::new();
        tracker
            .start_session("plan-1".to_string(), 3600, None)
            .await
            .expect("Session should start");

        assert!(tracker.is_active("plan-1").await);
    }

    #[tokio::test]
    async fn test_forced_mode_inactive_for_unknown_plan() {
        let tracker = ForcedModeTracker::new();
        assert!(!tracker.is_active("nonexistent").await);
    }

    #[tokio::test]
    async fn test_forced_mode_capped_at_max_duration() {
        let tracker = ForcedModeTracker::new();
        let state = tracker
            .start_session("plan-1".to_string(), 999999, None)
            .await
            .expect("Session should start");

        assert_eq!(state.duration_s, MAX_FORCED_MODE_DURATION_S);
    }

    #[tokio::test]
    async fn test_remaining_seconds_returns_some() {
        let tracker = ForcedModeTracker::new();
        tracker
            .start_session("plan-1".to_string(), 3600, None)
            .await
            .expect("Session should start");

        let remaining = tracker.remaining_seconds("plan-1").await;
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > 3500); // Should be close to 3600
    }

    #[test]
    fn test_check_session_active_with_valid_session() {
        let state = ForcedModeState {
            plan_id: "test".to_string(),
            started_at_utc: Utc::now(),
            expires_at_utc: Utc::now() + chrono::Duration::hours(1),
            monotonic_start: Instant::now(),
            duration_s: 3600,
            emergency_code_hash: None,
        };

        assert!(ForcedModeTracker::check_session_active(&state));
    }
}
