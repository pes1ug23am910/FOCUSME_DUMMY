# FocusMe — Privacy Policy

> **FILE:** privacy_policy.md
> **TASK:** T-007 (template — requires legal counsel review)
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **STATUS:** `[LEGAL REVIEW REQUIRED]` — draft template for legal counsel to finalize
> **LAST UPDATED:** `[DATE]`

---

**`[LEGAL REVIEW REQUIRED]` — This document is a structured template. A qualified attorney must review and approve before publication. All sections marked `[LEGAL REVIEW REQUIRED]` need specific legal input.**

---

## FocusMe Privacy Policy

**Effective Date:** `[EFFECTIVE DATE — LEGAL REVIEW REQUIRED]`
**Last Updated:** `[LAST UPDATED DATE]`
**Company:** `[LEGAL ENTITY NAME — LEGAL REVIEW REQUIRED]`
**Contact:** `[PRIVACY EMAIL — LEGAL REVIEW REQUIRED]`

---

### 1. Introduction

FocusMe ("we," "us," "our") is a productivity application that helps users block distracting websites and apps on customizable schedules. This Privacy Policy explains what data we collect, how we use it, and your rights regarding your data.

This policy applies to all FocusMe products: the desktop application (Windows, macOS, Linux), the browser extension (Chrome, Firefox, Edge), and the mobile application (Android).

`[LEGAL REVIEW REQUIRED — Verify entity name, jurisdiction, and applicable laws]`

---

### 2. Data We Collect

FocusMe is designed with a **local-first architecture**. The vast majority of data never leaves your device.

#### 2.1 Data Stored Locally (on your device only)

| Data Type | Purpose | Storage Location | Retention |
|-----------|---------|-----------------|-----------|
| Blocking plans (domains, apps, schedules) | Core functionality — enforcing user-defined blocks | Encrypted SQLite database (SQLCipher AES-256) | Until user deletes |
| Usage statistics (per-app/per-site time counters) | Daily usage tracking for quotas and insights | Encrypted SQLite database | 90 days rolling, then aggregated |
| Session events (plan start/stop, forced mode enter/exit) | Usage insights and debugging | Encrypted SQLite database | 90 days rolling |
| User preferences (UI settings, language) | Application configuration | OS-native storage (SharedPreferences / NSUserDefaults / config files) | Until user deletes |
| Forced Mode state (remaining time, unlock hash) | Enforcing time-locked focus sessions | Encrypted SQLite database | Until session completes |

#### 2.2 Data Transmitted to FocusMe Servers

**By default, FocusMe transmits NO data to external servers.**

`[LEGAL REVIEW REQUIRED — If telemetry is added post-MVP (PostHog), update this section]`

If opt-in telemetry is enabled in a future version:

| Data Type | Purpose | When Sent | Retention |
|-----------|---------|-----------|-----------|
| Crash reports (stack trace, OS version, app version) | Debugging and stability improvement | On app crash (opt-in only) | 30 days |
| Anonymous usage counters (feature usage, not content) | Product improvement | Daily aggregate (opt-in only) | 1 year, then deleted |
| Device identifier (random UUID, not hardware ID) | Deduplication of telemetry events | With telemetry (opt-in only) | Deleted on opt-out or uninstall |

**Telemetry schema reference:** See Appendix A of the FocusMe Build Plan for the complete telemetry event schema.

**We NEVER collect:**
- ❌ Browsing history or URLs visited
- ❌ Website content, form data, or passwords
- ❌ App content or screen captures
- ❌ Location data
- ❌ Contact lists or communications
- ❌ Financial or payment information
- ❌ Biometric data
- ❌ Data from other apps (AccessibilityService reads package names only, not content)

#### 2.3 VPN Service (Android)

FocusMe's Android app uses a **local VPN service** for DNS-level website blocking. This VPN:
- Routes **DNS queries only** through a local filter (on-device, not a remote server)
- Does NOT route your internet traffic through any external server
- Does NOT inspect, log, or modify your web traffic
- Does NOT act as a traditional VPN — no remote tunnel is created

---

### 3. How We Use Data

| Use | Legal Basis (GDPR) | Details |
|-----|-------------------|---------|
| Enforce blocking plans | Legitimate interest / Contract performance | Core app functionality |
| Track usage statistics | Consent (user creates quotas) | Only data user explicitly asked to track |
| Debug crashes | Legitimate interest (opt-in only) | Improve app stability |
| Product improvement | Consent (opt-in telemetry only) | Aggregate, anonymous metrics |

`[LEGAL REVIEW REQUIRED — Verify legal basis for each jurisdiction]`

---

### 4. Data Sharing

**We do not sell, rent, or share your personal data with any third party.**

Exceptions:
- **Legal requirements:** We may disclose data if required by law, court order, or governmental regulation `[LEGAL REVIEW REQUIRED — specify jurisdictions]`
- **Service providers:** If telemetry is enabled, PostHog (EU-hosted) processes anonymous crash/usage data. PostHog's privacy policy: https://posthog.com/privacy

---

### 5. Data Retention

| Data Type | Retention Period | Deletion Method |
|-----------|-----------------|-----------------|
| Usage events | 90 days | Automatic rolling deletion |
| Usage counters (aggregate) | 1 year | Automatic |
| Crash reports (if opt-in) | 30 days | Automatic server-side |
| User preferences | Until deletion or uninstall | User action or app uninstall |
| Blocking plans | Until deletion or uninstall | User action |

---

### 6. Your Rights

#### 6.1 Rights Under GDPR (EU/EEA Users)

`[LEGAL REVIEW REQUIRED — Verify applicability and DPO requirements]`

You have the right to:

- **Access (Art. 15):** Request a copy of all data we hold about you
- **Rectification (Art. 16):** Correct inaccurate data
- **Erasure (Art. 17):** Request deletion of your data ("right to be forgotten")
- **Data Portability (Art. 20):** Receive your data in a portable format
- **Restriction (Art. 18):** Restrict processing of your data
- **Objection (Art. 21):** Object to processing based on legitimate interest
- **Withdraw Consent:** Withdraw consent for opt-in telemetry at any time

**How to exercise:** Settings → Privacy → Export Data / Delete All Data
**Contact:** `[DPO EMAIL — LEGAL REVIEW REQUIRED]`

#### 6.2 Rights Under CCPA (California Users)

`[LEGAL REVIEW REQUIRED — Verify CCPA applicability thresholds]`

- **Right to Know:** What personal information we collect and why
- **Right to Delete:** Request deletion of personal information
- **Right to Opt-Out:** We do not sell personal information
- **Non-Discrimination:** We will not discriminate against you for exercising your rights

#### 6.3 Rights Under COPPA (Children)

`[LEGAL REVIEW REQUIRED — COPPA compliance verification needed]`

FocusMe is not directed at children under 13. We do not knowingly collect personal information from children under 13. If you believe a child under 13 has provided us with personal information, please contact us.

---

### 7. Data Export

Users can export all their data at any time:

**Desktop app:** Settings → Privacy → Export Data → JSON file download
**Android app:** Settings → Privacy → Export Data → share via system share sheet
**Browser extension:** Data is synced via `chrome.storage.sync` — accessible via browser settings

The exported JSON includes:
- All blocking plans and schedules
- Usage statistics
- User preferences
- Session history

---

### 8. Data Security

- **Encryption at rest:** All sensitive data stored in SQLCipher (AES-256-CBC) encrypted database
- **Encryption in transit:** N/A for default configuration (no data leaves device). If telemetry is enabled: TLS 1.3
- **Access control:** Database key derived from OS keychain (macOS Keychain, Windows DPAPI, Linux Secret Service)
- **Code signing:** All distributed binaries are code-signed (EV cert on Windows, notarized on macOS, Play App Signing on Android)

`[LEGAL REVIEW REQUIRED — Verify security claims match implementation]`

---

### 9. Cookies

FocusMe does **not** use cookies. FocusMe is a native desktop/mobile application, not a web service.

The browser extension uses `chrome.storage.sync` for configuration persistence, which is not a cookie.

---

### 10. Changes to This Policy

We will notify users of material changes to this privacy policy through:
- In-app notification
- Updated "Last Updated" date at the top of this document
- `[LEGAL REVIEW REQUIRED — notification method requirements per jurisdiction]`

---

### 11. Contact

For privacy-related inquiries:

- **Email:** `[PRIVACY EMAIL — LEGAL REVIEW REQUIRED]`
- **Data Protection Officer:** `[DPO NAME AND CONTACT — LEGAL REVIEW REQUIRED if applicable]`
- **Mailing Address:** `[PHYSICAL ADDRESS — LEGAL REVIEW REQUIRED]`
- **Supervisory Authority:** `[APPLICABLE DPA — LEGAL REVIEW REQUIRED for EU users]`

---

*Template generated Session 5. All `[LEGAL REVIEW REQUIRED]` sections must be completed by qualified legal counsel before publication.*
