**FocusMe-Style App**

Developer-Actionable Build Plan

Windows · macOS · Linux · Android

**EXECUTIVE SUMMARY**

*Build a cross-platform, system-level productivity enforcer that blocks apps and URLs by schedule, quota, and lockdown mode --- resistant to casual circumvention --- with optional team analytics, a privacy-first telemetry architecture, and enterprise deployment packaging for Windows, macOS, Linux, and Android.*

**Table of Contents**

1\. MVP Feature Set 3

2\. Technical Architecture 4

3\. Anti-Circumvention & Enforcement Strategy 8

4\. Privacy & Telemetry Plan 10

5\. Security & Threat Model 12

6\. Testing & Verification Matrix 14

7\. Packaging & Deployment 18

8\. Operations & Maintenance 20

9\. Roadmap & Prioritized To-Do List 22

10\. Open Questions & Assumptions 28

A. Appendix A --- Telemetry Schema 30

B. Appendix B --- Installer Checklist 31

C. Appendix C --- Sample Policy JSON 32

**1. MVP Feature Set**

Features are tiered as Must-Have (M), Nice-to-Have (N), or Future (F). MVP = all M items.

**1.1 Must-Have --- Core Blocking Engine**

  ----------------------------------------------------------------------------------------------------------
  **ID**    **Feature**                                                   **Tier**   **Scope**
  --------- ------------------------------------------------------------- ---------- -----------------------
  APP-01    System-level app blocking by process name/path                M          All desktop platforms

  APP-02    Whitelist-only mode (block everything except approved apps)   M          Win, macOS, Linux

  WEB-01    Domain + wildcard URL blocking (HTTP & HTTPS)                 M          All platforms

  WEB-02    Subdomain and path-level URL matching                         M          All platforms

  WEB-03    Browser extension connector (Chrome, Firefox, Edge)           M          Via WebExtension API

  SCH-01    Named Focus Plans with time windows and recurrence            M          All platforms

  SCH-02    Daily/weekly quota per app or URL group                       M          All platforms

  SCH-03    Forced/lockdown mode (plan cannot be stopped mid-session)     M          Win, macOS

  PRO-01    Plan protection via password or challenge code                M          All platforms

  PRO-02    Task Manager / process-kill protection                        M          Windows

  PRO-03    Restart-persistent enforcement                                M          All platforms

  AND-01    Android app blocking + screen time quotas                     M          Android 8+

  AND-02    Android VPN-based DNS blocking for URLs                       M          Android 8+
  ----------------------------------------------------------------------------------------------------------

**1.2 Nice-to-Have --- Productivity Layer**

  ---------------------------------------------------------------------------------------------------------
  **ID**    **Feature**                                                   **Tier**   **Scope**
  --------- ------------------------------------------------------------- ---------- ----------------------
  POM-01    Built-in Pomodoro timer synced with Focus Plans               N          All

  STAT-01   Per-user usage stats dashboard (web + app time)               N          All

  STAT-02   Team/org analytics dashboard (business tier)                  N          Cloud/Web

  WEB-04    Block specific page elements (e.g., YouTube Shorts, feed)     N          Browser ext.

  SCH-04    Rationing rules (N launches per day, break enforcement)       N          All

  WIN-01    Window-title-aware blocking for alternative browser windows   N          Windows

  NOT-01    End-of-session desktop notifications and summaries            N          All

  AND-03    Android companion sync with desktop plans                     N          Android
  ---------------------------------------------------------------------------------------------------------

**1.3 Future --- Enterprise & Advanced**

  ----------------------------------------------------------------------------------------------------------
  **ID**    **Feature**                                                  **Tier**   **Scope**
  --------- ------------------------------------------------------------ ---------- ------------------------
  ENT-01    MDM policy integration (Intune, Jamf)                        F          Win, macOS

  ENT-02    Centralized policy server / admin console (web)              F          Cloud

  ENT-03    LDAP/SSO user authentication                                 F          Cloud

  ENT-04    FERPA/GDPR-compliant data export (CSV / REST API)            F          Cloud

  AI-01     AI-assisted schedule suggestions from usage history          F          All

  IOS-01    iOS Screen Time API integration (partial, no system-level)   F          iOS --- see note below
  ----------------------------------------------------------------------------------------------------------

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **WHY iOS IS OUT OF SCOPE:** iOS Out of Scope: Apple does not permit third-party apps to programmatically intercept process execution or block arbitrary apps outside the Screen Time API (introduced iOS 12, FamilyControls framework iOS 15). This API requires parental/managed approval flows, is subject to Apple entitlement review, and cannot achieve the same enforcement depth as Android Accessibility Service + VPN. iOS support is deferred to a future tier and limited to Screen Time API integration only.

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**2. Technical Architecture**

**2.1 High-Level Component Overview**

The system is composed of five logical layers. Each is independently deployable and communicates over well-defined interfaces.

  --------------------------------------------------------------------------------------------------------------------------------------
  **Layer**                               **Responsibility**                               **Suggested Stack**
  --------------------------------------- ------------------------------------------------ ---------------------------------------------
  Layer 1 --- Enforcement Engine          Per-platform native daemon/service               Rust (Windows/Linux), Swift (macOS)

  Layer 2 --- Policy Store                SQLite DB + JSON plan files, encrypted at rest   SQLite via rusqlite / GRDB

  Layer 3 --- UI Shell                    Cross-platform GUI management app                Tauri (Rust + WebView) or Electron fallback

  Layer 4 --- Browser Connector           WebExtension communicating with local daemon     TypeScript / WebExtension Manifest V3

  Layer 5 --- Analytics / Sync (opt-in)   Cloud backend for team reporting and plan sync   Node.js + PostgreSQL (self-hostable)
  --------------------------------------------------------------------------------------------------------------------------------------

**2.2 Platform-Specific Enforcement Components**

**Windows**

Enforcement requires elevated privileges. Two mutually complementary approaches:

-   Process blocking: Use Job Objects (AssignProcessToJobObject) to constrain child process creation, or implement a Windows Filtering Platform (WFP) callout driver for network-layer blocking. For user-mode enforcement, hook CreateProcess via IAT patching or use WMI Win32_ProcessStartTrace event subscription to detect and kill blocked processes immediately after launch.

-   URL/DNS blocking: Install a WFP callout or modify the HOSTS file at %SystemRoot%\\System32\\drivers\\etc\\hosts programmatically (requires admin). Prefer WFP for granularity; HOSTS file as fallback.

-   Kernel driver (optional, highest enforcement): A minifilter driver (IRP_MJ_CREATE intercept) can block file execution at the kernel level. Requires code signing via Microsoft\'s EV certificate + WHQL or HVCI-compatible signing. Cost/complexity is high --- recommend deferring to post-MVP. Assumption: MVP uses user-mode WFP + process monitoring.

-   Service persistence: Implement as a Windows Service (SvcHost-compatible). Register with Service Control Manager. Set recovery actions to restart on failure. Protect against stop via SetServiceSecurity and deny SERVICE_STOP to non-admin SIDs.

-   Task Manager protection: Use SetWindowsHookEx(WH_KEYBOARD_LL) to intercept Ctrl+Shift+Esc, or deny PROCESS_TERMINATE access on the service process handle via SetKernelObjectSecurity. NOTE: this is imperfect; kernel-level protection requires a driver.

-   Key APIs: CreateToolhelp32Snapshot / Process32Next (enumerate processes), OpenProcess + TerminateProcess (kill), WFP FwpmFilterAdd0, NtSetInformationProcess (process mitigation), SSDT hooking (driver only, avoid for HVCI systems).

**macOS**

-   Process blocking: Use Endpoint Security Framework (ESF, requires com.apple.developer.endpoint-security.client entitlement, System Extensions capability). ESF provides ES_EVENT_TYPE_AUTH_EXEC authorization callbacks --- return ES_AUTH_RESULT_DENY to block launch. This is the correct modern approach (replaces deprecated Kernel Extensions / kexts).

-   URL/DNS blocking: NetworkExtension framework --- implement a NEFilterDataProvider (Content Filter) or a Packet Tunnel Provider (DNS proxy). DNS proxy approach (NEDNSProxyProvider) intercepts DNS and returns NXDOMAIN for blocked domains. Requires user approval via System Settings \> Network Extensions.

-   System Extension signing: Must be signed with a Developer ID Application + System Extension entitlement. Notarization required (macOS 10.15+). Apple Silicon also requires Universal Binary (arm64 + x86_64 slices).

-   Service persistence: LaunchDaemon plist in /Library/LaunchDaemons/ (runs as root, persists across logins). LaunchAgent for per-user processes.

-   Forced-mode protection: Use SMAppService (macOS 13+) or legacy SMJobBless to install a privileged helper. Prevent uninstall by removing write permission on app bundle via chmod --- but admin can override.

-   Key frameworks: EndpointSecurity.framework, NetworkExtension.framework, ServiceManagement.framework, Security.framework (SecCode, code signing validation).

**Linux**

-   Process blocking: Two viable approaches: (a) eBPF with LSM hooks (Linux 5.7+ with CONFIG_BPF_LSM) using bpf_lsm_bprm_check_security to deny execve --- this is the cleanest modern approach. (b) Fanotify with FAN_OPEN_EXEC_PERM to receive exec permission requests and deny them. Fanotify requires CAP_SYS_ADMIN.

-   URL/DNS blocking: Modify /etc/hosts, deploy a local DNS resolver (dnsmasq or Unbound as a subprocess), or use nftables/iptables rules to redirect port 53 traffic to a local sinkhole. eBPF TC (Traffic Control) hooks can intercept packets at an even lower level.

-   Signed kernel modules: Ubuntu 20.04+ with Secure Boot enabled requires module signing. If an LKM is used (e.g., for kernel-level process blocking), it must be signed with a MOK (Machine Owner Key) enrolled via mokutil. eBPF programs via CO-RE (Compile Once, Run Everywhere) with libbpf avoid this requirement --- STRONGLY PREFERRED.

-   Service persistence: systemd unit file (.service) in /etc/systemd/system/. Set Restart=always, RestartSec=3.

-   Target distros (Assumption): Ubuntu 22.04 LTS, Pop!\_OS 22.04, Linux Mint 21 (Cinnamon). All use systemd, kernel 5.15+, and support eBPF LSM with proper kernel config.

-   Key libraries: libbpf, bpftool, libnetfilter_queue (fallback), Fanotify API (Linux 5.0+), D-Bus for IPC.

**Android**

-   App blocking: Use AccessibilityService to detect foreground app (getWindows(), ROOT_IN_ACTIVE_WINDOW) and overlay a blocking screen when a blocked app is detected. Also UsageStatsManager.queryUsageStats() for quota tracking. Requires PACKAGE_USAGE_STATS permission (granted via Settings, not at install time).

-   URL blocking: Implement a local VPN service (VpnService) that routes all DNS traffic through a local resolver; return NXDOMAIN for blocked domains. No root required. This is the same approach used by AdGuard, Blokada, etc.

-   Device Admin / Work Profile: For business deployment, use Device Policy Manager (DevicePolicyManager) with a Device Admin app, or enroll in Android Enterprise to use managed configurations and app allow/block lists at the device policy level.

-   Foreground Service: The blocking daemon must run as a foreground service with a persistent notification (Android 8+ requirement) to avoid being killed by the OS.

-   Distribution: Google Play (primary) + APK sideload (for enterprise). Target API level 34+ (Android 14). minSdk = 26 (Android 8.0).

**2.3 IPC & Data Flow**

All inter-process communication uses a Unix Domain Socket (Linux/macOS) or Named Pipe (Windows) exposed by the system daemon. Messages are serialized as MessagePack (compact, typed) or JSON (for debugging). The UI shell and browser extension connect as clients.

  --------------------------------------------------------------------------------------------------------------------------------------
  **Channel**                         **Transport**                                          **Data**
  ----------------------------------- ------------------------------------------------------ -------------------------------------------
  UI Shell → Daemon                   Named Pipe / UDS                                       Plan CRUD, status query, unlock challenge

  Browser Extension → Daemon          Native Messaging Host (chrome.runtime.connectNative)   URL check, block event, quota query

  Android App → Cloud Sync (opt-in)   HTTPS REST + JWT                                       Plan sync, usage upload

  Daemon → Cloud (opt-in)             HTTPS REST                                             Usage telemetry batch upload

  Admin Console → Cloud API           HTTPS REST + OAuth2                                    Team policy management, reporting
  --------------------------------------------------------------------------------------------------------------------------------------

**2.4 Policy Persistence**

-   Plans stored as JSON files in a platform-specific app data directory (e.g., %APPDATA%\\FocusMe on Windows, \~/Library/Application Support/FocusMe on macOS, \~/.local/share/focusme on Linux).

-   SQLite database for usage logs, quota counters, and session history. Database is encrypted using SQLCipher (key derived from machine ID + user salt). This prevents trivial data extraction but not admin-level attacks.

-   Policy files should be owned by the daemon process user (typically root/SYSTEM) and not writable by the end user. The UI shell communicates plan changes via IPC; the daemon validates and writes them.

-   Plan schema version field (semver) enables forward/backward compatibility during updates.

**2.5 Browser Extension Architecture**

-   Manifest V3 (Chrome/Edge/Opera/Brave/Vivaldi), Manifest V2 (Firefox --- MV3 support is partial as of 2024). Ship two extension packages or a unified codebase with build flags.

-   Background service worker (MV3) or background page (MV2) opens a native messaging port to the local daemon on browser startup.

-   declarativeNetRequest (MV3) or webRequest.onBeforeRequest (MV2) for URL blocking. MV3 dynamic rules (updateDynamicRules) can hold up to 5,000 rules per extension (Chrome limit). For large block lists, split into rule sets or use the redirect action to a local block page.

-   For page element blocking (e.g., YouTube Shorts): inject content scripts that use MutationObserver to remove or overlay specific DOM selectors. These are fragile (site changes break them) --- version separately and A/B test.

-   Extension communicates current tab URL and domain to the daemon via native messaging. Daemon responds with ALLOW / BLOCK / QUOTA_EXCEEDED. If BLOCK, extension either redirects to chrome://newtab or injects a full-screen overlay.

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **SAFARI NOTE:** Safari (macOS): Uses Safari App Extension or Web Extension (Safari 14+, requires Xcode). Packaged as part of the macOS app. Ships as a separate build step --- do not block initial release on Safari support.

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**3. Anti-Circumvention & Enforcement Strategy**

**3.1 What to Protect**

  ----------------------------------------------------------------------------------------------------------------------------
  **Asset**                  **Defense**                                      **Known Limitation**
  -------------------------- ------------------------------------------------ ------------------------------------------------
  Plan data & policy files   Daemon owns files, read-only for user accounts   Admin can edit as root/SYSTEM --- by design

  Daemon process             Service recovery, protected handle, watchdog     Admin can use Task Manager / kill signal

  Browser extension          Extension is force-installed via GPO/MDM         User can uninstall browser and use another

  Hosts file / WFP rules     Protected by daemon, re-applied on tamper        Admin can flush DNS / disable WFP

  Uninstall protection       Custom uninstaller requires unlock challenge     Admin can force-remove via Programs & Features

  Forced Mode timer          Stored encrypted, survives reboots               System clock manipulation by admin
  ----------------------------------------------------------------------------------------------------------------------------

**3.2 Platform-Specific Enforcement Techniques**

**Windows Anti-Circumvention**

-   Service hardening: Use ChangeServiceConfig2 with SERVICE_CONFIG_FAILURE_ACTIONS set to SC_ACTION_RESTART (3 restarts, then system reboot). Set service to Protected Service (MsSecExt) if signing allows --- requires Microsoft signature in practice; alternative is ELAM driver.

-   Process protection: Call SetProcessMitigationPolicy(ProcessSignaturePolicy) on the daemon to require Microsoft-signed children. Use Dynamic Code Prohibition to prevent code injection. These are Arbitrary Code Guard (ACG) policies.

-   HOSTS file tamper detection: Use ReadDirectoryChangesW on the drivers\\etc directory. On change, validate and re-write blocked entries. Race condition possible --- combine with WFP for belt-and-suspenders.

-   Task Manager / process kill: Use NtSetSystemInformation(SystemExtendedHandleInformation) to monitor for process handle opens with PROCESS_TERMINATE rights. Alternatively, use a kernel driver (post-MVP) with ObRegisterCallbacks to deny PROCESS_TERMINATE.

-   Registry persistence: Store daemon registration in HKLM\\SYSTEM\\CurrentControlSet\\Services (Service entry) --- not HKCU, which users can edit.

**macOS Anti-Circumvention**

-   System Integrity Protection (SIP): SIP protects /System, /usr, and certain kernel extensions. The daemon should install to /Library/Application Support/FocusMe (SIP-unprotected, but admin-only write). Educate deployers to enable SIP --- it\'s on by default.

-   Gatekeeper + Notarization: Always notarize. An un-notarized app will be quarantined and cannot launch on modern macOS without explicit user override.

-   ESF callback authorization: If the ES client process is killed, exec events are no longer intercepted. Use a LaunchDaemon that restarts the ESF daemon within 2 seconds. ESF subscription requires exclusive lock --- if subscription is revoked, log and alert.

-   Forced Mode clock attack: Store Forced Mode expiry as a monotonic timestamp (mach_absolute_time base) in addition to wall clock. Accept only forward-moving time.

**Linux Anti-Circumvention**

-   eBPF LSM: Loaded programs are attached to kernel security hooks and cannot be detached without CAP_SYS_ADMIN + BPF_PROG_DETACH syscall. Pin programs to /sys/fs/bpf/ to persist across daemon restarts.

-   Fanotify mark persistence: Re-apply marks after daemon restart. Use a watchdog cron or systemd timer to ensure daemon is running.

-   Namespace escape: A user might enter a new mount namespace to bypass HOSTS changes. eBPF LSM hooks operate on the host namespace and are not bypassable this way.

-   Sudo / su protection: The daemon should not trust sudo\'d processes --- check UID, not effective UID, for privilege escalation detection.

**Android Anti-Circumvention**

-   Safe Mode bypass: Accessibility Services are disabled in Safe Mode --- this is a known Android limitation. Mitigate by detecting Safe Mode boot (ActivityManager.isUserAMonkey() is insufficient; check SystemProperties) and logging it. For MDM deployments, disabling safe mode boot is possible on some devices via DPM.

-   Alternative apps: Use UsageStatsManager to track all running apps, not just foreground. Monitor for package installs and check against block list.

-   VPN disable: If user disables the VPN service, URL blocking stops. Use DevicePolicyManager.setAlwaysOnVpnPackage() for managed devices to force VPN-always-on.

-   ADB bypass: In enterprise, disable ADB via DPM. For personal use, ADB is an accepted limitation.

**3.3 Known Hard Limits (Document for Deployers)**

  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **HARD LIMITS:** The following scenarios bypass ALL software-level enforcement. Deployers in high-stakes environments (exam proctoring, enterprise DLP) MUST pair this software with MDM controls, BIOS passwords, Secure Boot with custom PK, and physical security policies: (1) Boot from external media (USB/SD). (2) Starting Windows in Safe Mode with Networking. (3) Admin account with direct disk access (dislocker, Linux live boot). (4) BIOS/UEFI settings reset to disable Secure Boot. (5) Reset/reinstall of the OS. (6) Android factory reset.

  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**4. Privacy & Telemetry Plan**

**4.1 Data Collection Philosophy**

Collect the minimum data needed to power the product. All analytics are opt-in. No data is sold or used for advertising. The personal edition collects zero data by default.

**4.2 Data Categories**

  ------------------------------------------------------------------------------------------------------------------------------------------------------
  **Data Type**          **What Is Collected**                                      **Storage**                          **Retention**
  ---------------------- ---------------------------------------------------------- ------------------------------------ -------------------------------
  Device Identifier      SHA-256(machine-id + install-salt)                         Local only by default                Until uninstall

  Session Events         Plan start/stop, app/URL block events (hashed domain)      Local SQLite + opt-in cloud          90 days default, configurable

  Usage Counters         Daily active minutes per app category (not per-URL)        Local + opt-in cloud                 1 year

  Crash Reports          Stack trace, OS version, app version, device ID (hashed)   Opt-in, Sentry-compatible endpoint   30 days

  Plan Configurations    Plan names, schedules (no browsing content)                Local only / cloud if sync enabled   Until deleted

  Team Analytics (Biz)   Per-user usage aggregates (pseudonymous)                   Cloud (tenant-scoped)                90 days or per policy
  ------------------------------------------------------------------------------------------------------------------------------------------------------

**4.3 Pseudonymization & PII Handling**

-   User identifiers in analytics are derived from HMAC-SHA256(device_uuid, tenant_secret). The tenant secret is rotatable, enabling erasure without deleting records.

-   Blocked domain names are hashed before leaving the device. The cloud backend stores only domain_hash, not the plaintext URL. Deployers who need raw URL logging for compliance must opt-in explicitly and accept additional GDPR/FERPA data processing terms.

-   No IP addresses are stored in analytics records. IP is used only for rate limiting at the API gateway and is not persisted.

**4.4 User Controls & Opt-In Flow**

1.  On first run, the app presents a data consent screen (cannot be skipped). Options: (a) Essential only --- no data leaves device; (b) Analytics --- usage stats sent to cloud (if cloud account linked); (c) Crash reports --- anonymized crash data to improve stability.

2.  Users can change consent at any time in Settings \> Privacy.

3.  Data export: Settings \> Privacy \> Export My Data generates a ZIP containing all local SQLite records in CSV format. Cloud data export available via user account portal (REST API: GET /v1/export?format=csv).

4.  Deletion: Uninstaller includes a \'Delete all data\' option. Cloud: DELETE /v1/account triggers 30-day soft delete followed by hard purge.

**4.5 GDPR / FERPA Compliance Notes**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **Obligation**                    **Mitigation**                                                                                                                          **Action Required**
  --------------------------------- --------------------------------------------------------------------------------------------------------------------------------------- -------------------------------------------------------------
  GDPR Art. 17 (Right to Erasure)   Soft-delete + hard purge within 30 days; HMAC rotation for pseudonymous records                                                         Validate with DPA in target EU country

  GDPR Art. 20 (Data Portability)   CSV + JSON export via Settings and REST API                                                                                             Include all processing records, not just raw events

  FERPA (US Education)              No student educational records stored; usage data is operational, not academic. Legal basis: legitimate interest or explicit consent.   Work with institution\'s FERPA officer before deployment

  COPPA (under 13)                  Do not knowingly collect data from users under 13 without verifiable parental consent. Age gate on cloud account creation.              Android app must not target children per Google Play policy
  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**5. Security & Threat Model**

**5.1 Threat Actors**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **Actor**                        **Goal**                                                       **Risk**            **Primary Vectors**
  -------------------------------- -------------------------------------------------------------- ------------------- -------------------------------------------------------
  Motivated End User               Self-circumvention to access blocked content during lockdown   Medium              Process kill, clock manipulation, safe mode

  Sibling / Peer (shared device)   Disable blocking for personal use                              Medium              HOSTS edit, uninstall attempt, alternate browser

  Malicious IT Insider             Exfiltrate usage logs, modify policies for surveillance        High (enterprise)   Direct DB access, config file modification

  Remote Attacker                  Exploit update mechanism to gain privileged code execution     High                MITM update, malicious policy injection via cloud API

  Supply Chain                     Compromise third-party dependency to inject malware            Medium              npm/cargo dependency poisoning
  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**5.2 Key Mitigations**

**Secure Update Strategy**

-   Auto-update uses a delta-update mechanism (e.g., Tauri updater or a custom binary diff). Update manifests are signed with Ed25519. The public key is embedded in the binary at build time and cannot be changed without a new signed binary.

-   Update endpoint: HTTPS only with certificate pinning (pin the CA + leaf). On macOS, use NSURLSession with pinning via CertificateTransparency + NSPinnedDomains. On Windows, use WinHTTP with INTERNET_OPTION_SECURITY_FLAGS.

-   Downloaded update packages are verified against SHA-256 checksum and Ed25519 signature before execution. Reject if either check fails.

-   Staged rollout: Phase 1 (5% of install base), Phase 2 (25%), Phase 3 (100%). Automatic rollback if error rate exceeds 2% in Phase 1. Use feature flags (LaunchDarkly or self-hosted Unleash) to gate.

**Code Signing**

  --------------------------------------------------------------------------------------------------------------------------------
  **Platform**        **Certificate/Key Type**                                  **Notes**
  ------------------- --------------------------------------------------------- --------------------------------------------------
  Windows             EV Code Signing cert (DigiCert or Sectigo)                Required for SmartScreen; SHA-256 + timestamp

  macOS               Apple Developer ID + Notarization                         Notarize every build; staple ticket to DMG/PKG

  Linux               GPG-signed .deb/.rpm packages; apt repo signed with GPG   Publish public key on HTTPS key server

  Android             Android keystore (RSA-4096 or EC P-521)                   Store keystore in HSM or Google Play App Signing

  Browser Extension   Chrome Web Store / Firefox AMO signing (mandatory)        CWS signs automatically; AMO requires review
  --------------------------------------------------------------------------------------------------------------------------------

**IPC & API Security**

-   Named Pipe / UDS: Set permissions so only the daemon user and the logged-in user can connect. Validate all incoming JSON/MessagePack with a schema before processing.

-   Cloud API: JWT (RS256) with short TTL (15 minutes) + refresh token. Refresh token stored in OS keychain (macOS Keychain, Windows DPAPI-protected Credential Manager, Linux libsecret).

-   Rate limiting on all API endpoints. Enforce at API gateway (e.g., nginx + Lua or Kong). Brute-force protection on plan unlock challenge endpoint.

-   Input validation: All policy values (domain strings, time ranges, quota integers) are validated against strict schemas before persistence. Reject inputs exceeding defined bounds (e.g., domain length \> 253 chars, quota \> 86400 seconds/day).

**Dependency Security**

-   Lock file pinning: package-lock.json (npm), Cargo.lock (Rust), go.sum (Go if used). Never run npm install without a lockfile in CI.

-   Automated vulnerability scanning: Dependabot (GitHub) or Renovate + cargo-audit + npm audit in CI pipeline. Block merges on CRITICAL CVEs.

-   SBOM (Software Bill of Materials): Generate CycloneDX SBOM on every release. Publish alongside installer for enterprise procurement.

**6. Testing & Verification Matrix**

**6.1 Unit Tests**

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Description**                                                            **Success Criterion**
  -------- -------------------------------------------------------------------------- ----------------------------------------------------------------------------------
  UT-01    Plan scheduler --- correct time window calculation across DST boundaries   Returns correct activation/deactivation instants for all TZ offsets

  UT-02    URL rule matching --- domain, wildcard, path, subdomain                    All 12 pattern types match and reject correctly per spec table

  UT-03    Quota counter --- decrement, reset at midnight, carry-over disabled        Counter reaches zero at correct wall-clock time; resets at 00:00 local

  UT-04    Policy JSON schema validation --- accept valid, reject malformed           100% of malformed schemas rejected; no panic/crash on malformed input

  UT-05    Forced Mode timer --- monotonic clock advance only                         Rollback of system clock by 60min does not reduce timer; time still expires

  UT-06    Ed25519 signature verification --- valid, tampered, expired manifest       Tampered signature rejected; valid signature accepted; expired manifest rejected

  UT-07    SQLite quota ledger --- concurrent write safety                            No data corruption under 100 concurrent inserts (use WAL mode)
  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------

**6.2 Integration Tests**

  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Description**                                                                               **Success Criterion**
  -------- --------------------------------------------------------------------------------------------- -------------------------------------------------------------------------------------
  IT-01    Windows Service starts, applies HOSTS entries, process blocked within 2s of plan activation   Blocked process terminates within 2 seconds of plan start

  IT-02    macOS ESF exec callback blocks target app; ESF client restart re-applies block within 5s      Target app launch returns \'Operation not permitted\'; re-block after restart \< 5s

  IT-03    Linux eBPF LSM hook blocks execve for blocked binary path                                     execve returns EPERM for blocked path; unblocked path succeeds

  IT-04    Browser extension native messaging handshake with daemon succeeds on Chrome/Firefox/Edge      Extension connects within 3s of browser start; PING/PONG round-trip \< 100ms

  IT-05    Android VPN service intercepts DNS; NXDOMAIN returned for blocked domain                      curl to blocked domain fails with NXDOMAIN; allowed domain resolves normally

  IT-06    Plan sync round-trip: create plan on desktop → verify it appears in Android companion         Plan visible on Android within 30s of creation on desktop (with cloud sync enabled)

  IT-07    Update pipeline: daemon downloads, verifies, and applies mock update package                  Update applied successfully; rollback triggered on injected hash mismatch
  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**6.3 Anti-Circumvention / Bypass Tests**

  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Test Scenario**               **Method**                                                                                     **Success Criterion**
  -------- ------------------------------- ---------------------------------------------------------------------------------------------- -------------------------------------------------------------------------------------------------------
  BT-01    Windows Safe Mode               Boot Windows 11 in Safe Mode; check if blocking service starts and HOSTS entries are present   Service does NOT run in Safe Mode --- document as known limitation; verify HOSTS entries persist

  BT-02    Process kill via Task Manager   Attempt to kill daemon via Task Manager and Process Explorer (non-admin user)                  Non-admin user receives \'Access Denied\'; process remains running

  BT-03    HOSTS file edit                 Non-admin user attempts to edit %SystemRoot%\\System32\\drivers\\etc\\hosts                    Edit fails; daemon detects and restores within 5s if admin edit occurs

  BT-04    Alternate browser               Install an unlisted browser (e.g., Waterfox); attempt to navigate to blocked domain            WFP/DNS blocking applies regardless of browser; domain is blocked at network layer

  BT-05    DNS-over-HTTPS bypass           Enable DoH in browser settings; navigate to blocked domain                                     WFP rule or HOSTS entry still blocks IP after DNS resolution; document DoH bypass risk

  BT-06    VPN bypass                      User installs a third-party VPN; routes traffic through VPN; navigates to blocked domain       Blocked at WFP/iptables before VPN encapsulation; or document as limitation requiring firewall policy

  BT-07    macOS Recovery Mode             Boot to macOS Recovery; attempt to modify LaunchDaemon plist                                   Recovery Mode access is a known limitation --- document; recommend FileVault + firmware password

  BT-08    Android Safe Mode               Boot Android in Safe Mode; check if blocking VPN and AccessibilityService are active           Accessibility Service disabled --- document; VPN may remain active (test device-specific)

  BT-09    Process injection (Windows)     Attempt DLL injection into daemon process via CreateRemoteThread (non-admin)                   Injection fails due to ACG/no-remote-code policy; daemon integrity uncompromised

  BT-10    Clock rollback (Forced Mode)    Set system clock back 2 hours during an active Forced Mode session                             Forced Mode timer continues from monotonic clock; session does not end prematurely

  BT-11    Boot from USB (all platforms)   Boot from Linux Live USB; mount system drive; edit config files                                Out-of-scope bypass --- document; require BIOS password + Secure Boot for mitigation

  BT-12    Alternate admin account         Create a second admin account; attempt to uninstall FocusMe                                    Uninstaller requires challenge code even for admins; note admin can still force-remove
  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**6.4 Cross-Platform Compatibility Matrix**

  -------------------------------------------------------------------------------------------------------------------
  **OS / Version**         **Arch**                **Support Level**   **Key Test Focus**
  ------------------------ ----------------------- ------------------- ----------------------------------------------
  Windows 10 (22H2)        x64                     Full                Test blocking, UI, updater, installer

  Windows 11 (23H2)        x64                     Full                Primary target; test HVCI compat

  Windows 11 ARM64         ARM64                   Best-effort         ARM64 native build; verify WFP behavior

  macOS 13 Ventura         Apple Silicon (M1/M2)   Full                ESF + NetworkExtension

  macOS 14 Sonoma          Intel + Apple Silicon   Full                Primary macOS target

  macOS 12 Monterey        Intel                   Best-effort         Verify ESF availability

  Ubuntu 22.04 LTS         x64                     Full                eBPF LSM, systemd, deb package

  Pop!\_OS 22.04           x64                     Full                Same kernel as Ubuntu 22.04

  Linux Mint 21 Cinnamon   x64                     Best-effort         Desktop-specific UX testing

  Android 10--14           ARM64                   Full                AccessibilityService + VPN on each API level
  -------------------------------------------------------------------------------------------------------------------

**6.5 Performance Benchmarks**

  ----------------------------------------------------------------------------------------------------------------------------------------------------------
  **Metric**                                     **Target**                                              **How to Measure**
  ---------------------------------------------- ------------------------------------------------------- ---------------------------------------------------
  App launch latency (ESF/eBPF hook overhead)    \< 50ms added latency on allowed apps                   Measure with hyperfine; average of 100 launches

  URL blocking decision latency (browser ext.)   \< 20ms from navigation start to block/allow decision   Puppeteer timing trace on 500 navigation events

  Daemon CPU at idle                             \< 0.5% CPU on modern hardware                          top/Activity Monitor for 1 hour steady state

  Daemon memory footprint                        \< 30MB RSS                                             Monitor over 24hr with 10 plans active

  SQLite write throughput                        \> 1,000 events/sec without WAL stall                   fio + custom write bench at 1k events/sec for 60s
  ----------------------------------------------------------------------------------------------------------------------------------------------------------

**7. Packaging & Deployment**

**7.1 Windows Packaging**

-   Installer: NSIS or WiX Toolset v4 (preferred for enterprise MSI). WiX generates a standards-compliant MSI consumable by Intune/SCCM/Group Policy.

-   MSI features: Silent install (/quiet), transform files (.mst) for enterprise configuration, custom actions for driver/service installation, rollback support on failure.

-   Code signing: Sign with EV cert using signtool.exe. Timestamp with RFC 3161 timestamping (ensures validity after cert expiry). Command: signtool sign /tr http://timestamp.digicert.com /td SHA256 /fd SHA256 /a installer.msi

-   MSIX/AppX: Produce an MSIX package for Microsoft Store distribution (optional). MSIX cannot install kernel-mode components --- this limits enforcement depth. Use MSIX for personal edition; MSI for business.

-   Auto-update: Implement Squirrel.Windows-compatible update mechanism or a custom updater that downloads a signed delta patch, verifies signature, and applies via Windows Installer patch (MSP).

**7.2 macOS Packaging**

-   Package types: DMG (drag-to-Applications, user install) and PKG (macOS Installer, supports privileged helper installation). Business deployments use PKG.

-   PKG structure: Main app bundle in /Applications/FocusMe.app; System Extension in the bundle; LaunchDaemon plist installed to /Library/LaunchDaemons/com.focusme.daemon.plist via a postinstall script; SMJobBless helper if needed.

-   Notarization pipeline: xcrun altool (legacy) or xcrun notarytool (modern). Submit PKG → poll for status → staple ticket → distribute. Automate in CI with: xcrun notarytool submit FocusMe.pkg \--apple-id \$APPLE_ID \--password \$APP_PASSWORD \--team-id \$TEAM_ID \--wait

-   Universal Binary: Build with -arch arm64 -arch x86_64 in Xcode or with cargo build \--target universal-apple-darwin using lipo.

-   Auto-update: Sparkle 2 (open source, Ed25519 support). Appcast XML hosted on HTTPS endpoint. Sparkle validates signature before applying.

**7.3 Linux Packaging**

-   .deb (Debian/Ubuntu/Mint) and .rpm (RHEL/Fedora --- optional). Use fpm (Effing Package Management) or native packaging tools to produce both.

-   postinst script: Installs systemd unit, enables and starts daemon, loads eBPF programs, prompts for MOK enrollment if Secure Boot is detected.

-   Signed APT repository: Host on GitHub Releases or an S3 bucket. Sign repo metadata with GPG. Publish public key for users to add with apt-key (deprecated) or by placing in /etc/apt/trusted.gpg.d/.

-   AppImage (optional): Self-contained portable binary for distros not on .deb/.rpm. Does not install daemon --- limited to user-level blocking only. Clearly label this limitation.

-   Flatpak / Snap: Out of scope for initial release --- sandboxing constraints prevent system-level enforcement.

**7.4 Android Distribution**

-   Primary: Google Play Store. Package as AAB (Android App Bundle). Target API 34, minSdk 26.

-   Enterprise: APK sideload + managed Google Play (Android Enterprise). Publish to private Play track. MDM enrollment auto-installs and configures the app via managed configurations.

-   Required permissions: BIND_ACCESSIBILITY_SERVICE, BIND_VPN_SERVICE, PACKAGE_USAGE_STATS (must be granted via Settings on Android 10+), RECEIVE_BOOT_COMPLETED, FOREGROUND_SERVICE.

**7.5 CI/CD Pipeline**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **Stage**        **Tool / Service**                                                                                             **Notes**
  ---------------- -------------------------------------------------------------------------------------------------------------- -------------------------------------------------------------
  Source control   GitHub (private repo) + branch protection rules                                                                Require PR review + CI pass before merge

  Build system     GitHub Actions (matrix: win/mac/linux/android)                                                                 Self-hosted macOS runner for Xcode builds

  Testing          cargo test + jest (extension) + pytest (packaging)                                                             Required on all PRs; block merge on failure

  Signing          Secrets stored in GitHub Actions Secrets (EV cert, Apple creds, GPG key)                                       Use Hardware Security Module for prod signing (YubiKey HSM)

  Release          Tag-based release trigger. Auto-generate SBOM, sign artifacts, publish to release page and update endpoints.   Staged rollout via feature flag service
  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**8. Operations & Maintenance**

**8.1 Logging**

-   Structured logging (JSON) using tracing (Rust) or os_log (macOS). Log levels: ERROR, WARN, INFO, DEBUG, TRACE. Default level: INFO in production.

-   Log rotation: Keep last 7 days of logs, max 50MB/file. Rotate via logrotate (Linux), os_log (macOS), or EventLog (Windows).

-   Sensitive data in logs: NEVER log full URLs, plan names, or domain names in plaintext. Log domain_hash and event_type only.

-   Log location: Platform-specific: Windows --- %PROGRAMDATA%\\FocusMe\\logs; macOS --- /Library/Logs/FocusMe; Linux --- /var/log/focusme.

**8.2 Crash Reporting**

-   Integrate with Sentry (self-hosted or cloud) using the platform SDK. On crash: capture minidump (Windows), crash report (macOS), core dump hash (Linux), ANR trace (Android).

-   Opt-in only. PII scrubbing: strip file paths, usernames, and environment variables from crash reports before upload.

-   Deduplicate by stack trace fingerprint. Alert on-call engineer when new crash type appears with frequency \> 10/hour.

**8.3 Support UX for Locked-Out Users**

Forced Mode creates a UX risk: users may become stuck if they forget a password or if a plan runs longer than intended. Mitigation plan:

5.  Emergency unlock code: During plan creation, generate a one-time emergency code (TOTP-based, 8-digit) and prompt user to save it offline. Display in plan setup wizard only --- not re-viewable.

6.  Time-based auto-expiry: Forced Mode has a maximum duration cap (configurable, default 24 hours) to prevent permanently locked devices.

7.  Support unlock: A signed unlock token can be generated by a support team member using a support keypair (private key held by vendor). User submits a device ID + request; support issues a time-limited signed token.

8.  MDM override: Enterprise deployments can push a policy update via MDM that disables Forced Mode for a specific device --- document this in the enterprise guide.

**8.4 Rollout & Maintenance Cadence**

  ------------------------------------------------------------------------------------------------------------------------
  **Release Type**        **Scope**                                                   **Target Cadence**
  ----------------------- ----------------------------------------------------------- ------------------------------------
  Hotfix                  Critical security vulnerabilities or crash-loop bugs        48 hours from discovery to release

  Patch release (x.y.Z)   Bug fixes, minor improvements, updated block lists          Every 2 weeks

  Minor release (x.Y.0)   New features, platform support, major dependency upgrades   Every 6--8 weeks

  Major release (X.0.0)   Architecture changes, breaking API changes                  Annually or as needed
  ------------------------------------------------------------------------------------------------------------------------

**9. Roadmap & Prioritized To-Do List**

Tasks are organized into phases. Each task has an owner role, required inputs, and a concrete output artifact. No time estimates are given --- prioritize by phase order.

**Phase 0 --- Foundations (Pre-coding)**

  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                  **Owner**   **Output / Artifact**
  -------- ---------------------------------------------------------------------------------------------------------------------------------------------- ----------- ----------------------------------------------------------------
  T-001    Validate eBPF LSM availability on target Linux kernels. Boot Ubuntu 22.04 + Pop!\_OS; run \'grep CONFIG_BPF_LSM /boot/config-\$(uname -r)\'.   Eng         Decision doc: eBPF LSM vs Fanotify fallback per distro

  T-002    Obtain Apple Endpoint Security Framework entitlement. Apply via Apple Developer portal; confirm approval timeline (typically 1--3 weeks).      Eng/PM      Approved provisioning profile with ESF entitlement

  T-003    Procure EV Code Signing certificate (Windows). Select DigiCert or Sectigo; complete identity verification (2--5 days).                         PM          EV cert + private key stored in HSM or secure vault

  T-004    Set up GitHub org, repos (monorepo recommended), branch protection, secrets vault (GitHub Secrets + 1Password for dev team).                   Eng         Repo structure, CI skeleton (empty workflows), CODEOWNERS file

  T-005    Define policy JSON schema v1.0 (see Appendix C). Validate with JSON Schema draft-2020-12.                                                      Eng         policy_schema_v1.json + validation test suite

  T-006    Define IPC protocol spec. Document MessagePack message types, versioning strategy, error codes.                                                Eng         ipc_protocol_v1.md with full type definitions

  T-007    Legal review: draft privacy policy, ToS, EULA. Confirm GDPR lawful basis (legitimate interest vs. consent) for analytics.                      PM/Legal    privacy_policy.md, tos.md, eula.md

  T-008    Select analytics/telemetry backend: PostHog (self-hostable, OSS) recommended. Stand up a test instance and validate event ingestion.           Eng         PostHog test instance URL + event schema validated
  ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Phase 1 --- Core Daemon & Enforcement (Desktop MVP)**

  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                                                     **Owner**   **Output / Artifact**
  -------- --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- ----------- -------------------------------------------------------------
  T-010    Implement Windows daemon as a Rust Windows Service using the windows-service crate. Daemon registers, starts, and responds to SCM control codes. Recovers on crash.               Eng         focusme-daemon.exe + service install script + unit tests

  T-011    Implement HOSTS file URL blocking on Windows. Daemon writes/removes entries. FileSystemWatcher detects tampering; restores within 2s.                                             Eng         hosts_manager.rs + tamper detection test (BT-03)

  T-012    Implement WFP-based URL blocking on Windows. Use FwpmFilterAdd0 to redirect blocked IPs to 0.0.0.0. Falls back to HOSTS if WFP callout fails.                                     Eng         wfp_manager.rs + integration test IT-01 pass

  T-013    Implement process enumeration + kill on Windows. Poll CreateToolhelp32Snapshot every 500ms; kill if process name/path matches active plan blocklist.                              Eng         process_monitor.rs + test: blocked process killed within 2s

  T-014    Implement macOS ESF daemon in Swift. Subscribe to ES_EVENT_TYPE_AUTH_EXEC; return DENY for blocked paths. LaunchDaemon plist for persistence.                                     Eng         FocusMeESF.swift + LaunchDaemon plist + test IT-02 pass

  T-015    Implement macOS NEDNSProxyProvider for URL blocking. Return synthesized NXDOMAIN for blocked domains. User must approve Network Extension in System Settings.                     Eng         DNSProxyProvider.swift + test: blocked domain NXDOMAIN

  T-016    Implement Linux eBPF LSM program for exec blocking. Use libbpf + CO-RE; pin to /sys/fs/bpf/focusme_exec_block. Fanotify fallback if LSM unavailable.                              Eng         focusme_lsm.bpf.c + loader.rs + test IT-03 pass

  T-017    Implement Linux DNS blocking via local Unbound instance. Daemon writes Unbound rpz (Response Policy Zone) rules; sets /etc/resolv.conf to 127.0.0.1.                              Eng         dns_blocker.rs + test: blocked domain NXDOMAIN on Ubuntu

  T-018    Implement SQLite policy store (rusqlite + SQLCipher). Schema: plans, rules, quota_ledger, sessions, events. WAL mode, migrations via refinery crate.                              Eng         db_schema.sql + migration files + unit tests UT-07

  T-019    Implement IPC server (Unix Domain Socket + Named Pipe). Accept connections from UI shell and browser extension native messaging host. Validate all messages against IPC schema.   Eng         ipc_server.rs + protocol conformance tests

  T-020    Implement plan scheduler. Load plans from policy store; activate/deactivate blocking rules at correct times; handle DST; emit activation events to IPC clients.                   Eng         scheduler.rs + unit tests UT-01 to UT-03

  T-021    Implement Forced Mode timer. Store expiry as both wall clock (ISO-8601) and monotonic offset. Reject time rollback. Emit FORCED_MODE_ACTIVE event.                                Eng         forced_mode.rs + unit test UT-05

  T-022    Implement plan protection: password hash (Argon2id, min cost params) + random character challenge. Store hash in policy store (daemon-owned DB only).                             Eng         plan_protection.rs + security review checklist
  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Phase 2 --- Browser Extension & UI Shell**

  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                                                               **Owner**   **Output / Artifact**
  -------- ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- ----------- ------------------------------------------------------------
  T-030    Scaffold browser extension (TypeScript, Webpack, MV3/MV2 dual). Background service worker opens native messaging port; implements PING/PONG with daemon.                                    Eng         extension/ directory + manifest.json (MV3 + MV2 variants)

  T-031    Implement URL check in extension. On chrome.webNavigation.onBeforeNavigate, query daemon via native messaging. On BLOCK response: redirect to chrome-extension://\...block.html.            Eng         background.ts + test IT-04 pass on Chrome, Firefox, Edge

  T-032    Implement declarativeNetRequest rules for extension. Convert active plan\'s domain block list to DNR rules. Update dynamically when plans change via IPC event.                             Eng         rule_converter.ts + test: 500 rules applied \< 200ms

  T-033    Implement content script for page element blocking (YouTube Shorts selector). MutationObserver removes matching elements. Version independently; use feature flag.                          Eng         content_scripts/element_blocker.ts + selector config JSON

  T-034    Build native messaging host binary (Rust). Reads stdin framing (4-byte length prefix), forwards to daemon UDS/Named Pipe, writes response. Sign and install to correct path per platform.   Eng         native_messaging_host.exe/.app/bin + install script

  T-035    Build UI shell with Tauri (Rust backend, WebView frontend in React/TypeScript). Features: plan list, create/edit plan wizard, active session display, settings, stats dashboard.            Eng         tauri_app/ directory + basic plan CRUD E2E test

  T-036    Implement plan wizard: name, schedules (day/time picker), app rules (path picker), URL rules (domain input + wildcard preview), quota settings, Forced Mode toggle, protection settings.    Eng         PlanWizard.tsx + jest component tests for each step

  T-037    Implement usage stats display. Read from SQLite via daemon IPC. Display per-app and per-domain-category bar charts (Recharts or Chart.js in WebView).                                       Eng         StatsPage.tsx + mock data display test

  T-038    Accessibility and i18n foundations. Define string catalog; implement English baseline. RTL layout not required for MVP. WCAG 2.1 AA for UI shell.                                           Eng         strings_en.json + i18n utility wrapper + a11y audit report
  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Phase 3 --- Android App**

  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                                                            **Owner**   **Output / Artifact**
  -------- ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- ----------- -------------------------------------------------------------
  T-040    Scaffold Android app (Kotlin, Jetpack Compose). Minimum SDK 26, target 34. Implement onboarding flow requesting AccessibilityService and PACKAGE_USAGE_STATS permissions.                Eng         Android project skeleton + permission request flow

  T-041    Implement AccessibilityService for foreground app detection. On window state change event, check foreground package against active plan blocklist. Overlay blocking screen if matched.   Eng         FocusMeAccessibilityService.kt + test: blocked app overlaid

  T-042    Implement local VPN service for DNS blocking. Use VpnService, TunInterface, and a local DNS resolver (dnsjava or custom). Return NXDOMAIN for blocked domains.                           Eng         FocusMeVpnService.kt + test IT-05 pass

  T-043    Implement UsageStatsManager quota tracking. Track foreground time per app; enforce daily quota; post notification at 80% and 100% of quota.                                              Eng         QuotaTracker.kt + test: quota enforced after X minutes

  T-044    Implement Foreground Service for daemon persistence. Show persistent notification (required Android 8+). Handle BOOT_COMPLETED broadcast to restart after reboot.                        Eng         FocusMeDaemonService.kt + boot-persistence test

  T-045    Implement Android plan UI (Jetpack Compose). Plan list, create/edit screens, quota settings, schedule picker. Sync with cloud if account linked.                                         Eng         Android UI screens + Compose UI tests
  ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Phase 4 --- QA, Security Hardening & Packaging**

  -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                      **Owner**   **Output / Artifact**
  -------- -------------------------------------------------------------------------------------------------------------------------------------------------- ----------- -------------------------------------------------------------------------
  T-050    Execute full bypass test matrix (Section 6.3 BT-01 through BT-12) on each target OS image.                                                         QA          Bypass test report with pass/fail per platform + documented limitations

  T-051    Performance benchmark: measure app launch latency, CPU idle, memory footprint on minimum-spec hardware (Intel i5 2018 / M1 / mid-range Android).   QA/Eng      performance_benchmarks.md with measurements vs. targets

  T-052    Security review: review IPC message handling, SQLite schema, update pipeline, code signing chain. Engage external pen tester if budget allows.     Security    security_review.md + SBOM (CycloneDX JSON)

  T-053    Build Windows MSI with WiX v4. Silent install test, GPO deployment test on a domain-joined Windows 11 VM.                                          Eng         FocusMe_x64.msi + WiX source .wxs files

  T-054    Build macOS PKG with postinstall script. Notarize, staple, and verify Gatekeeper acceptance on macOS 13 + 14.                                      Eng         FocusMe.pkg + notarization receipt + installer_checklist.md

  T-055    Build Linux .deb and .rpm packages with fpm. Test apt install and rpm -i on each target distro. Verify eBPF program loads on first install.        Eng         focusme_amd64.deb + focusme.x86_64.rpm + install test log

  T-056    Submit browser extension to Chrome Web Store + Firefox AMO. Complete store listing, screenshots, privacy disclosures, permissions justification.   PM/Eng      Extension listing URLs + store review checklist

  T-057    Publish Android app to Google Play. Complete data safety section, content rating questionnaire, permissions declarations.                          PM/Eng      Play Store listing + data_safety_declaration.md
  -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Phase 5 --- Cloud Backend & Team Analytics (Post-MVP)**

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Task (with method/detail)**                                                                                                                                       **Owner**   **Output / Artifact**
  -------- ------------------------------------------------------------------------------------------------------------------------------------------------------------------- ----------- -----------------------------------------------------------
  T-060    Design and implement cloud API (Node.js + Express or Fastify + PostgreSQL). Endpoints: /auth, /plans, /events, /reports, /export.                                   Eng         OpenAPI 3.1 spec + API implementation + integration tests

  T-061    Implement pseudonymous usage event pipeline. Daemon batches events (max 100 events, max 60s delay); HTTPS POST to /events with HMAC-SHA256 authentication.          Eng         event_batcher.rs + cloud ingestion endpoint + test

  T-062    Build team analytics dashboard (React/Next.js). Per-user usage trends, top blocked sites per org, plan compliance rate.                                             Eng         analytics_dashboard/ + Cypress E2E tests

  T-063    Implement data export endpoint. GET /v1/export?user=&from=&to=&format=csv\|json. Respect retention limits; return 404 after data purge.                             Eng         export.ts + test: CSV export matches event records

  T-064    GDPR/FERPA compliance validation. Run against GDPR checklist; produce Record of Processing Activities (ROPA). Legal review of data processing agreement template.   PM/Legal    ROPA.md + DPA template + compliance checklist
  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**10. Open Questions & Assumptions**

**10.1 Items to Validate Early**

  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Question / Risk**                                                                                                                                                                                                                **Impact**
  -------- ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- ---------------------------------------------------
  OQ-01    Apple ESF entitlement approval: timeline unpredictable (1--7 weeks). Apply immediately in Phase 0. Without it, macOS enforcement is limited to DNS-only blocking.                                                                  CRITICAL --- blocks macOS enforcement

  OQ-02    Does the target Linux kernel have CONFIG_BPF_LSM=y? Run T-001 validation before committing to eBPF approach. Pop!\_OS 22.04 should have it; Linux Mint may not.                                                                    CRITICAL --- may require Fanotify fallback

  OQ-03    Does the business require a self-hosted cloud option? Affects whether the cloud backend uses Dockerized deployment (Compose/K8s Helm chart) or managed cloud only.                                                                 Blocks cloud architecture decision

  OQ-04    DoH bypass on Windows: Chrome/Firefox with DoH enabled bypass HOSTS-based blocking. WFP can block port 443 to known DoH provider IPs but maintaining that list is ongoing. Confirm acceptable threat model.                        Affects enforceability guarantee for URL blocking

  OQ-05    Browser extension store review times: Chrome Web Store typically 3--7 days; Firefox AMO up to 14 days. Build review buffer into Phase 4 timeline.                                                                                  Affects release date

  OQ-06    macOS System Extension user approval friction: On first install, users MUST go to System Settings \> Privacy & Security and manually approve the System Extension. This is a UX friction point --- test conversion rate in beta.   Affects macOS adoption

  OQ-07    Android AccessibilityService restrictions: Google Play policy may scrutinize apps using AccessibilityService. Prepare a detailed justification for accessibility usage in the Play Store declaration.                              Potential Play Store rejection risk

  OQ-08    Secure Boot + MOK on Linux: If eBPF LSM requires a signed kernel module (distro-specific), the install flow must guide users through mokutil enrollment. Test on a VM with Secure Boot enabled before committing.                  Affects Linux installer complexity
  -----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**10.2 Key Assumptions Made in This Plan**

  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
  **ID**   **Assumption**                                                                                               **Contingency**
  -------- ------------------------------------------------------------------------------------------------------------ ------------------------------------------------------------------------
  A-01     MVP targets user-mode enforcement only (no kernel driver). Kernel drivers are post-MVP.                      Admin users can kill daemon on Windows; enforce with service hardening

  A-02     Target Linux kernel is 5.15+ with eBPF LSM support on at least Ubuntu 22.04.                                 If kernel \< 5.15, use Fanotify + iptables fallback

  A-03     iOS is out of scope entirely for the build (not just system-level blocking).                                 Re-evaluate if Apple relaxes Screen Time API restrictions

  A-04     Cloud backend is opt-in and not required for core functionality.                                             Offline-first design required for all enforcement components

  A-05     Team size is 2 engineers for daemon/core + 1 engineer for UI/extension + 1 QA.                               Phase 5 (cloud) may require additional cloud/backend engineer

  A-06     Tauri is the UI framework. If Tauri limitations arise (WebView inconsistencies), Electron is the fallback.   Electron increases binary size by \~100MB but is more proven

  A-07     SQLCipher is used for database encryption. License is Apache 2.0 for open-source builds.                     Commercial use of SQLCipher is free; confirm for enterprise builds
  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

**Appendix A --- Sample Telemetry Event Schema**

JSON Schema v2020-12 for a single telemetry event sent to the cloud analytics endpoint.

+----------------------------------------------------------------------------------------------------------------------------------------------+
| {                                                                                                                                            |
|                                                                                                                                              |
| \"\$schema\": \"https://focusme.app/schemas/telemetry_event_v1.json\",                                                                       |
|                                                                                                                                              |
| \"schema_version\": \"1.0.0\",                                                                                                               |
|                                                                                                                                              |
| \"event_id\": \"\<UUIDv4\>\",                                                                                                                |
|                                                                                                                                              |
| \"device_id\": \"\<HMAC-SHA256(machine_uuid, install_salt)\>\",                                                                              |
|                                                                                                                                              |
| \"tenant_id\": \"\<HMAC-SHA256(org_id, tenant_secret)\>\", // null for personal                                                              |
|                                                                                                                                              |
| \"app_version\": \"1.2.3\",                                                                                                                  |
|                                                                                                                                              |
| \"os\": \"windows \| macos \| linux \| android\",                                                                                            |
|                                                                                                                                              |
| \"os_version\": \"10.0.22631\",                                                                                                              |
|                                                                                                                                              |
| \"arch\": \"x64 \| arm64\",                                                                                                                  |
|                                                                                                                                              |
| \"event_type\": \"PLAN_STARTED \| PLAN_STOPPED \| APP_BLOCKED \| URL_BLOCKED \| QUOTA_REACHED \| FORCED_MODE_STARTED \| FORCED_MODE_ENDED\", |
|                                                                                                                                              |
| \"timestamp_utc\": \"\<ISO-8601\>\",                                                                                                         |
|                                                                                                                                              |
| \"session_id\": \"\<UUIDv4\>\",                                                                                                              |
|                                                                                                                                              |
| \"plan_id\": \"\<UUIDv4\>\",                                                                                                                 |
|                                                                                                                                              |
| \"rule_id\": \"\<UUIDv4 \| null\>\",                                                                                                         |
|                                                                                                                                              |
| \"subject_hash\": \"\<SHA-256 of blocked process name or domain\>\",                                                                         |
|                                                                                                                                              |
| \"duration_ms\": 1234, // for PLAN_STOPPED and FORCED_MODE_ENDED events                                                                      |
|                                                                                                                                              |
| \"quota_used_s\": 3600, // seconds of quota consumed today                                                                                   |
|                                                                                                                                              |
| \"quota_limit_s\": 7200                                                                                                                      |
|                                                                                                                                              |
| }                                                                                                                                            |
+----------------------------------------------------------------------------------------------------------------------------------------------+

**Appendix B --- Installer Pre-Release Checklist**

Run before every release. QA must sign off on each item.

**Windows Installer (MSI / NSIS)**

  --------------------------------------------------------------------------------------------------------------------------------------------------
  **Check**                                                             **Method**                                                **Result**
  --------------------------------------------------------------------- --------------------------------------------------------- ------------------
  Silent install completes without UI prompts                           Install-FocusMe.msi /quiet /norestart                     Pass / Fail

  Service registers and starts automatically                            Get-Service FocusMeDaemon shows Running                   Pass / Fail

  EV code signing certificate is valid                                  signtool verify /pa FocusMe.msi                           Pass / Fail

  Timestamp on signature is present (post-cert expiry resilience)       signtool verify /pa /v shows timestamp                    Pass / Fail

  SmartScreen warning does NOT appear (requires EV cert + reputation)   Launch EXE on clean Windows 11 VM                         Pass / Fail

  Uninstall removes all files and registry entries                      Programs & Features \> Uninstall; verify with Revo        Pass / Fail

  MSI transform (.mst) applies enterprise defaults                      msiexec /i FocusMe.msi TRANSFORMS=enterprise.mst /quiet   Pass / Fail
  --------------------------------------------------------------------------------------------------------------------------------------------------

**macOS Installer (PKG)**

  --------------------------------------------------------------------------------------------------------------------------------------
  **Check**                                                           **Method**                                      **Result**
  ------------------------------------------------------------------- ----------------------------------------------- ------------------
  Notarization accepted --- no Gatekeeper warning                     spctl -a -v FocusMe.pkg                         Pass / Fail

  Universal Binary contains both arm64 and x86_64 slices              lipo -info FocusMe.app/Contents/MacOS/FocusMe   Pass / Fail

  LaunchDaemon loads after install                                    launchctl list \| grep focusme                  Pass / Fail

  System Extension visible in System Settings \> Privacy & Security   Manual verification post-install                Pass / Fail

  User prompted to approve Network Extension                          Fresh install on macOS 14 VM                    Pass / Fail

  Sparkle update check works from installed app                       Hold Option + click \'Check for Updates\'       Pass / Fail
  --------------------------------------------------------------------------------------------------------------------------------------

**Linux (.deb)**

  -------------------------------------------------------------------------------------------------------------------------------
  **Check**                                                     **Method**                                     **Result**
  ------------------------------------------------------------- ---------------------------------------------- ------------------
  apt install completes without errors on Ubuntu 22.04          sudo apt install ./focusme_amd64.deb 2\>&1     Pass / Fail

  systemd unit is enabled and active after install              systemctl status focusme                       Pass / Fail

  eBPF program pinned to /sys/fs/bpf/focusme_exec_block         ls /sys/fs/bpf/focusme_exec_block              Pass / Fail

  GPG signature on .deb verifies against published key          dpkg-sig \--verify focusme_amd64.deb           Pass / Fail

  apt remove cleanly uninstalls service and removes eBPF pins   sudo apt remove focusme; verify /sys/fs/bpf/   Pass / Fail
  -------------------------------------------------------------------------------------------------------------------------------

**Appendix C --- Sample Policy JSON (Focus Plan)**

A complete Focus Plan object as stored in the policy database and exchanged over IPC. Version 1.0.0.

+-------------------------------------------------------------------------------------+
| {                                                                                   |
|                                                                                     |
| \"schema_version\": \"1.0.0\",                                                      |
|                                                                                     |
| \"plan_id\": \"a1b2c3d4-e5f6-7890-abcd-ef1234567890\",                              |
|                                                                                     |
| \"name\": \"Deep Work --- Morning Block\",                                          |
|                                                                                     |
| \"enabled\": true,                                                                  |
|                                                                                     |
| \"forced_mode\": true,                                                              |
|                                                                                     |
| \"forced_mode_max_duration_s\": 14400,                                              |
|                                                                                     |
| \"protection\": {                                                                   |
|                                                                                     |
| \"type\": \"argon2id_password\",                                                    |
|                                                                                     |
| \"hash\": \"\<argon2id hash of PIN\>\",                                             |
|                                                                                     |
| \"challenge_required\": true                                                        |
|                                                                                     |
| },                                                                                  |
|                                                                                     |
| \"schedules\": \[                                                                   |
|                                                                                     |
| {                                                                                   |
|                                                                                     |
| \"days\": \[\"mon\",\"tue\",\"wed\",\"thu\",\"fri\"\],                              |
|                                                                                     |
| \"start_time\": \"09:00\",                                                          |
|                                                                                     |
| \"end_time\": \"12:00\",                                                            |
|                                                                                     |
| \"timezone\": \"America/New_York\"                                                  |
|                                                                                     |
| }                                                                                   |
|                                                                                     |
| \],                                                                                 |
|                                                                                     |
| \"app_rules\": \[                                                                   |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"process_name\", \"value\": \"Spotify.exe\" },   |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"process_name\", \"value\": \"Discord\" },       |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"path_prefix\", \"value\": \"C:\\\\Games\\\\\" } |
|                                                                                     |
| \],                                                                                 |
|                                                                                     |
| \"url_rules\": \[                                                                   |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"domain\", \"value\": \"reddit.com\" },          |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"wildcard\", \"value\": \"\*.twitter.com\" },    |
|                                                                                     |
| { \"type\": \"block\", \"match\": \"path\", \"value\": \"youtube.com/shorts\" },    |
|                                                                                     |
| { \"type\": \"allow\", \"match\": \"domain\", \"value\": \"github.com\" }           |
|                                                                                     |
| \],                                                                                 |
|                                                                                     |
| \"quotas\": \[                                                                      |
|                                                                                     |
| {                                                                                   |
|                                                                                     |
| \"target_type\": \"domain\",                                                        |
|                                                                                     |
| \"target\": \"youtube.com\",                                                        |
|                                                                                     |
| \"daily_limit_s\": 1800,                                                            |
|                                                                                     |
| \"rationing\": { \"launches_per_day\": 3, \"min_break_s\": 1800 }                   |
|                                                                                     |
| }                                                                                   |
|                                                                                     |
| \],                                                                                 |
|                                                                                     |
| \"pomodoro\": {                                                                     |
|                                                                                     |
| \"enabled\": true,                                                                  |
|                                                                                     |
| \"work_duration_s\": 1500,                                                          |
|                                                                                     |
| \"break_duration_s\": 300,                                                          |
|                                                                                     |
| \"long_break_after\": 4,                                                            |
|                                                                                     |
| \"long_break_duration_s\": 900                                                      |
|                                                                                     |
| },                                                                                  |
|                                                                                     |
| \"created_at\": \"2025-01-15T09:00:00Z\",                                           |
|                                                                                     |
| \"modified_at\": \"2025-03-01T14:22:00Z\"                                           |
|                                                                                     |
| }                                                                                   |
+-------------------------------------------------------------------------------------+

*End of Document*

FocusMe-Style App Build Plan --- Version 1.0 --- Confidential Engineering Document
