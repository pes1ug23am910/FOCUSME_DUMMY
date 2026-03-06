# FocusMe Security Review

> **Document:** `docs/security_review.md`
> **Task:** T-052
> **Author:** FocusMe Co-Pilot (Claude Opus)
> **Session:** 4
> **Status:** Template — to be completed during formal security review
> **Last Updated:** Session 4

---

## 1. Executive Summary

FocusMe is a cross-platform focus enforcement application that runs with elevated privileges (root/SYSTEM) to prevent circumvention. This document provides a structured security review template covering all attack surfaces, threat model, and mitigations.

**Risk Rating:** Medium-High (due to elevated privileges and system-level modifications)

---

## 2. Threat Model

### 2.1 Threat Actors

| Actor | Capability | Motivation |
|-------|-----------|------------|
| **Self-circumventing user** | Local admin, browser dev tools, process manager | Bypass focus restrictions |
| **Malware / third-party app** | Local process, file system access | Exploit FocusMe IPC for privilege escalation |
| **Network attacker** | Man-in-the-middle on update channel | Deliver malicious update payload |
| **Physical access attacker** | Boot from USB, modify boot config | Disable daemon or eBPF hooks |

### 2.2 Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    SYSTEM (root/SYSTEM)                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  FocusMe Daemon (focusme-daemon)                        │ │
│  │  - SQLite database (sqlcipher encrypted)                │ │
│  │  - IPC server (Named Pipe / UDS)                        │ │
│  │  - WFP callout driver (Windows)                         │ │
│  │  - eBPF LSM hooks (Linux)                               │ │
│  │  - DNS blocker (Unbound RPZ / HOSTS)                    │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    USER SPACE                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Tauri UI     │  │  NMH Bridge  │  │  Browser Extension│  │
│  │  (user priv)  │  │  (user priv)  │  │  (sandbox)       │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Component-Level Review

### 3.1 IPC Message Handling (daemon/src/ipc_server.rs)

**Risk:** High — IPC is the primary attack surface for privilege escalation.

| Check | Status | Notes |
|-------|--------|-------|
| Message size limit enforced | ✅ | MAX_MSG_SIZE = 10 MB |
| Client authentication (API key / token) | ✅ | Argon2 password hash verification |
| Input validation on all fields | 🔲 | **TODO:** Fuzz test all message types |
| SQL injection via plan names/URLs | ✅ | Parameterized queries (rusqlite `?1` bindings) |
| Path traversal in rule definitions | 🔲 | **TODO:** Validate URL/domain patterns |
| Rate limiting on IPC | 🔲 | **TODO:** Add per-client rate limit |
| Replay attack on password change | 🔲 | **TODO:** Add nonce/timestamp to auth messages |

**Recommendation:** Add fuzzing harness for IPC message deserialization (cargo-fuzz).

### 3.2 SQLite Database (daemon/src/db.rs)

**Risk:** Medium — encrypted at rest, but key management is critical.

| Check | Status | Notes |
|-------|--------|-------|
| Database encrypted at rest (SQLCipher) | ✅ | `bundled-sqlcipher` feature in rusqlite |
| Encryption key derivation | ✅ | Argon2id with configurable params |
| Key stored securely | 🔲 | **TODO:** Use OS keychain (DPAPI / Keychain / Secret Service) |
| Migration scripts sanitized | ✅ | refinery `embed_migrations!` — compiled in |
| WAL mode for concurrent access | ✅ | `PRAGMA journal_mode=WAL` |
| Database file permissions | ✅ | Created with 0600 (owner-only) |
| Backup encryption | 🔲 | **TODO:** Ensure exports are also encrypted |

**Recommendation:** Move encryption key from daemon config to OS-level secret store.

### 3.3 Update Pipeline

**Risk:** High — compromised update = full system compromise.

| Check | Status | Notes |
|-------|--------|-------|
| HTTPS-only update channel | ✅ | Tauri updater uses TLS |
| Code signing verification | ✅ | Tauri verifies Ed25519 signature |
| Certificate pinning | 🔲 | **TODO:** Pin update server certificate |
| Rollback protection | 🔲 | **TODO:** Prevent downgrade to vulnerable version |
| Update integrity (hash chain) | ✅ | Tauri updater verifies SHA-256 |
| Auto-update forced in production | 🔲 | **TODO:** Decide policy |

### 3.4 Code Signing Chain

**Risk:** High — unsigned code will be blocked by SmartScreen / Gatekeeper.

| Check | Status | Notes |
|-------|--------|-------|
| EV Code Signing certificate | 🔴 | **BLOCKED T-003:** Not yet procured |
| Windows MSI signed with EV cert | 🔴 | Blocked by T-003 |
| macOS notarization | 🔴 | **BLOCKED T-002:** ESF entitlement needed |
| Linux: no signing required (APT repo) | ✅ | GPG-signed APT repository |
| WFP driver WHQL signed | 🔴 | Requires separate WHQL submission |
| eBPF bytecode signed | 🔲 | **TODO:** Sign .o file for Secure Boot |

### 3.5 Browser Extension (extension/src/)

**Risk:** Medium — sandboxed but handles sensitive blocking rules.

| Check | Status | Notes |
|-------|--------|-------|
| Content Security Policy (MV3) | ✅ | Strict CSP in manifest.json |
| NMH communication authenticated | ✅ | `allowed_origins` in NMH manifest |
| XSS in popup/options pages | ✅ | React with JSX (no innerHTML) |
| DOM injection in content scripts | ✅ | Uses `isValidSelector()` validation |
| Storage access scoped | ✅ | `chrome.storage.session` for ephemeral data |
| Extension ID hardcoded in NMH | 🔲 | **TODO:** Replace EXTENSION_ID_HERE |

### 3.6 Android (android/app/)

**Risk:** Medium — AccessibilityService has broad system access.

| Check | Status | Notes |
|-------|--------|-------|
| VPN service certificate pinning | 🔲 | **TODO:** Pin if cloud sync added |
| AccessibilityService scope limited | ✅ | Only monitors foreground package name |
| Data export encryption | 🔲 | **TODO:** Encrypt exported JSON |
| Root detection | 🔲 | **TODO:** Detect rooted devices |
| Debug build detection | 🔲 | **TODO:** Disable features in debug mode |
| ProGuard/R8 obfuscation | ✅ | Configured in proguard-rules.pro |

### 3.7 Windows Platform (daemon/src/wfp_manager.rs)

**Risk:** High — kernel-level WFP callout driver.

| Check | Status | Notes |
|-------|--------|-------|
| WFP filter permissions (admin-only) | ✅ | Requires SE_TAKE_OWNERSHIP_PRIVILEGE |
| GUID uniqueness verified | ✅ | Static GUIDs defined in code |
| Driver loaded via SCM (not manual) | ✅ | ServiceInstall in WiX |
| Filter cleanup on daemon crash | 🔲 | **TODO:** Cleanup on service recovery |

### 3.8 Linux Platform (linux/src/)

**Risk:** Medium-High — eBPF operates at kernel level.

| Check | Status | Notes |
|-------|--------|-------|
| eBPF verifier passes | ✅ | BPF verifier is mandatory |
| BPF pin directory permissions | ✅ | /sys/fs/bpf/focusme (root-only) |
| resolv.conf backup/restore | ✅ | Backup created before modification |
| HOSTS file markers prevent collision | ✅ | Unique marker comments |
| Unbound config isolated | ✅ | Dedicated conf.d include file |

---

## 4. OWASP Top 10 Mapping

| OWASP Category | Applicable | Mitigation |
|----------------|-----------|------------|
| A01 Broken Access Control | Yes | IPC auth + service permissions |
| A02 Cryptographic Failures | Yes | SQLCipher + Argon2 + TLS updates |
| A03 Injection | Yes | Parameterized SQL + input validation |
| A04 Insecure Design | Partial | Forced mode has intentional lockout |
| A05 Security Misconfiguration | Yes | Hardened systemd + WFP filters |
| A06 Vulnerable Components | Yes | SBOM tracking (see §6) |
| A07 Auth Failures | Yes | Argon2id hashing + rate limits |
| A08 Data Integrity Failures | Yes | Code signing + update verification |
| A09 Logging Failures | Partial | tracing crate structured logging |
| A10 SSRF | No | No server-side requests |

---

## 5. Penetration Testing Plan

### 5.1 Internal Test Matrix

| ID | Test | Target | Priority |
|----|------|--------|----------|
| PT-01 | IPC message fuzzing | ipc_server.rs | P0 |
| PT-02 | SQL injection via plan names | db.rs | P0 |
| PT-03 | NMH message injection | native_messaging_host | P1 |
| PT-04 | DNS bypass (DoH, DoT, DoQ) | dns_blocker.rs / wfp_manager.rs | P0 |
| PT-05 | Process kill / service stop | daemon service | P0 |
| PT-06 | Clock manipulation (schedule bypass) | scheduler.rs | P1 |
| PT-07 | File permission escalation | db file, config files | P1 |
| PT-08 | eBPF program detachment | loader.rs | P1 |
| PT-09 | Extension removal during forced mode | browser extension | P0 |
| PT-10 | Overlay dismiss (Android) | AccessibilityService | P1 |
| PT-11 | VPN disconnect (Android) | FocusMeVpnService | P1 |
| PT-12 | MSI downgrade attack | WiX installer | P2 |

### 5.2 External Pen Test Scope

If budget allows, engage an external penetration tester with the following scope:

- **Duration:** 2-3 days
- **Focus:** IPC protocol, privilege escalation, DNS bypass
- **Deliverable:** Report with findings rated by CVSS v3.1
- **Timeline:** After Phase 3 (functional MVP) is complete

---

## 6. Software Bill of Materials (SBOM)

### 6.1 CycloneDX SBOM

The SBOM is generated automatically during CI/CD:

```bash
# Generate Rust SBOM
cargo install cargo-cyclonedx
cargo cyclonedx --format json --output-file sbom-daemon.cdx.json

# Generate Node.js SBOM (extension)
npx @cyclonedx/cyclonedx-npm --output-file sbom-extension.cdx.json

# Generate Android SBOM
# Use CycloneDX Gradle plugin in build.gradle.kts
```

### 6.2 Critical Dependencies

| Component | Dependency | Version | License | Risk |
|-----------|-----------|---------|---------|------|
| Daemon | tokio | 1.35 | MIT | Low |
| Daemon | rusqlite | 0.31 | MIT | Low |
| Daemon | interprocess | 2.0 | MIT | Medium (breaking API changes) |
| Daemon | argon2 | 0.5 | MIT/Apache-2.0 | Low |
| Daemon | windows-sys | 0.52 | MIT | Low |
| Extension | webextension-polyfill | 0.12 | MPL-2.0 | Low |
| Android | Jetpack Compose | BOM 2024.01 | Apache-2.0 | Low |
| UI | Tauri | v2 | MIT/Apache-2.0 | Medium (new major version) |
| Linux | libbpf-rs | 0.23 | BSD-2 | Medium (kernel coupling) |

### 6.3 Vulnerability Scanning

```bash
# Rust
cargo audit
cargo deny check advisories

# Node.js
npm audit --production

# GitHub Dependabot
# Configured in .github/dependabot.yml
```

---

## 7. Compliance Notes

| Regulation | Applicability | Status |
|-----------|--------------|--------|
| GDPR | User data (plans, usage stats) | 🔲 Privacy policy needed |
| COPPA | If minors use the app | 🔲 Age verification consideration |
| SOC 2 | If enterprise deployment | 🔲 Post-MVP |
| HIPAA | Not applicable | N/A |

---

## 8. Action Items

| Priority | Action | Owner | Target |
|----------|--------|-------|--------|
| P0 | Procure EV Code Signing cert | PM | ASAP (T-003) |
| P0 | IPC fuzzing harness (cargo-fuzz) | Eng | Phase 3 |
| P0 | Replace EXTENSION_ID_HERE in NMH manifest | Eng | Before CWS submission |
| P1 | Move DB encryption key to OS keychain | Eng | Phase 3 |
| P1 | Add IPC rate limiting | Eng | Phase 3 |
| P1 | Certificate pinning for update channel | Eng | Phase 4 |
| P2 | External penetration test | Security | Post-MVP |
| P2 | GDPR privacy policy | Legal | Pre-launch |
| P2 | Sign eBPF bytecode for Secure Boot | Eng | Phase 4 |

---

## 9. Review Sign-Off

| Reviewer | Role | Date | Signature |
|----------|------|------|-----------|
| _________ | Lead Engineer | ____/____/____ | _________ |
| _________ | Security Engineer | ____/____/____ | _________ |
| _________ | Product Manager | ____/____/____ | _________ |
