# FocusMe — A Cross-Platform, System-Level Productivity Enforcer

[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20Browser-green.svg)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/typescript-5.3%2B-blue.svg)](https://www.typescriptlang.org/)
[![Kotlin](https://img.shields.io/badge/kotlin-Android-purple.svg)](https://kotlinlang.org/)

> A focus tool that blocks distracting apps and websites at the **operating-system level** instead of just inside the browser — so casual workarounds (incognito, a different browser, disabling an extension) don't defeat it.

**Author:** Yash Verma · SRN `PES1UG23AM910` · [@pes1ug23am910](https://github.com/pes1ug23am910)

I built FocusMe to learn real systems programming end-to-end: privileged OS daemons, kernel-adjacent enforcement APIs, encrypted local storage, an IPC protocol, a browser extension, a mobile app, and a cloud sync backend — all tied together into one product. It's written primarily in **Rust** (daemon + backend), with **TypeScript** (browser extension + Tauri UI), **Kotlin** (Android), and **Swift** (macOS).

---

## Table of Contents

- [Why I built it](#why-i-built-it)
- [Project status (read this first)](#project-status-read-this-first)
- [Features](#features)
- [Architecture](#architecture)
- [Platform enforcement matrix](#platform-enforcement-matrix)
- [Technology stack](#technology-stack)
- [Repository structure](#repository-structure)
- [Build & run](#build--run)
- [Testing](#testing)
- [Design decisions & trade-offs](#design-decisions--trade-offs)
- [What I learned](#what-i-learned)
- [Roadmap](#roadmap)
- [Documentation](#documentation)
- [License & contact](#license--contact)

---

## Why I built it

Browser-only blockers are trivially bypassed — you can open another browser, switch to a private window, or just turn the extension off. I wanted to see whether I could raise the cost of bypassing a blocker by enforcing rules across several independent layers, so defeating one doesn't defeat the whole system:

- **Network layer** — DNS filtering and outbound-connection blocking
- **Process layer** — preventing or terminating blocked applications
- **Browser layer** — an extension that coordinates with the local service
- **Data layer** — encrypted, tamper-resistant policy storage

The goal was never DRM-grade unbreakability — a determined user with admin rights can always win. The goal was to make *casual* circumvention impractical, and to learn the platform security APIs that make that possible.

---

## Project status (read this first)

This is an in-progress portfolio project, and I think being precise about what runs today is more useful than claiming it's all production-ready. The **core engine is functional**; several **platform-specific enforcement backends are scaffolded or blocked on OS-level approvals** (notably Apple's Endpoint Security entitlement and a kernel build flag for eBPF LSM on Linux).

| Subsystem | Status | Notes |
|---|---|---|
| Encrypted policy store (SQLCipher, 10-table schema, 33 data-access methods) | ✅ Working | Full CRUD for plans, schedules, rules, quotas, sessions, events |
| IPC server (12 commands, MessagePack + JSON framing) | ✅ Working | `PING`, `CONNECT`, plan CRUD, `URL_CHECK`, `APP_CHECK`, `STATUS`, `UNLOCK`, `STATS` |
| Plan protection (Argon2id hashing, challenge + emergency code, rate-limit constants) | ✅ Working | OWASP-recommended password hashing |
| Windows DNS/IP blocking via WFP (real `Fwpm*` FFI, transactional filter add/remove) | ✅ Working | Dynamic session, auto-cleanup on exit; deep-packet callout driver is post-MVP |
| Windows process monitoring (ToolHelp snapshot + `TerminateProcess`) | ✅ Working | Poll-loop with process-name / path matching |
| HOSTS-file manager (marker-delimited block, hash-based tamper check) | ✅ Working | Cross-platform fallback path |
| DST-aware scheduler (`chrono-tz`, overnight windows, manual override) | ◑ Partial | Activation logic done; event wiring to enforcement engines is in progress |
| Forced/lockdown mode (dual-clock: monotonic + wall, 24h cap) | ◑ Partial | In-memory timer works; reboot-survival persistence is wired in the DB layer but not yet hooked into the module |
| Cloud backend (Axum: JWT auth, plan sync push/pull, family dashboard, rate-limit + security-header middleware) | ✅ Working | Requires PostgreSQL; optional component |
| Browser extension (MV3 + MV2 manifests, DNR/`webRequest` rule converter, content-script element blocker) | ◑ Partial | Rule conversion + background worker done; native-messaging ↔ daemon IPC is currently a stub |
| Android (Jetpack Compose UI, VPN + Accessibility service skeletons) | ◑ Partial | UI and service scaffolding present; VPN packet path has placeholders |
| macOS Endpoint Security (exec-auth blocking) | ⛔ Blocked | Requires an Apple-granted ESF entitlement before it can run |
| macOS DNS proxy (`NEDNSProxyProvider`) | ◑ Partial | Query interception + NXDOMAIN synthesis scaffolded |
| Linux eBPF LSM exec blocking | ⛔ Blocked | Needs `CONFIG_BPF_LSM=y`; a Fanotify (`FAN_OPEN_EXEC_PERM`) fallback is partially implemented |
| Test suite | ✅ ~145 tests | ~76 in the daemon, ~69 in the backend (unit + integration) |

**Legend:** ✅ functional · ◑ partial / in progress · ⛔ blocked on external approval or kernel config

---

## Features

### Blocking
- **Website blocking** with domain, subdomain, and path-level matching (wildcards supported)
- **Application blocking** by process name or executable path
- **Schedule-based activation** — time windows with daily/weekly recurrence and DST handling
- **Quota management** — per-app and per-site time limits with a usage ledger
- **Whitelist mode** — block everything except an explicit allow-list

### Anti-circumvention
- **Forced / lockdown mode** — a time-locked window tracked with a dual clock (a monotonic timer to resist clock tampering plus wall-clock time for reboot survival)
- **Argon2id-protected plan edits** — modifying a locked plan requires the password
- **Encrypted policy storage** — SQLCipher (AES-256) for everything on disk
- **Active process termination** on Windows for blocked apps
- **Tamper checks** — hash verification on the HOSTS file

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                           USER INTERFACES                              │
│  ┌──────────────┐   ┌───────────────────┐   ┌──────────────────────┐  │
│  │  Tauri Shell  │   │ Browser Extension │   │     Android App      │  │
│  │  React + TS   │   │   MV3 / MV2 + NMH │   │  Compose + Services  │  │
│  └──────┬───────┘   └─────────┬─────────┘   └──────────┬───────────┘  │
│         │ IPC (MessagePack)   │ Native Msg (4-byte LE) │ local IPC     │
├─────────┴─────────────────────┴────────────────────────┴──────────────┤
│                       DAEMON / SERVICE LAYER (Rust)                     │
│  ┌─────────┐  ┌───────────┐  ┌─────────────┐  ┌────────────────────┐   │
│  │   IPC   │  │ Scheduler │  │ Forced Mode │  │  Plan Protection   │   │
│  │ Server  │  │   (DST)   │  │ (dual-clock)│  │     (Argon2id)     │   │
│  └────┬────┘  └─────┬─────┘  └──────┬──────┘  └────────────────────┘   │
│       └─────────────┴───────────────┴───────────────────────────────┐  │
│        Policy Store — SQLCipher (AES-256) + WAL, 10-table schema     │  │
├──────────────────────────────────────────────────────────────────────┤
│                     PLATFORM ENFORCEMENT ENGINES                       │
│  ┌──────────┐   ┌───────────┐   ┌────────────┐   ┌─────────────────┐  │
│  │  WFP +   │   │  ESF +    │   │   eBPF /   │   │ Accessibility + │  │
│  │  HOSTS   │   │ DNS Proxy │   │  Fanotify  │   │   VPN Service   │  │
│  │ (Windows)│   │  (macOS)  │   │  (Linux)   │   │   (Android)     │  │
│  └──────────┘   └───────────┘   └────────────┘   └─────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Data flow: plan creation → enforcement
1. The user creates a plan in the Tauri UI, which sends a `PLAN_CREATE` IPC message.
2. The daemon validates the payload and stores it in the encrypted SQLCipher database.
3. The scheduler loads active rules at the scheduled time window.
4. The relevant platform engine applies the rules (e.g. WFP filters on Windows).
5. The browser extension polls for the current rule set and applies `declarativeNetRequest` rules.

> The scheduler → engine notification path and the extension's native-messaging bridge to the daemon are the two integration seams I'm still wiring up (see [Project status](#project-status-read-this-first)).

---

## Platform enforcement matrix

| Platform | DNS / network blocking | App / process blocking | Notes |
|---|---|---|---|
| **Windows** | WFP filters (`Fwpm*` FFI) + HOSTS | `CreateToolhelp32Snapshot` + `TerminateProcess` | Most complete backend; admin required |
| **macOS** | `NEDNSProxyProvider` (NXDOMAIN) | ESF `ES_EVENT_TYPE_AUTH_EXEC` | Exec blocking blocked on Apple ESF entitlement |
| **Linux** | Unbound RPZ + HOSTS | eBPF LSM hook / Fanotify fallback | LSM attach needs `CONFIG_BPF_LSM=y` |
| **Android** | Local VPN DNS intercept | Foreground-app detection (Accessibility) | VPN packet path partially implemented |
| **Browser** | `declarativeNetRequest` (MV3) / `webRequest` (MV2) | N/A | Coordinates with daemon via a native-messaging host |

---

## Technology stack

**Daemon (Rust)** — `tokio` async runtime, `serde` + `rmp-serde` (MessagePack), `rusqlite` with bundled SQLCipher, `refinery` migrations, `argon2`, `sha2`, `hmac`, `ed25519-dalek`, `chrono` + `chrono-tz`, `tracing`, `interprocess` (Named Pipes on Windows / Unix domain sockets elsewhere), `jsonschema`. Windows-specific: `windows-sys` (WFP, ToolHelp, Services) and `windows-service`. Linux-specific: `libbpf-rs` and `nix` (Fanotify).

**Cloud backend (Rust)** — `axum` for HTTP, `sqlx` against PostgreSQL, JWT auth with Argon2id password hashing, and `tower`/`tower-http` middleware for CORS, rate limiting, and security headers.

**Desktop UI (Tauri + React)** — Tauri v2, React 18, TypeScript 5.3, Tailwind CSS, with a plan-creation wizard and a usage/stats view that talk to the daemon over IPC.

**Browser extension (TypeScript)** — Webpack 5 build, Manifest V3 for Chrome/Edge and V2 for Firefox, a rule converter that maps FocusMe URL rules to `declarativeNetRequest` rules and `webRequest` patterns, a `MutationObserver`-based content-script element blocker, and a Rust native-messaging host using Chrome's 4-byte length-prefixed framing.

**Android (Kotlin)** — Jetpack Compose (Material 3), an MVVM structure, a foreground VPN service for DNS interception, and an Accessibility service for foreground-app detection. Min SDK 26.

**macOS (Swift)** — an Endpoint Security Framework client for execution authorization and a Network Extension DNS proxy.

**Linux (Rust + eBPF C)** — a CO-RE eBPF LSM program (`lsm/bprm_check_security`) with a Rust loader, plus a Fanotify-based fallback and Unbound RPZ for DNS.

---

## Repository structure

```
focusme/
├── daemon/                      # Core enforcement service (Rust) — the heart of the project
│   ├── src/
│   │   ├── main.rs              # Windows Service / Linux daemon entry point
│   │   ├── db.rs                # SQLCipher policy store (~1,060 lines, 33 methods)
│   │   ├── db_schema.sql        # 10-table schema + indexes
│   │   ├── ipc_server.rs        # MessagePack/JSON IPC server (12 commands)
│   │   ├── scheduler.rs         # DST-aware plan activation
│   │   ├── forced_mode.rs       # Dual-clock lockdown timer
│   │   ├── process_monitor.rs   # Win32 process enumeration + termination
│   │   ├── hosts_manager.rs     # HOSTS-file manipulation + tamper check
│   │   ├── wfp_manager.rs       # Windows Filtering Platform FFI (~600 lines)
│   │   └── plan_protection.rs   # Argon2id hashing + challenge/emergency codes
│   ├── tests/                   # Integration tests
│   └── migrations/              # Refinery SQL migrations
│
├── backend/                     # Optional cloud server (Rust + Axum)
│   ├── src/
│   │   ├── main.rs              # Router: /plans, /sync, /family, /health, /api/v1/auth
│   │   ├── auth.rs              # JWT + Argon2id
│   │   ├── routes/              # auth_routes, sync (push/pull), family, health
│   │   ├── middleware/          # rate_limit, request_id, security_headers
│   │   └── analytics.rs         # Opt-in telemetry schema
│   ├── migrations/              # PostgreSQL schema (sqlx)
│   └── openapi.yml              # API specification
│
├── extension/                   # Browser extension (TypeScript + Rust NMH)
│   ├── src/
│   │   ├── background.ts        # Service worker, native-messaging client
│   │   ├── rule_converter.ts    # FocusMe rules → DNR / webRequest patterns
│   │   └── content_scripts/     # MutationObserver element blocker
│   ├── native_messaging_host/   # Rust NMH binary (length-prefixed framing)
│   ├── manifest.v3.json         # Chrome / Edge
│   └── manifest.v2.json         # Firefox
│
├── ui/                          # Desktop UI (Tauri + React)
│   ├── src/                     # PlanWizard.tsx, StatsPage.tsx, i18n strings
│   └── src-tauri/               # Tauri Rust bridge to the daemon
│
├── android/                     # Android app (Kotlin, Jetpack Compose)
│   └── app/src/main/java/com/focusme/android/
│       ├── service/             # VPN, Accessibility, foreground daemon services
│       └── ui/screens/          # Compose screens (plan list/edit, etc.)
│
├── macos/                       # macOS System Extension (Swift)
│   └── FocusMeESF/              # ESF client + NEDNSProxyProvider
│
├── linux/                       # Linux enforcement (Rust + eBPF C)
│   ├── bpf/                     # eBPF LSM program (CO-RE)
│   └── src/                     # Loader, Fanotify fallback, Unbound DNS blocker
│
├── packaging/                   # Installers (WiX MSI, .deb/RPM, macOS PKG)
├── docs/                        # Architecture, IPC spec, security review, audit, ADRs
├── LICENSE                      # MIT
├── privacy_policy.md  tos.md  eula.md
└── README.md                    # This file
```

---

## Build & run

### Prerequisites
- **Rust** 1.75+ (stable)
- **Node.js** 18 or 20 LTS
- Platform build tools:
  - **Windows:** Visual Studio Build Tools 2022 + Windows SDK 10.0.22000+
  - **macOS:** Xcode 15+ and Command Line Tools
  - **Linux:** `clang`, `libbpf-dev`, `pkg-config`, `libssl-dev`, `unbound`
  - **Android:** Android Studio, SDK 26+, NDK 25+
- The daemon needs **Administrator (Windows)** or **root/sudo (Linux/macOS)** to enforce.

### 1. Clone
```bash
git clone https://github.com/pes1ug23am910/focusme.git
cd focusme
```

### 2. Build the daemon (core service)
```bash
cd daemon
cargo build --release
# → target/release/focusme-daemon(.exe)
```

Install it as a service:

```bash
# Linux (systemd)
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo cp ../linux/focusme.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now focusme.service

# macOS (LaunchDaemon)
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo cp ../macos/com.focusme.daemon.plist /Library/LaunchDaemons/
sudo launchctl bootstrap system /Library/LaunchDaemons/com.focusme.daemon.plist
```

```powershell
# Windows (run as Administrator)
sc.exe create FocusMeDaemon binPath= "C:\Program Files\FocusMe\focusme-daemon.exe" start= auto
sc.exe start FocusMeDaemon
```

### 3. Build the desktop UI (Tauri)
```bash
cd ui
npm install
npm run tauri dev      # hot-reload development
npm run tauri build    # production bundle
```

### 4. Build the browser extension
```bash
cd extension
npm install
npm run build:mv3      # Chrome / Edge → dist-chrome/
npm run build:mv2      # Firefox      → dist-firefox/
```
Load it unpacked from `chrome://extensions` (Developer mode → Load unpacked) or `about:debugging` in Firefox.

### 5. Build the Android app (optional)
```bash
cd android
./gradlew assembleDebug   # → app/build/outputs/apk/debug/app-debug.apk
```

### 6. Run the cloud backend (optional)
```bash
cd backend
docker compose up -d postgres
cp .env.example .env       # set JWT_SECRET, DATABASE_URL
cargo install sqlx-cli --features postgres --no-default-features
cargo sqlx migrate run
cargo run --release        # listens on http://0.0.0.0:8080
```

---

## Testing

```bash
cd daemon  && cargo test          # ~76 unit + integration tests
cd backend && cargo test          # ~69 tests (auth, sync, family, middleware)
cd extension && npm test
cd ui && npm test
cd android && ./gradlew testDebugUnitTest
```

The daemon tests cover the policy store, IPC routing, the scheduler's DST/overnight logic, forced-mode timing, Argon2id verification, HOSTS manipulation, and WFP helpers. The backend tests cover auth flows, sync contracts, the family dashboard, and the rate-limit/security-header middleware.

---

## Design decisions & trade-offs

| Decision | Why |
|---|---|
| **MessagePack for IPC** | Compact binary framing, faster than JSON; I kept a JSON fallback for debugging. |
| **SQLCipher + WAL** | Transparent AES-256 at rest with concurrent reads — the policy store holds the only persistent state, so encrypting it mattered. |
| **Dual-clock forced mode** | A monotonic timer resists "just change the system clock" bypasses, while wall-clock time lets a lock survive a reboot. |
| **Argon2id over bcrypt** | Memory-hard, OWASP-recommended, resistant to GPU cracking. |
| **Rust for the daemon and backend** | Memory safety in privileged, long-running code, plus a single language across the service and server. |
| **Monorepo** | All platforms in one place made the IPC protocol and the shared policy schema far easier to keep in sync. |
| **WFP filters before a kernel callout driver** | Filter-level IP blocking gets most of the value without the complexity and signing burden of a kernel-mode driver, which I scoped as post-MVP. |

---

## What I learned

This project pushed me well outside application code. The parts that taught me the most:

- **Privileged services are their own discipline** — service lifecycle, shutdown signalling, and "what happens on reboot" are design problems, not afterthoughts.
- **Each OS exposes blocking through a completely different model** — WFP on Windows, Endpoint Security on macOS, eBPF/LSM and Fanotify on Linux, and a VPN + Accessibility combination on Android. Designing one policy schema that all of them can consume was the central architectural challenge.
- **Some doors only open with permission** — Apple's ESF entitlement and the Linux `CONFIG_BPF_LSM` kernel flag are hard external dependencies, and learning to design a graceful fallback (Fanotify, DNS proxy) around a blocked capability was a real lesson.
- **Designing the IPC protocol and the encrypted schema first** made the UI, extension, and mobile clients much cheaper to build against.

---

## Roadmap

- [ ] Wire the scheduler's activation events directly into each enforcement engine
- [ ] Connect the browser extension's native-messaging host to the daemon over IPC
- [ ] Persist forced-mode state across reboots through the existing DB methods
- [ ] Obtain the Apple ESF entitlement to enable macOS execution blocking
- [ ] Finish the Android VPN packet path (UDP/53 routing, checksums)
- [ ] Add a WFP callout driver for deep-packet inspection
- [ ] End-to-end integration tests spanning UI → daemon → enforcement

---

## Documentation

Detailed design notes live in [`docs/`](docs/):

| Document | What's in it |
|---|---|
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | System architecture, data flows, enforcement model |
| [`docs/ipc_protocol_v1.md`](docs/ipc_protocol_v1.md) | IPC framing, message envelope, error codes |
| [`docs/policy_schema_v1.json`](docs/policy_schema_v1.json) | JSON Schema for Focus Plans |
| [`docs/security_review.md`](docs/security_review.md) | Threat model and bypass analysis |
| [`docs/bypass_tests.md`](docs/bypass_tests.md) | Documented bypass attempts and expected outcomes |
| [`docs/decisions.md`](docs/decisions.md) | Architecture decision records |
| [`docs/PROJECT_AUDIT.md`](docs/PROJECT_AUDIT.md) | A full status audit of the codebase |

---

## License & contact

Licensed under the **MIT License** — see [LICENSE](LICENSE).

**Yash Verma** · SRN `PES1UG23AM910` · GitHub [@pes1ug23am910](https://github.com/pes1ug23am910)

*A learning project in cross-platform systems programming — built with Rust, TypeScript, Kotlin, and Swift.*
