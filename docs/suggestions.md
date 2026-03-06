# FocusMe — Suggestions Log

All suggestions emitted during development sessions are tracked here.

---

[S-001] RISK — DoH bypass not mitigated in hosts_manager.rs
  What: Browsers with DoH enabled will bypass HOSTS-based URL blocking entirely.
  Why it matters: Affects Chrome, Firefox, Edge with default or custom DoH settings.
  Recommended action: Add WFP rule to block port 443 to known DoH provider IPs (1.1.1.1, 8.8.8.8, etc.) as a belt-and-suspenders measure.
  Build plan ref: Section 3.2, OQ-04.

[S-002] RISK — Apple ESF entitlement has 1–7 week approval lag
  What: macOS enforcement via Endpoint Security Framework requires a special entitlement from Apple.
  Why it matters: Without it, macOS blocking is limited to DNS-only (NEDNSProxyProvider). Process blocking is impossible.
  Recommended action: Apply immediately at developer.apple.com. Do not wait for Phase 1 coding to begin.
  Build plan ref: OQ-01, T-002.

[S-003] RISK — EV Code Signing certificate procurement takes 2–5 days
  What: Windows MSI signing and SmartScreen reputation requires an EV certificate.
  Why it matters: Without EV cert, Windows installs trigger SmartScreen warnings. Enterprise deployment via Intune/SCCM requires signed MSI.
  Recommended action: Begin procurement with DigiCert or Sectigo immediately during Phase 0.
  Build plan ref: T-003, Section 5.2.

[S-004] MISSING ARTIFACT — Legal documents (privacy policy, ToS, EULA)
  What: T-007 requires privacy_policy.md, tos.md, eula.md.
  Why it matters: GDPR compliance and app store submissions require these before distribution.
  Recommended action: Engage legal counsel or use template drafts for initial review.
  Build plan ref: T-007, Section 4.5.

[S-005] ✅ RESOLVED (Session 6) — OPTIMIZATION — Use tokio::sync::RwLock instead of Mutex for plan store
  What: The plan store is read far more than it is written. RwLock allows concurrent reads.
  Why it matters: Reduces contention during high-frequency process polling loop (T-013).
  Recommended action: Use Arc<RwLock<PlanStore>> instead of Arc<Mutex<PlanStore>>.
  Resolution: Migrated db.rs from std::sync::Mutex to tokio::sync::RwLock<Connection>.
    11 read methods use blocking_read(), 13 write methods use blocking_write().
    Decision D-013 logged.
  Build plan ref: T-013, T-020.

[S-006] DEPENDENCY ALERT — libbpf licensing (Apache-2.0 / LGPL-2.1)
  What: libbpf uses LGPL-2.1 which requires dynamic linking if the final binary is proprietary.
  Why it matters: Static linking of LGPL code into proprietary binary is a license violation.
  Recommended action: Use dynamic linking (dlopen) for libbpf or confirm LGPL-2.1 linking exception applies.
  Build plan ref: T-016, T-052.

[S-007] RISK — Android AccessibilityService Play Store scrutiny
  What: Google Play policy scrutinizes apps using AccessibilityService.
  Why it matters: Potential Play Store rejection if justification is insufficient.
  Recommended action: Prepare detailed justification for accessibility usage in Play Store declaration (OQ-07).
  Build plan ref: OQ-07, T-040.

[S-008] ✅ RESOLVED (Session 8) — MISSING ARTIFACT — Telemetry backend test instance
  What: T-008 requires a PostHog test instance with validated event schema.
  Why it matters: Analytics pipeline cannot be tested without a running instance.
  Recommended action: Stand up PostHog (self-hosted or cloud trial) and validate Appendix A event schema.
  Resolution: Created backend/src/analytics.rs with AnalyticsClient (fire-and-forget,
    no-op when POSTHOG_API_KEY absent) and 6 event types matching Appendix A:
    UserRegistered, UserLoggedIn, PlanSynced, FamilyCreated, FamilyMemberInvited,
    PlanShared. Wired into AppState. .env.example updated with POSTHOG_API_KEY.
  Build plan ref: T-008.

[S-009] ✅ RESOLVED (Session 4) — rmp-serde dependency present in Cargo.toml

[S-010] ✅ RESOLVED (Session 4) — Extension message type constants

[S-011] ✅ RESOLVED (Session 9) — VALIDATION — PostHog analytics event schema validation
  What: PostHog analytics wired into cloud backend with 6 event types. Validate
    that the Appendix A event schema matches the actual PostHog dashboard after
    first deployment.
  Why it matters: Incorrect event schemas cause silent data loss in PostHog.
  Recommended action: After first deployment, verify all 6 events appear in PostHog
    with correct properties. Ensure distinct_id is hashed user_id (not raw UUID)
    for privacy compliance per GDPR. Consider adding a SHA-256 hash wrapper around
    the user UUID before sending to PostHog.
  Resolution: analytics.rs distinct_id now SHA-256 hashed with `fm_` prefix via
    anonymize_id(). analytics_schema.rs validates all 6 event variants with correct
    event names, required properties, and hashed distinct_id format (67 chars).
    hex 0.4 + sha2 added to Cargo.toml.
  Build plan ref: T-008, Session 8 A5.

[S-012] LEGAL — PostHog Data Processing Agreement (GDPR Art. 28)
  What: PostHog processes EU personal data on behalf of FocusMe. GDPR Article 28
    requires a signed Data Processing Agreement (DPA) between FocusMe (controller)
    and PostHog (processor).
  Why it matters: Without a DPA, processing EU user data through PostHog is a GDPR
    violation regardless of whether distinct_id is hashed. Hashing reduces risk but
    does not eliminate the DPA requirement.
  Recommended action: (1) Sign PostHog's standard DPA (available in PostHog dashboard
    under Settings → Legal). (2) Confirm PostHog cloud region is EU if serving EU
    users. (3) Document DPA completion in this file.
  Build plan ref: T-008, Session 9 A1.
