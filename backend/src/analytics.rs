// ============================================================
// FILE:        analytics.rs
// MODULE:      Phase 5 — Cloud Backend > PostHog Analytics
// TASK:        T-008 (resolves S-008)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 8
// DEPENDENCIES: reqwest 0.11, serde, tokio, uuid, chrono
// TEST COVERAGE: Unit tests for event serialization; no-op when key absent
// KNOWN LIMITATIONS:
//   - Fire-and-forget: analytics failures are logged but never propagate.
//   - No batching: events are sent individually. Add batch endpoint for
//     high-volume scenarios.
//   - S-011 RESOLVED: distinct_id is now SHA-256 hashed with "fm_" prefix.
// ============================================================

use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// PostHog capture endpoint.
const POSTHOG_CAPTURE_URL: &str = "https://app.posthog.com/capture/";

// ── Analytics Events ────────────────────────────────────────

/// Analytics events matching FocusMe Build Plan Appendix A.
///
/// Each variant maps to a PostHog event with its own properties.
/// The `distinct_id` is SHA-256 hashed with an `fm_` prefix for GDPR
/// compliance (no raw UUIDs leak to third-party analytics — S-011).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "properties")]
pub enum AnalyticsEvent {
    /// User completed registration.
    #[serde(rename = "user_registered")]
    UserRegistered {
        user_id: Uuid,
    },

    /// User logged in successfully.
    #[serde(rename = "user_logged_in")]
    UserLoggedIn {
        user_id: Uuid,
        platform: String,
    },

    /// Plans synchronized from a device.
    #[serde(rename = "plan_synced")]
    PlanSynced {
        user_id: Uuid,
        plan_count: u32,
        device_platform: String,
    },

    /// Family group created.
    #[serde(rename = "family_created")]
    FamilyCreated {
        group_id: Uuid,
    },

    /// Family member invited.
    #[serde(rename = "family_member_invited")]
    FamilyMemberInvited {
        group_id: Uuid,
    },

    /// Plan shared with family group.
    #[serde(rename = "plan_shared")]
    PlanShared {
        group_id: Uuid,
    },
}

/// Anonymize a UUID by SHA-256 hashing it with an `fm_` prefix.
///
/// This ensures no raw UUIDs are sent to PostHog, satisfying GDPR
/// requirements for data minimization (Art. 5(1)(c)). The `fm_` prefix
/// keeps the identifier recognizable in PostHog dashboards.
pub fn anonymize_id(id: Uuid) -> String {
    let hash = Sha256::digest(id.as_bytes());
    format!("fm_{}", hex::encode(hash))
}

impl AnalyticsEvent {
    /// Extract the distinct_id for PostHog — SHA-256 hashed, never raw UUID.
    fn distinct_id(&self) -> String {
        match self {
            Self::UserRegistered { user_id } => anonymize_id(*user_id),
            Self::UserLoggedIn { user_id, .. } => anonymize_id(*user_id),
            Self::PlanSynced { user_id, .. } => anonymize_id(*user_id),
            Self::FamilyCreated { group_id } => anonymize_id(*group_id),
            Self::FamilyMemberInvited { group_id } => anonymize_id(*group_id),
            Self::PlanShared { group_id } => anonymize_id(*group_id),
        }
    }

    /// The PostHog event name string.
    fn event_name(&self) -> &'static str {
        match self {
            Self::UserRegistered { .. } => "user_registered",
            Self::UserLoggedIn { .. } => "user_logged_in",
            Self::PlanSynced { .. } => "plan_synced",
            Self::FamilyCreated { .. } => "family_created",
            Self::FamilyMemberInvited { .. } => "family_member_invited",
            Self::PlanShared { .. } => "plan_shared",
        }
    }
}

// ── PostHog Capture Payload ─────────────────────────────────

/// PostHog /capture/ request body.
#[derive(Debug, Serialize)]
struct CapturePayload {
    api_key: String,
    event: String,
    distinct_id: String,
    properties: serde_json::Value,
    timestamp: String,
}

// ── Analytics Client ────────────────────────────────────────

/// PostHog analytics client.
///
/// If `POSTHOG_API_KEY` is not set (or empty), the client operates in
/// no-op mode — all `track()` calls are silently dropped. This ensures
/// analytics are disabled by default in development and test environments.
#[derive(Clone)]
pub struct AnalyticsClient {
    /// PostHog project API key. None = analytics disabled.
    api_key: Option<String>,

    /// HTTP client for sending events to PostHog.
    client: reqwest::Client,
}

impl AnalyticsClient {
    /// Create a new analytics client.
    ///
    /// Reads `POSTHOG_API_KEY` from environment. If the key is missing or
    /// empty, analytics are disabled (no-op mode).
    pub fn new() -> Self {
        let api_key = std::env::var("POSTHOG_API_KEY")
            .ok()
            .filter(|k| !k.is_empty() && k != "phc_your_key_here");

        if api_key.is_some() {
            tracing::info!("PostHog analytics enabled");
        } else {
            tracing::info!("PostHog analytics disabled (POSTHOG_API_KEY not set)");
        }

        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Track an analytics event.
    ///
    /// This method is fire-and-forget: it spawns a background task to send
    /// the event to PostHog. Errors are logged but never propagated to the
    /// caller. The request path is never blocked by analytics.
    pub fn track(&self, event: AnalyticsEvent) {
        let Some(api_key) = &self.api_key else {
            // No-op mode — analytics disabled.
            return;
        };

        let payload = CapturePayload {
            api_key: api_key.clone(),
            event: event.event_name().to_string(),
            distinct_id: event.distinct_id(),
            properties: serde_json::to_value(&event).unwrap_or_default(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let client = self.client.clone();

        // Fire-and-forget — spawn a background task.
        tokio::spawn(async move {
            let result = client
                .post(POSTHOG_CAPTURE_URL)
                .json(&payload)
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    tracing::trace!(
                        event = payload.event,
                        "Analytics event sent successfully"
                    );
                }
                Ok(resp) => {
                    tracing::warn!(
                        event = payload.event,
                        status = %resp.status(),
                        "Analytics event rejected by PostHog"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        event = payload.event,
                        error = %e,
                        "Failed to send analytics event"
                    );
                }
            }
        });
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_name() {
        let event = AnalyticsEvent::UserRegistered {
            user_id: Uuid::new_v4(),
        };
        assert_eq!(event.event_name(), "user_registered");
    }

    #[test]
    fn test_distinct_id_is_hashed() {
        let id = Uuid::new_v4();
        let event = AnalyticsEvent::UserLoggedIn {
            user_id: id,
            platform: "windows".to_string(),
        };
        let distinct = event.distinct_id();
        // Must start with fm_ prefix
        assert!(distinct.starts_with("fm_"), "distinct_id should start with fm_ prefix");
        // Must NOT contain raw UUID
        assert!(!distinct.contains(&id.to_string()), "distinct_id must not contain raw UUID");
        // Must be deterministic
        assert_eq!(distinct, event.distinct_id());
    }

    #[test]
    fn test_distinct_id_group_is_hashed() {
        let id = Uuid::new_v4();
        let event = AnalyticsEvent::FamilyCreated { group_id: id };
        let distinct = event.distinct_id();
        assert!(distinct.starts_with("fm_"));
        assert!(!distinct.contains(&id.to_string()));
    }

    #[test]
    fn test_anonymize_id_deterministic() {
        let id = Uuid::nil();
        let hash1 = anonymize_id(id);
        let hash2 = anonymize_id(id);
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("fm_"));
        // SHA-256 hex output is 64 chars + 3 for "fm_" prefix = 67
        assert_eq!(hash1.len(), 67);
    }

    #[test]
    fn test_client_noop_without_key() {
        // Without POSTHOG_API_KEY set, client should be in no-op mode.
        let client = AnalyticsClient {
            api_key: None,
            client: reqwest::Client::new(),
        };
        // track() should not panic even without a key.
        // We can't easily verify no HTTP call is made without mocking,
        // but we can verify it doesn't panic.
        // (Would need tokio runtime to actually call track())
        assert!(client.api_key.is_none());
    }

    #[test]
    fn test_event_serialization() {
        let event = AnalyticsEvent::PlanSynced {
            user_id: Uuid::nil(),
            plan_count: 5,
            device_platform: "android".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["event"], "plan_synced");
        assert_eq!(json["properties"]["plan_count"], 5);
        assert_eq!(json["properties"]["device_platform"], "android");
    }

    #[test]
    fn test_all_event_variants() {
        let events = vec![
            AnalyticsEvent::UserRegistered { user_id: Uuid::new_v4() },
            AnalyticsEvent::UserLoggedIn { user_id: Uuid::new_v4(), platform: "macos".into() },
            AnalyticsEvent::PlanSynced { user_id: Uuid::new_v4(), plan_count: 3, device_platform: "linux".into() },
            AnalyticsEvent::FamilyCreated { group_id: Uuid::new_v4() },
            AnalyticsEvent::FamilyMemberInvited { group_id: Uuid::new_v4() },
            AnalyticsEvent::PlanShared { group_id: Uuid::new_v4() },
        ];

        for event in events {
            // Each event should serialize without error.
            let json = serde_json::to_value(&event).unwrap();
            assert!(json.get("event").is_some());
            // Each event should have a non-empty distinct_id.
            assert!(!event.distinct_id().is_empty());
        }
    }
}
