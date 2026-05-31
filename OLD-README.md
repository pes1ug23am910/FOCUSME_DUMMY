# FocusMe — Cross-Platform Productivity Enforcer

[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android%20%7C%20Browser-green.svg)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/typescript-5.3%2B-blue.svg)](https://www.typescriptlang.org/)

> **System-level productivity enforcement with kernel-level blocking, encrypted policy storage, and anti-circumvention protections across 5 platforms.**

**Developed by:** Yash Verma (PES1UG23AM910)

FocusMe is an advanced, multi-platform productivity application that blocks distracting websites and applications at the **operating system level**, making circumvention significantly harder than browser-only solutions. Built with Rust for performance and safety, it demonstrates production-grade system programming, cross-platform architecture, and real-world security hardening.

---

## Table of Contents

- [Problem Statement](#-problem-statement)
- [Key Features](#-key-features)
- [Architecture](#️-architecture--system-design)
- [Technology Stack](#-technology-stack)
- [Setup & Installation](#️-setup--installation)
- [Project Structure](#-project-structure)
- [Testing](#-testing)
- [Documentation](#-documentation)
- [Skills Demonstrated](#-skills-demonstrated-for-recruiters)

---

## 🎯 Problem Statement

Browser-only content blockers are trivially bypassed:
- ❌ Disable the extension
- ❌ Use incognito/private mode
- ❌ Switch to a different browser
- ❌ Uninstall and reinstall later

**FocusMe's Solution:** Multi-layer enforcement that requires defeating multiple independent subsystems:

1. **Network Layer:** DNS filtering + packet inspection
2. **Process Layer:** Binary execution prevention at kernel level
3. **Browser Layer:** Extension-based coordination via Native Messaging
4. **Data Layer:** Encrypted, tamper-resistant policy storage

This design raises the effort threshold significantly, making casual bypasses impractical.

---

## 🚀 Key Features

### Core Blocking Capabilities
- ✅ **Website Blocking:** Domain, subdomain, and path-level matching with wildcard support
- ✅ **Application Blocking:** Process name and executable path matching
- ✅ **Schedule-Based Activation:** Time windows with daily/weekly recurrence and DST handling
- ✅ **Quota Management:** Per-app and per-site time limits with rollover options
- ✅ **Whitelist Mode:** Block everything except explicitly allowed apps/sites

### Security & Anti-Circumvention
- 🔒 **Forced Mode:** Time-locked enforcement that survives process termination and system reboots using dual-clock (monotonic + wall time) tracking
- 🔒 **Plan Protection:** Argon2id password hashing (OWASP-recommended) for plan modifications
- 🔒 **Tamper Resistance:** Self-healing service monitoring, write-protected enforcement
- 🔒 **Encrypted Storage:** SQLCipher (AES-256) for all policy data
- 🔒 **Process Termination:** Actively terminates blocked processes (Windows)

### Platform-Specific Enforcement

| Platform | DNS Blocking | Process/App Blocking | Bypass Resistance |
|----------|-------------|---------------------|-------------------|
| **Windows** | WFP DNS filter + HOSTS | `CreateToolhelp32Snapshot` + `TerminateProcess` | **High** (kernel callout) |
| **macOS** | `NEDNSProxyProvider` NXDOMAIN | ESF `ES_EVENT_TYPE_AUTH_EXEC` | **Very High** (System Extension) |
| **Linux** | Unbound RPZ + HOSTS | eBPF LSM / Fanotify `FAN_OPEN_EXEC_PERM` | **High** (CAP_SYS_ADMIN) |
| **Android** | Local VPN DNS intercept | Foreground app detection | **Medium** (VPN permission) |
| **Browser** | `declarativeNetRequest` (MV3) / `webRequest` (MV2) | N/A | **Low** (extension-level) |

---

## 🏗️ Architecture & System Design

### High-Level Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                          USER INTERFACES                              │
│  ┌──────────────┐  ┌──────────────────┐  ┌──────────────────────┐   │
│  │ Tauri Shell   │  │ Browser Extension│  │   Android App        │   │
│  │ React + TS    │  │ MV3/MV2 + NMH   │  │ Compose + Services   │   │
│  └──────┬────────┘  └────────┬─────────┘  └──────────┬───────────┘   │
│         │ IPC (MessagePack)  │ NMH (4-byte LE)       │ Local IPC     │
├─────────┴────────────────────┴───────────────────────┴───────────────┤
│                      DAEMON / SERVICE LAYER                           │
│  ┌─────────┐  ┌──────────┐  ┌────────────┐  ┌───────────────────┐   │
│  │ IPC     │  │ Scheduler│  │ Forced Mode│  │ Plan Protection   │   │
│  │ Server  │  │ (DST)    │  │ (dual-clk) │  │ (Argon2id)        │   │
│  └────┬────┘  └─────┬────┘  └─────┬──────┘  └───────────────────┘   │
│       │             │              │                                  │
│  ┌────┴─────────────┴──────────────┴──────────────────────────────┐  │
│  │            Policy Store (SQLCipher + WAL mode)                  │  │
│  │      10-table schema: plans, schedules, rules, quotas, etc.     │  │
│  └─────────────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────────────┤
│                   PLATFORM ENFORCEMENT ENGINES                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌──────────────────┐   │
│  │ WFP +    │  │ ESF +    │  │ eBPF/     │  │ Accessibility +  │   │
│  │ HOSTS    │  │ DNS Proxy│  │ Fanotify  │  │ VPN Service      │   │
│  │ (Win)    │  │ (macOS)  │  │ (Linux)   │  │ (Android)        │   │
│  └──────────┘  └──────────┘  └───────────┘  └──────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### Data Flow: Plan Creation → Enforcement

1. **User creates plan** → Tauri UI calls `create_plan` IPC command
2. **Daemon receives request** → Validates schema and stores in SQLCipher database
3. **Scheduler activates plan** → Loads rules into memory at scheduled time window
4. **Enforcement engines apply rules** → Platform-specific kernel hooks intercept and block
5. **Browser extension syncs** → Native Messaging Host provides rule updates (30s interval)

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **MessagePack IPC** | Compact binary format (smaller than JSON), fast serialization |
| **SQLCipher + WAL** | Transparent AES-256 encryption with concurrent read performance |
| **Dual-clock forced mode** | Monotonic timer prevents time manipulation, wall clock survives reboot |
| **RwLock for database** | Multiple concurrent readers, exclusive writer, no deadlocks |
| **Tokio async runtime** | High-performance async I/O for IPC and network operations |
| **Monorepo structure** | Simplified CI/CD, easier cross-module refactoring |

---

## 💻 Technology Stack

### Backend / Daemon (Rust)
- **Async Runtime:** `tokio` 1.35 (full features)
- **Serialization:** `serde`, `rmp-serde` (MessagePack), `serde_json` (debug fallback)
- **Database:** `rusqlite` with `bundled-sqlcipher`, `refinery` (migrations)
- **Cryptography:** `argon2` (password hashing), `ed25519-dalek` (signature verification)
- **Time Management:** `chrono` + `chrono-tz` (IANA timezone support, DST-aware)
- **Logging:** `tracing`, `tracing-subscriber` (structured JSON logs)
- **IPC:** `interprocess` (Named Pipes on Windows, Unix Domain Sockets on Linux/macOS)

**Platform-Specific:**
- **Windows:** Windows Filtering Platform (WFP) callouts, Win32 process enumeration
- **Linux:** libbpf (eBPF), `fanotify` for execution monitoring, Unbound DNS RPZ
- **macOS:** Endpoint Security Framework (ESF), Network Extension (`NEDNSProxyProvider`)

### Frontend / UI (Tauri + React)
- **Framework:** Tauri v2.0, React 18, TypeScript 5.3
- **State Management:** Zustand 4.4 (lightweight, minimal boilerplate)
- **Data Fetching:** TanStack Query 5.17 (async state + caching)
- **Routing:** React Router v6
- **Styling:** Tailwind CSS 3.4
- **Charts:** Recharts 2.10 (quota/usage visualization)
- **Icons:** Lucide React 0.306

### Browser Extension (TypeScript)
- **Build Tool:** Webpack 5
- **Manifest:** V3 (Chrome/Edge), V2 (Firefox) with polyfills
- **Native Messaging:** Custom Rust-based host (stdin/stdout framing)
- **APIs:** `chrome.declarativeNetRequest`, `chrome.webRequest`, `chrome.storage`

### Android (Kotlin)
- **UI:** Jetpack Compose (Material 3)
- **Architecture:** MVVM (ViewModel + LiveData)
- **Services:** Foreground VPN Service, AccessibilityService
- **Build:** Gradle 8.2, Android Gradle Plugin 8.1
- **Min SDK:** 26 (Android 8.0 Oreo)

### Backend Server (Optional - Phase 5)
- **Framework:** Axum 0.7 (async HTTP/WebSocket)
- **Database:** PostgreSQL 16 (JSONB for flexible plan schemas)
- **ORM:** sqlx 0.7 (compile-time checked queries)
- **Auth:** JWT (bearer tokens) + Argon2id password hashing
- **Middleware:** `tower`, `tower-http` (CORS, rate limiting, tracing)

---

## 📋 System Requirements

### Development Environment
- **Operating System:** Windows 10+, macOS 13+, or Ubuntu 22.04+
- **Rust:** 1.75+ (stable toolchain)
- **Node.js:** 18 LTS or 20 LTS
- **Build Tools:**
  - **Windows:** Visual Studio Build Tools 2022, Windows SDK 10.0.22000+
  - **macOS:** Xcode 15+, Command Line Tools
  - **Linux:** clang, libbpf-dev, pkg-config, libssl-dev, unbound
  - **Android:** Android Studio, SDK 26+, NDK 25+

### Runtime Requirements
- **Privileges:** Administrator (Windows), root/sudo (Linux/macOS)
- **Special Permissions:**
  - **macOS:** Full Disk Access, System Extension approval (ESF)
  - **Android:** Accessibility Service, VPN, Usage Stats

---

## 🛠️ Setup & Installation

### 1. Clone the Repository

```bash
git clone https://github.com/pes1ug23am910/focusme.git
cd focusme
```

### 2. Build the Daemon (Core Service)

The daemon is the core enforcement service that runs as a privileged system service.

```bash
cd daemon
cargo build --release
# Output: target/release/focusme-daemon(.exe)
```

#### Install as System Service

**Windows (run as Administrator):**
```powershell
sc.exe create FocusMeDaemon binPath= "C:\Program Files\FocusMe\focusme-daemon.exe" start= auto
sc.exe start FocusMeDaemon
```

**Linux:**
```bash
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo cp linux/focusme.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now focusme.service
```

**macOS:**
```bash
sudo cp target/release/focusme-daemon /usr/local/bin/
sudo cp macos/com.focusme.daemon.plist /Library/LaunchDaemons/
sudo launchctl bootstrap system /Library/LaunchDaemons/com.focusme.daemon.plist
```

### 3. Build the Desktop UI (Tauri)

The Tauri UI provides a modern, cross-platform interface for managing plans.

```bash
cd ui
npm install
npm run tauri dev      # Development with hot-reload
npm run tauri build    # Production executable
```

**Build Outputs:**
- **Windows:** `ui/src-tauri/target/release/FocusMe.exe`
- **macOS:** `ui/src-tauri/target/release/bundle/macos/FocusMe.app`
- **Linux:** `ui/src-tauri/target/release/focusme-ui`

### 4. Build the Browser Extension

The extension coordinates with the daemon via Native Messaging Host.

```bash
cd extension
npm install

# Chrome/Edge (Manifest V3)
npm run build:mv3
# Output: extension/dist-chrome/

# Firefox (Manifest V2)
npm run build:mv2
# Output: extension/dist-firefox/
```

**Load Extension:**
- **Chrome/Edge:** Open `chrome://extensions/`, enable "Developer mode", click "Load unpacked", select `dist-chrome/`
- **Firefox:** Open `about:debugging#/runtime/this-firefox`, click "Load Temporary Add-on", select `dist-firefox/manifest.json`

### 5. Build the Android App (Optional)

```bash
cd android
./gradlew assembleDebug         # Debug APK
./gradlew assembleRelease       # Signed release (requires keystore)
```

**Output:** `android/app/build/outputs/apk/debug/app-debug.apk`

### 6. Build the Cloud Backend (Optional - Phase 5)

```bash
cd backend

# Start PostgreSQL
docker compose up -d postgres

# Configure environment
cp .env.example .env
# Edit .env: Set JWT_SECRET, DATABASE_URL, optional POSTHOG_API_KEY

# Run database migrations
cargo install sqlx-cli --features postgres --no-default-features
cargo sqlx migrate run

# Start server
cargo run --release
# Listens on http://0.0.0.0:8080
```

---

## 🧪 Testing

```bash
# Daemon unit + integration tests
cd daemon && cargo test

# Browser extension tests
cd extension && npm test

# UI component tests
cd ui && npm test

# Android unit tests
cd android && ./gradlew testDebugUnitTest

# Backend API tests
cd backend && cargo test
```

---

## 📁 Project Structure

```
focusme/
├── daemon/                       # Core enforcement service (Rust)
│   ├── src/
│   │   ├── main.rs              # Entry point, service setup
│   │   ├── db.rs                # SQLCipher policy store (1,062 lines)
│   │   ├── ipc_server.rs        # MessagePack IPC server (12 handlers)
│   │   ├── scheduler.rs         # DST-aware plan activation
│   │   ├── forced_mode.rs       # Dual-clock forced mode
│   │   ├── process_monitor.rs   # App blocking (Windows)
│   │   ├── hosts_manager.rs     # HOSTS file manipulation
│   │   ├── wfp_manager.rs       # Windows Filtering Platform
│   │   └── plan_protection.rs   # Argon2id password hashing
│   └── migrations/              # Refinery SQL migrations
│
├── backend/                      # Optional cloud server (Rust + Axum)
│   ├── src/
│   │   ├── main.rs              # HTTP server + WebSocket
│   │   ├── auth.rs              # JWT auth + Argon2id
│   │   ├── routes/              # Plan sync, family dashboard
│   │   └── analytics.rs         # PostHog telemetry
│   ├── migrations/              # PostgreSQL schema (sqlx)
│   └── openapi.yml              # API specification
│
├── extension/                    # Browser extension (TypeScript)
│   ├── src/
│   │   ├── background.ts        # Event page, NMH communication
│   │   ├── content.ts           # Content script injection
│   │   ├── popup.ts             # Extension popup UI
│   │   └── blocked.ts           # Block page
│   ├── native_messaging_host/   # Rust NMH binary
│   ├── manifest.v3.json         # Chrome/Edge manifest
│   └── manifest.v2.json         # Firefox manifest
│
├── ui/                           # Desktop UI (Tauri + React)
│   ├── src/
│   │   ├── components/          # React components
│   │   ├── pages/               # App pages (Dashboard, Plans, Settings)
│   │   ├── stores/              # Zustand state stores
│   │   └── lib/                 # Tauri IPC wrappers
│   └── src-tauri/               # Tauri Rust backend
│       └── src/lib.rs           # IPC bridge to daemon
│
├── android/                      # Android app (Kotlin)
│   └── app/src/main/
│       ├── java/com/focusme/
│       │   ├── services/        # VPN, Accessibility, Foreground
│       │   ├── ui/              # Jetpack Compose screens
│       │   └── data/            # ViewModel, Repository
│       └── res/                 # Resources, layouts
│
├── macos/                        # macOS-specific (Swift)
│   └── FocusMeESF/              # System Extension + DNS Proxy
│       ├── ESFClient.swift      # Endpoint Security Framework
│       └── DNSProxy.swift       # NEDNSProxyProvider
│
├── linux/                        # Linux-specific (Rust + eBPF)
│   ├── bpf/                     # eBPF C programs (CO-RE)
│   │   ├── block_exec.bpf.c    # LSM hook for exec blocking
│   │   └── dns_filter.bpf.c    # Network filter
│   └── src/                     # Rust loader + Fanotify fallback
│
├── packaging/                    # Platform installers
│   ├── windows/                 # WiX v4 MSI installer
│   ├── linux/                   # Debian .deb + RPM
│   └── macos/                   # PKG installer
│
├── docs/                         # Documentation
│   ├── ARCHITECTURE.md          # System architecture deep-dive
│   ├── ipc_protocol_v1.md       # IPC message specification
│   ├── security_review.md       # Threat model + pen test plan
│   ├── bypass_tests.md          # 12 bypass test procedures
│   ├── decisions.md             # Architecture decision log (D-001–D-013)
│   └── store_submissions/       # App store submission guides
│
├── .gitignore                   # Production-grade exclusion list
├── CHANGELOG.md                 # Version history
├── CONTRIBUTING.md              # Developer guide
├── README.md                    # This file
├── LICENSE                      # License file
├── privacy_policy.md            # GDPR/CCPA privacy policy
├── tos.md                       # Terms of Service
└── eula.md                      # End User License Agreement
```

---

## 📖 Documentation

| Document | Description |
|----------|-------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Detailed system architecture, data flows, enforcement matrix |
| [docs/ipc_protocol_v1.md](docs/ipc_protocol_v1.md) | IPC message format and handler specification |
| [docs/policy_schema_v1.json](docs/policy_schema_v1.json) | JSON Schema for plan policies |
| [docs/security_review.md](docs/security_review.md) | Threat model, OWASP mapping, penetration testing plan |
| [docs/bypass_tests.md](docs/bypass_tests.md) | 12 bypass test procedures with expected outcomes |
| [docs/decisions.md](docs/decisions.md) | Architecture Decision Records (D-001 through D-013) |
| [docs/performance_benchmarks.md](docs/performance_benchmarks.md) | Performance targets and benchmark scripts |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development setup, code style, commit format, PR checklist |
| [CHANGELOG.md](CHANGELOG.md) | Version history (Keep a Changelog format) |

---

## 🎓 Skills Demonstrated (For Recruiters)

This project showcases a wide range of advanced software engineering skills:

### System Programming & OS Internals
- **Kernel-Level Development:** WFP drivers (Windows), eBPF programs (Linux), ESF (macOS)
- **Process Management:** Process enumeration, termination, execution monitoring
- **Network Stack:** DNS filtering, packet inspection, VPN implementation
- **IPC Mechanisms:** Named Pipes, Unix Domain Sockets, Native Messaging Protocol

### Cross-Platform Development
- **5 Platforms:** Windows, macOS, Linux, Android, Web (browser extensions)
- **Platform Abstraction:** Conditional compilation, platform-specific modules
- **Unified Codebase:** Monorepo structure with shared business logic

### Security Engineering
- **Cryptography:** AES-256 (SQLCipher), Argon2id (password hashing), Ed25519 (signatures)
- **Threat Modeling:** 12 documented bypass attempts with mitigations
- **Anti-Tamper:** Process protection, file system monitoring, integrity checks
- **Secure Architecture:** Defense in depth, least privilege, fail-secure design

### Software Architecture
- **Layered Architecture:** Clear separation of concerns (UI, Business Logic, Enforcement)
- **Design Patterns:** Repository pattern, Command pattern (IPC), Observer pattern (scheduler)
- **Scalability:** Async/await, concurrent data structures, database connection pooling
- **Extensibility:** Plugin-style enforcement engines, protocol versioning

### Modern Development Practices
- **Type Safety:** Rust (memory safety), TypeScript (type-checking)
- **Testing:** Unit tests, integration tests, bypass scenario tests
- **Documentation:** Architecture docs, API specs (OpenAPI), code comments
- **Version Control:** Git, conventional commits, structured branching

### Full-Stack Capabilities
- **Backend:** Rust (Axum framework), PostgreSQL, REST APIs, WebSocket
- **Frontend:** React, TypeScript, Tailwind CSS, responsive design
- **Mobile:** Android (Kotlin), Jetpack Compose, MVVM architecture
- **DevOps:** Docker, systemd services, Windows Services, LaunchDaemons

### Problem-Solving & Research
- **Complex Problem Domain:** Multi-platform enforcement with minimal user friction
- **Research-Driven:** Studied platform security APIs, kernel development, anti-cheat systems
- **Iterative Refinement:** Multiple design iterations based on bypass testing

---

## ⚠️ Important Notes

### Large Assets Not Included
This repository contains **source code only**. The following assets are excluded via `.gitignore`:
- Build artifacts (`target/`, `node_modules/`, `build/`)
- Compiled binaries (`.exe`, `.apk`, `.dmg`, `.msi`)
- Database files (`.db`, `.sqlite`)
- IDE configurations
- Secrets and credentials

To build the project, follow the [Setup & Installation](#️-setup--installation) instructions above.

### Academic Project Disclaimer
This is a portfolio project developed for educational purposes and placement demonstrations. It showcases advanced software engineering skills but is not intended for commercial distribution without proper security audits and legal compliance reviews.

---

## 📧 Contact

**Yash Verma**  
SRN: PES1UG23AM910  
GitHub: [@pes1ug23am910](https://github.com/pes1ug23am910)

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- **Rust Community:** For excellent documentation and crates ecosystem
- **Tauri Team:** For making cross-platform desktop development accessible
- **Platform Documentation:** Windows WFP, Linux eBPF, macOS ESF documentation

---

**Built with ❤️ using Rust, TypeScript, and Kotlin**
