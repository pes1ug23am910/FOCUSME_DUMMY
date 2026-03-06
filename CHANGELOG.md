# Changelog

All notable changes to the FocusMe project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Pending
- [ ] Apple ESF entitlement approval (BLOCKED T-002)
- [ ] EV Code Signing certificate procurement (BLOCKED T-003)
- [ ] eBPF kernel config validation (BLOCKED T-001)
- [ ] Legal review of privacy policy, ToS, EULA
- [ ] Live privacy policy URL for store submissions
- [ ] Phase 5 cloud backend (post-MVP)

---

## [0.1.0-alpha] — 2026-03 (Sessions 1–6)

### Added

#### Daemon (Rust — Windows/Linux cross-platform)
- `daemon/src/main.rs` — DaemonState with Arc<T> subsystem sharing (D-009), Windows SCM service + Linux systemd daemon entry points
- `daemon/src/db.rs` — 1,062-line policy store: SQLCipher WAL encryption (D-006), 10-table CRUD, refinery migrations, 8 unit tests
- `daemon/src/hosts_manager.rs` — HOSTS file manager with SHA-256 poll-based tamper detection (D-010, 2s interval), auto-restore
- `daemon/src/wfp_manager.rs` — ~450-line Windows Filtering Platform FFI: DNS/DoH/HTTPS blocking, 14 DoH provider IPs blocked (S-001 resolved), 5 unit tests
- `daemon/src/process_monitor.rs` — CreateToolhelp32Snapshot + TerminateProcess process scanning
- `daemon/src/ipc_server.rs` — Named Pipe (Windows) / UDS (Unix) IPC server, 12 DB-backed message handlers, 4-byte LE framing
- `daemon/src/scheduler.rs` — DST-aware plan scheduler with chrono-tz, JSON day parsing, DB plan loading
- `daemon/src/forced_mode.rs` — Dual-clock forced mode (monotonic + wall), Argon2id emergency unlock, DB persistence
- `daemon/src/plan_protection.rs` — Argon2id password hashing, 8-char challenge, 8-digit unlock code, 6 unit tests
- `daemon/migrations/V1__initial_schema.sql` — 10-table schema: plans, schedules, app_rules, url_rules, quotas, quota_ledger, sessions, events, forced_mode_state, settings

#### macOS (Swift)
- `macos/FocusMeESF/DNSProxyProvider.swift` — NEDNSProxyProvider with NXDOMAIN synthesis, DNS header parsing, UDP forwarding
- `macos/FocusMeESF/FocusMeESF.swift` — ESF exec blocking scaffold (BLOCKED T-002 — awaiting Apple entitlement)
- `macos/com.focusme.daemon.plist` — LaunchDaemon configuration

#### Linux (Rust + eBPF)
- `linux/src/loader.rs` — Full Fanotify fallback (FAN_OPEN_EXEC_PERM event loop, deny/allow response, HashSet blocked paths) + eBPF LSM skeleton (D-004). LGPL-2.1 compliance notice (S-006 resolved)
- `linux/src/dns_blocker.rs` — Unbound RPZ zone writes + HOSTS fallback + resolv.conf backup/restore
- `linux/bpf/focusme_lsm.bpf.c` — eBPF LSM bprm_check_security hook (BLOCKED T-001 — attach pending kernel config check)
- `linux/focusme.service` — Hardened systemd unit file

#### Browser Extension (TypeScript — MV3/MV2)
- `extension/manifest.v3.json` — Chrome MV3 manifest with declarativeNetRequest
- `extension/manifest.v2.json` — Firefox MV2 manifest with webRequest
- `extension/src/background.ts` — Native messaging with exponential backoff (2s→32s, 5 retries), BLOCK redirect, 30s alarm sync
- `extension/src/rule_converter.ts` — DNR rule converter: `||domain|` format, MAX_DNR=5000, allow rules priority=2
- `extension/src/content_scripts/element_blocker.ts` — Dual MutationObserver with data-focusme-protected style, 2s integrity re-check
- `extension/src/popup/` — Extension popup UI (HTML + CSS + TypeScript)
- `extension/src/blocked/` — Blocked page UI (HTML + CSS + TypeScript)
- `extension/icons/focusme_icon.svg` — Shield + crosshair icon design (blue-to-teal gradient)
- `extension/native_messaging_host/` — Rust NMH: Named Pipe/UDS + 4-byte LE + MessagePack (D-002)

#### Desktop UI (Tauri v2 — React/TypeScript)
- `ui/src/PlanWizard.tsx` — 7-step plan creation wizard with Tauri invoke()
- `ui/src/StatsPage.tsx` — Statistics page with Recharts ComposedChart + Tauri IPC
- `ui/src/i18n/strings_en.json` — 120+ i18n keys
- `ui/src-tauri/` — Tauri v2 shell: lib.rs with real IPC commands, tauri.conf.json, rmp-serde (D-011)

#### Android (Kotlin — Jetpack Compose)
- `android/app/src/main/java/.../FocusMeAccessibilityService.kt` — AccessibilityService with XML overlay inflation
- `android/app/src/main/java/.../FocusMeVpnService.kt` — Local VPN DNS interceptor: IPv4/UDP/DNS parse, NXDOMAIN synthesis, IP checksum (D-007)
- `android/app/src/main/java/.../QuotaTracker.kt` — UsageStatsManager INTERVAL_DAILY reconciliation
- `android/app/src/main/java/.../FocusMeDaemonService.kt` — SharedPreferences JSON plan management (D-012), 30s Handler scheduler
- `android/app/src/main/java/.../ui/screens/` — 4 Compose screens: PlanList, PlanEdit, Settings, MainNavigation
- `android/app/src/main/res/layout/overlay_blocked.xml` — ConstraintLayout blocking overlay

#### Packaging
- `packaging/windows/focusme.wxs` — WiX v4 MSI: ServiceInstall, NMH registry (Chrome/Edge/Firefox), upgrade support
- `packaging/windows/enterprise.mst` — Enterprise MST transform (SILENT_INSTALL, DISABLE_UNINSTALL)
- `packaging/linux/postinst` — Debian postinst: systemd enable, eBPF load, Secure Boot detection, DNS config
- `packaging/linux/prerm` — Debian prerm: service stop, HOSTS cleanup, eBPF unload
- `packaging/macos/postinstall` — PKG postinstall: LaunchDaemon load, DNS proxy enable, Accessibility check, logging setup
- `packaging/macos/preinstall` — PKG preinstall: macOS 12+ check, stop existing service, backup config, disk space, SIP status

#### Documentation
- `docs/policy_schema_v1.json` — JSON Schema 2020-12 plan policy format
- `docs/ipc_protocol_v1.md` — Full IPC protocol specification
- `docs/security_review.md` — Threat model, OWASP mapping, penetration test plan, CycloneDX SBOM
- `docs/performance_benchmarks.md` — Performance targets, test matrices, stress test procedures, benchmark scripts
- `docs/bypass_tests.md` — 12 bypass test procedures (BT-01 through BT-12)
- `docs/installer_checklist.md` — 43-item pre-release sign-off checklist (Appendix B)
- `docs/store_submissions/chrome_web_store.md` — CWS listing, permissions justification, privacy practices
- `docs/store_submissions/firefox_amo.md` — AMO listing, source code submission, self-hosted XPI fallback
- `docs/store_submissions/google_play.md` — Play listing, Data Safety form, AccessibilityService justification (S-007 resolved)
- `docs/ARCHITECTURE.md` — System architecture reference (data flow, IPC, platform enforcement matrix)

#### Legal Templates
- `privacy_policy.md` — GDPR/CCPA/COPPA compliant privacy policy (LEGAL REVIEW REQUIRED)
- `tos.md` — Terms of Service (LEGAL REVIEW REQUIRED)
- `eula.md` — End User License Agreement with platform addenda (LEGAL REVIEW REQUIRED)

#### Infrastructure
- `.github/workflows/ci.yml` — 6-job CI matrix: daemon (Win/Mac/Linux), extension, NMH, Android, UI, security audit
- `.github/dependabot.yml` — Automated dependency updates for Cargo, npm, Gradle, GitHub Actions
- `CONTRIBUTING.md` — Development environment setup, code style, commit format, PR checklist
- `CODEOWNERS` — Code ownership mapping
- `.gitignore` — Comprehensive ignore rules

### Changed
- `daemon/src/db.rs` — Migrated from `std::sync::Mutex` to `tokio::sync::RwLock` for read/write lock separation (D-013, resolves S-005). 11 read methods use `blocking_read()`, 13 write methods use `blocking_write()`.

### Security
- S-001: DoH bypass mitigated — WFP blocks 14 known DoH provider IPs (1.1.1.1, 8.8.8.8, etc.)
- S-005: Read/write lock separation reduces contention window for concurrent plan store access
- S-006: LGPL-2.1 compliance notice added to loader.rs for libbpf-rs dependency
- S-007: Google Play AccessibilityService justification documented with core functionality declaration

### Architecture Decisions
- D-001: Monorepo layout
- D-002: MessagePack primary + JSON debug for IPC
- D-003: HOSTS + WFP belt-and-suspenders (Windows)
- D-004: eBPF LSM primary + Fanotify fallback (Linux)
- D-005: Tauri for UI shell
- D-006: SQLCipher database encryption
- D-007: Local VPN for Android DNS blocking
- D-008: Dual MV3 + MV2 extension builds
- D-009: DaemonState Arc<T> subsystem sharing
- D-010: Poll-based HOSTS tamper detection (2s interval)
- D-011: Tauri v2
- D-012: SharedPreferences + JSON for Android plans
- D-013: tokio::sync::RwLock for database connection (replaces Mutex)

---

## [0.2.0-beta] — TBD

### Planned
- macOS ESF exec blocking (pending T-002 entitlement)
- eBPF LSM attach (pending T-001 kernel config check)
- Signed Windows MSI (pending T-003 EV cert)
- Chrome Web Store + Firefox AMO + Google Play submissions (pending legal review)
- Performance benchmark measurements on real hardware
- Bypass test execution (BT-01 through BT-12)

---

## [1.0.0] — TBD

### Planned
- All platform enforcement fully operational
- Store-distributed builds (CWS, AMO, Play Store)
- Signed and notarized installers (Windows MSI, macOS PKG)
- Enterprise deployment support (GPO, Intune, MDM)
- Phase 5 cloud backend (sync, family dashboard, plan sharing)
