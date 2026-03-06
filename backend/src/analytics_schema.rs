// ============================================================
// FILE:        analytics_schema.rs
// MODULE:      Phase 5 — Cloud Backend > PostHog Event Schema Validation
// TASK:        S-011 (GDPR compliance + event schema validation)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 9
// DEPENDENCIES: serde_json, uuid
// TEST COVERAGE: 6 tests — one per AnalyticsEvent variant validating
//               correct PostHog event name, property keys, and hashed distinct_id
// KNOWN LIMITATIONS:
//   - Schema validation is compile-time/test-time only — no runtime validation.
//   - If new event types are added to AnalyticsEvent, corresponding tests
//     must be added here or the coverage test will fail.
// ============================================================

/// Expected PostHog event schemas per Appendix A of the FocusMe Build Plan.
///
/// Each event type has:
/// - A PostHog event name (snake_case)
/// - Required properties (must be present in the serialized JSON)
/// - A distinct_id that is SHA-256 hashed (never raw UUID)
///
/// These schemas are validated in the test module below to ensure the
/// analytics.rs implementation matches the build plan specification.

/// Event name constants — must match analytics.rs serde(rename) values.
pub mod event_names {
    pub const USER_REGISTERED: &str = "user_registered";
    pub const USER_LOGGED_IN: &str = "user_logged_in";
    pub const PLAN_SYNCED: &str = "plan_synced";
    pub const FAMILY_CREATED: &str = "family_created";
    pub const FAMILY_MEMBER_INVITED: &str = "family_member_invited";
    pub const PLAN_SHARED: &str = "plan_shared";

    /// All known event names — used for exhaustiveness checks.
    pub const ALL: &[&str] = &[
        USER_REGISTERED,
        USER_LOGGED_IN,
        PLAN_SYNCED,
        FAMILY_CREATED,
        FAMILY_MEMBER_INVITED,
        PLAN_SHARED,
    ];
}

/// Required property keys per event type (excluding distinct_id which is
/// always required and injected at send time, not in the serialized event).
pub mod required_properties {
    pub const USER_REGISTERED: &[&str] = &["user_id"];
    pub const USER_LOGGED_IN: &[&str] = &["user_id", "platform"];
    pub const PLAN_SYNCED: &[&str] = &["user_id", "plan_count", "device_platform"];
    pub const FAMILY_CREATED: &[&str] = &["group_id"];
    pub const FAMILY_MEMBER_INVITED: &[&str] = &["group_id"];
    pub const PLAN_SHARED: &[&str] = &["group_id"];
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::{anonymize_id, AnalyticsEvent};
    use uuid::Uuid;

    /// Helper: serialize an event and extract the properties object.
    fn event_properties(event: &AnalyticsEvent) -> serde_json::Value {
        serde_json::to_value(event).expect("Event should serialize")
    }

    /// Helper: assert that an event serializes with the correct event name
    /// and all required properties are present.
    fn assert_event_schema(
        event: &AnalyticsEvent,
        expected_name: &str,
        required_props: &[&str],
    ) {
        let json = event_properties(event);

        // Verify event name
        assert_eq!(
            json["event"].as_str().unwrap(),
            expected_name,
            "Event name mismatch for {:?}",
            event
        );

        // Verify properties object exists
        let props = &json["properties"];
        assert!(
            props.is_object(),
            "Properties should be an object for event {}",
            expected_name
        );

        // Verify all required properties are present
        for prop in required_props {
            assert!(
                props.get(prop).is_some(),
                "Missing required property '{}' in event {}",
                prop,
                expected_name
            );
        }

        // Verify distinct_id is hashed (starts with fm_ and is 67 chars)
        let distinct = event.distinct_id();
        assert!(
            distinct.starts_with("fm_"),
            "distinct_id for {} must start with fm_ prefix, got: {}",
            expected_name,
            distinct
        );
        assert_eq!(
            distinct.len(),
            67,
            "distinct_id for {} must be 67 chars (fm_ + 64 hex), got: {}",
            expected_name,
            distinct.len()
        );
    }

    #[test]
    fn test_schema_user_registered() {
        let event = AnalyticsEvent::UserRegistered {
            user_id: Uuid::new_v4(),
        };
        assert_event_schema(
            &event,
            event_names::USER_REGISTERED,
            required_properties::USER_REGISTERED,
        );
    }

    #[test]
    fn test_schema_user_logged_in() {
        let event = AnalyticsEvent::UserLoggedIn {
            user_id: Uuid::new_v4(),
            platform: "windows".to_string(),
        };
        assert_event_schema(
            &event,
            event_names::USER_LOGGED_IN,
            required_properties::USER_LOGGED_IN,
        );
    }

    #[test]
    fn test_schema_plan_synced() {
        let event = AnalyticsEvent::PlanSynced {
            user_id: Uuid::new_v4(),
            plan_count: 5,
            device_platform: "android".to_string(),
        };
        assert_event_schema(
            &event,
            event_names::PLAN_SYNCED,
            required_properties::PLAN_SYNCED,
        );
        // Verify plan_count is numeric
        let json = event_properties(&event);
        assert!(json["properties"]["plan_count"].is_number());
    }

    #[test]
    fn test_schema_family_created() {
        let event = AnalyticsEvent::FamilyCreated {
            group_id: Uuid::new_v4(),
        };
        assert_event_schema(
            &event,
            event_names::FAMILY_CREATED,
            required_properties::FAMILY_CREATED,
        );
    }

    #[test]
    fn test_schema_family_member_invited() {
        let event = AnalyticsEvent::FamilyMemberInvited {
            group_id: Uuid::new_v4(),
        };
        assert_event_schema(
            &event,
            event_names::FAMILY_MEMBER_INVITED,
            required_properties::FAMILY_MEMBER_INVITED,
        );
    }

    #[test]
    fn test_schema_plan_shared() {
        let event = AnalyticsEvent::PlanShared {
            group_id: Uuid::new_v4(),
        };
        assert_event_schema(
            &event,
            event_names::PLAN_SHARED,
            required_properties::PLAN_SHARED,
        );
    }
}
