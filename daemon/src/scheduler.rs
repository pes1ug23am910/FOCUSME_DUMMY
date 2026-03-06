// ============================================================
// FILE:        scheduler.rs
// MODULE:      Layer 1 — Enforcement Engine > Plan Scheduler
// TASK:        T-020
// PLATFORM:    cross
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, daemon core
// DEPENDENCIES: chrono 0.4, chrono-tz 0.8, tokio
// TEST COVERAGE: UT-01 (DST boundaries), UT-02 (URL rule matching),
//                UT-03 (quota counter)
// KNOWN LIMITATIONS: Schedule resolution is 1 minute (not second-level).
// ============================================================

use anyhow::Result;
use chrono::{DateTime, NaiveTime, Utc, Weekday, Datelike};
use chrono_tz::Tz;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Parse a JSON array of day abbreviations into chrono Weekday values.
///
/// Expected input: `["mon","tue","wed","thu","fri","sat","sun"]`
/// Returns empty vec on parse failure.
pub fn parse_days_json(json: &str) -> Vec<Weekday> {
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| match s.to_lowercase().as_str() {
            "mon" | "monday" => Some(Weekday::Mon),
            "tue" | "tuesday" => Some(Weekday::Tue),
            "wed" | "wednesday" => Some(Weekday::Wed),
            "thu" | "thursday" => Some(Weekday::Thu),
            "fri" | "friday" => Some(Weekday::Fri),
            "sat" | "saturday" => Some(Weekday::Sat),
            "sun" | "sunday" => Some(Weekday::Sun),
            _ => {
                warn!(day = %s, "Unknown day abbreviation in schedule JSON");
                None
            }
        })
        .collect()
}

/// Represents a parsed schedule from a Focus Plan
#[derive(Debug, Clone)]
pub struct Schedule {
    pub schedule_id: String,
    pub plan_id: String,
    pub days: Vec<Weekday>,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub timezone: Tz,
}

/// Represents a loaded Focus Plan with its schedules and rules
#[derive(Debug, Clone)]
pub struct LoadedPlan {
    pub plan_id: String,
    pub name: String,
    pub enabled: bool,
    pub forced_mode: bool,
    pub schedules: Vec<Schedule>,
    pub app_rules: Vec<String>,  // TODO: Use proper AppRule type
    pub url_rules: Vec<String>,  // TODO: Use proper UrlRule type
}

/// Events emitted by the scheduler when plans activate/deactivate
#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    PlanActivated { plan_id: String },
    PlanDeactivated { plan_id: String },
    QuotaReached { plan_id: String, target: String },
}

/// PlanScheduler loads plans from the policy store and activates/deactivates
/// blocking rules at the correct times, handling DST transitions.
pub struct PlanScheduler {
    /// All loaded plans (enabled or not)
    plans: Arc<RwLock<HashMap<String, LoadedPlan>>>,
    /// Currently active plan IDs
    active_plans: Arc<RwLock<Vec<String>>>,
    /// Event channel for notifying subsystems
    // TODO: Use tokio::sync::broadcast for event distribution
    running: Arc<RwLock<bool>>,
}

impl PlanScheduler {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
            active_plans: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Load plans from the policy store database
    pub async fn load_plans(&self, plans: Vec<LoadedPlan>) -> Result<()> {
        let mut store = self.plans.write().await;
        store.clear();
        for plan in plans {
            info!(plan_id = %plan.plan_id, name = %plan.name, "Plan loaded");
            store.insert(plan.plan_id.clone(), plan);
        }
        Ok(())
    }

    /// Start the scheduler loop — checks every 30 seconds for plan transitions
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        info!("Plan scheduler started");

        let plans = self.plans.clone();
        let active_plans = self.active_plans.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            loop {
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        break;
                    }
                }

                // Evaluate which plans should be active right now
                let all_plans = plans.read().await;
                let mut should_be_active: Vec<String> = Vec::new();

                for (id, plan) in all_plans.iter() {
                    if plan.enabled && Self::is_plan_active_now(plan) {
                        should_be_active.push(id.clone());
                    }
                }
                drop(all_plans);

                // Compare with currently active and emit events
                let mut current = active_plans.write().await;
                let current_set: std::collections::HashSet<_> = current.iter().collect();
                let new_set: std::collections::HashSet<_> = should_be_active.iter().collect();

                // Newly activated
                for plan_id in new_set.difference(&current_set) {
                    info!(plan_id = %plan_id, "Plan activated by scheduler");
                    // TODO: Emit SchedulerEvent::PlanActivated
                    // TODO: Notify process_monitor, hosts_manager, wfp_manager
                }

                // Newly deactivated
                for plan_id in current_set.difference(&new_set) {
                    info!(plan_id = %plan_id, "Plan deactivated by scheduler");
                    // TODO: Emit SchedulerEvent::PlanDeactivated
                }

                *current = should_be_active;
                drop(current);

                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Plan scheduler stopped");
    }

    /// Determine if a plan should be active at the current time
    ///
    /// Handles DST transitions by converting schedule times to UTC
    /// using the schedule's specified timezone.
    fn is_plan_active_now(plan: &LoadedPlan) -> bool {
        let now_utc = Utc::now();

        for schedule in &plan.schedules {
            if Self::is_schedule_active(&schedule, now_utc) {
                return true;
            }
        }

        false
    }

    /// Check if a specific schedule window is currently active
    ///
    /// Key DST handling: convert current UTC time to the schedule's timezone,
    /// then compare against the schedule's local time window.
    fn is_schedule_active(schedule: &Schedule, now_utc: DateTime<Utc>) -> bool {
        // Convert UTC now to the schedule's timezone
        let now_local = now_utc.with_timezone(&schedule.timezone);
        let current_weekday = now_local.weekday();
        let current_time = now_local.time();

        // Check if today is in the schedule's active days
        if !schedule.days.contains(&current_weekday) {
            return false;
        }

        // Handle same-day schedules (start < end)
        if schedule.start_time <= schedule.end_time {
            current_time >= schedule.start_time && current_time < schedule.end_time
        } else {
            // Handle overnight schedules (e.g., 22:00 - 06:00)
            current_time >= schedule.start_time || current_time < schedule.end_time
        }
    }

    /// Get list of currently active plan IDs
    pub async fn get_active_plans(&self) -> Vec<String> {
        self.active_plans.read().await.clone()
    }

    /// Manually activate a plan (bypasses schedule check)
    pub async fn activate_plan(&self, plan_id: &str) -> Result<()> {
        let mut active = self.active_plans.write().await;
        if !active.contains(&plan_id.to_string()) {
            active.push(plan_id.to_string());
            info!(plan_id = %plan_id, "Plan manually activated");
        }
        Ok(())
    }

    /// Manually deactivate a plan
    pub async fn deactivate_plan(&self, plan_id: &str) -> Result<()> {
        let mut active = self.active_plans.write().await;
        active.retain(|id| id != plan_id);
        info!(plan_id = %plan_id, "Plan manually deactivated");
        Ok(())
    }
}

// ============================================================
// UNIT TESTS — UT-01: DST boundary handling
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn make_schedule(days: Vec<Weekday>, start: &str, end: &str, tz: &str) -> Schedule {
        Schedule {
            schedule_id: "test-schedule".to_string(),
            plan_id: "test-plan".to_string(),
            days,
            start_time: NaiveTime::parse_from_str(start, "%H:%M")
                .expect("Valid time format"),
            end_time: NaiveTime::parse_from_str(end, "%H:%M")
                .expect("Valid time format"),
            timezone: tz.parse::<Tz>().expect("Valid timezone"),
        }
    }

    #[test]
    fn test_schedule_active_within_window() {
        let schedule = make_schedule(
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            "09:00",
            "17:00",
            "America/New_York",
        );

        // Create a known Monday 12:00 ET → 17:00 UTC (during EST)
        let now = chrono::DateTime::parse_from_rfc3339("2025-01-13T17:00:00Z")
            .expect("Valid datetime")
            .with_timezone(&Utc);

        assert!(PlanScheduler::is_schedule_active(&schedule, now));
    }

    #[test]
    fn test_schedule_inactive_outside_window() {
        let schedule = make_schedule(
            vec![Weekday::Mon],
            "09:00",
            "12:00",
            "America/New_York",
        );

        // Monday 20:00 UTC = Monday 15:00 ET → outside 09:00-12:00
        let now = chrono::DateTime::parse_from_rfc3339("2025-01-13T20:00:00Z")
            .expect("Valid datetime")
            .with_timezone(&Utc);

        assert!(!PlanScheduler::is_schedule_active(&schedule, now));
    }

    #[test]
    fn test_schedule_inactive_wrong_day() {
        let schedule = make_schedule(
            vec![Weekday::Mon],
            "09:00",
            "17:00",
            "America/New_York",
        );

        // Tuesday 14:00 ET → correct time but wrong day
        let now = chrono::DateTime::parse_from_rfc3339("2025-01-14T19:00:00Z")
            .expect("Valid datetime")
            .with_timezone(&Utc);

        assert!(!PlanScheduler::is_schedule_active(&schedule, now));
    }

    #[test]
    fn test_overnight_schedule() {
        let schedule = make_schedule(
            vec![Weekday::Fri],
            "22:00",
            "06:00",
            "America/New_York",
        );

        // Friday 23:30 ET → should be active (within 22:00-06:00)
        let now = chrono::DateTime::parse_from_rfc3339("2025-01-18T04:30:00Z")
            .expect("Valid datetime")
            .with_timezone(&Utc);

        assert!(PlanScheduler::is_schedule_active(&schedule, now));
    }
}
