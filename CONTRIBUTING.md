# Contributing to FocusMe

Thank you for your interest in contributing to FocusMe! This document provides guidelines and instructions for contributing to the project.

---

## Table of Contents

- [Development Environment Setup](#development-environment-setup)
- [Code Style](#code-style)
- [Commit Format](#commit-format)
- [Branch Naming](#branch-naming)
- [Pull Request Checklist](#pull-request-checklist)
- [Adding a New Rule Type](#adding-a-new-rule-type)
- [Security Disclosures](#security-disclosures)

---

## Development Environment Setup

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust (stable) | 1.75+ | Daemon, NMH, Linux eBPF userspace |
| Node.js | 20 LTS | Browser extension, Tauri UI |
| pnpm | 8+ | Package manager (extension + UI) |
| Android Studio | 2023.1+ | Android app (Kotlin + Compose) |
| Xcode | 15+ | macOS System Extension + DNS Proxy |
| Docker | 24+ | Optional — local Postgres for Phase 5 backend |

### Clone & Build

```bash
git clone https://github.com/focusme/focusme.git
cd focusme

# Daemon (Windows/Linux)
cd daemon && cargo build

# Browser extension
cd extension && pnpm install && pnpm build

# Desktop UI (Tauri)
cd ui && pnpm install && pnpm tauri build

# Android
cd android && ./gradlew assembleDebug

# macOS
open macos/FocusMe.xcodeproj
# Build via Xcode (requires signing entitlements)
```

### Running Tests

```bash
# Daemon unit tests
cd daemon && cargo test

# Extension
cd extension && pnpm test

# UI
cd ui && pnpm test

# Android
cd android && ./gradlew test
```

---

## Code Style

### Rust (Daemon, NMH, Linux)

- **Formatter:** `cargo fmt` (default rustfmt config)
- **Linter:** `cargo clippy -- -D warnings`
- **Naming:** `snake_case` for functions/variables, `PascalCase` for types/traits
- **Error handling:** Use `anyhow::Result` for application errors, `thiserror` for library errors
- **Logging:** Use `tracing` macros (`info!`, `warn!`, `error!`, `debug!`) — never `println!`
- **Async:** `tokio` runtime; prefer `async fn` over `spawn` where possible
- **Comments:** Every public function needs a `///` doc comment
- **File headers:** Use the standard header block (see existing files for template)

### TypeScript (Extension, UI)

- **Formatter:** Prettier (default config)
- **Linter:** ESLint with recommended rules
- **Naming:** `camelCase` for functions/variables, `PascalCase` for components/types
- **Imports:** Prefer named imports, group by: external → internal → relative
- **Async:** `async/await` over `.then()` chains

### Kotlin (Android)

- **Formatter:** ktlint
- **Naming:** Kotlin conventions (`camelCase` vars, `PascalCase` classes)
- **Coroutines:** Use structured concurrency with `viewModelScope` / `lifecycleScope`

### Swift (macOS)

- **Formatter:** SwiftFormat (default rules)
- **Naming:** Swift API Design Guidelines

---

## Commit Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Use When |
|------|----------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code change that neither fixes nor adds |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `chore` | Build, CI, dependency updates |
| `security` | Security-related change |

### Scopes

| Scope | Component |
|-------|-----------|
| `daemon` | Rust daemon (Windows/Linux) |
| `extension` | Browser extension (MV3/MV2) |
| `ui` | Tauri desktop UI |
| `android` | Android app |
| `macos` | macOS System Extension + DNS Proxy |
| `linux` | Linux-specific (eBPF, Fanotify) |
| `ci` | GitHub Actions workflows |
| `docs` | Documentation |
| `packaging` | Installers, signing, distribution |

### Examples

```
feat(daemon): add RwLock for database connection (D-013)
fix(extension): handle NMH disconnection during sync
docs: update ARCHITECTURE.md with forced mode design
test(daemon): expand scheduler unit tests for DST handling
security(daemon): migrate Mutex to RwLock for read concurrency
chore(ci): add cargo audit to security pipeline
```

---

## Branch Naming

```
<type>/<ticket-or-short-description>
```

### Examples

```
feat/rwlock-migration
fix/nmh-reconnect-backoff
docs/architecture-reference
test/scheduler-dst-tests
chore/dependabot-config
```

### Protected Branches

| Branch | Protection |
|--------|-----------|
| `main` | Require PR, 1 approval, CI pass, no force push |
| `release/*` | Same as main + require signed commits |

---

## Pull Request Checklist

Before submitting a PR, verify:

- [ ] **Code compiles** — `cargo build` / `pnpm build` / `./gradlew assembleDebug`
- [ ] **Tests pass** — `cargo test` / `pnpm test`
- [ ] **Linter clean** — `cargo clippy -- -D warnings` / `pnpm lint`
- [ ] **Formatted** — `cargo fmt --check` / `pnpm format:check`
- [ ] **No new warnings** — check CI output
- [ ] **Documentation updated** — if adding/changing public API, update docs
- [ ] **Decision logged** — if making an architectural choice, add to `docs/decisions.md`
- [ ] **Session log updated** — add entry to `docs/session_log.md`
- [ ] **CHANGELOG updated** — add entry under `[Unreleased]`
- [ ] **Security reviewed** — if touching auth, crypto, or enforcement logic, review `docs/security_review.md`

### PR Template

```markdown
## Summary
<!-- One-paragraph description of what this PR does -->

## Task Reference
<!-- T-XXX from the build plan, or "N/A" for ad-hoc work -->

## Type of Change
- [ ] Feature
- [ ] Bug fix
- [ ] Documentation
- [ ] Refactor
- [ ] Test
- [ ] CI/CD

## Testing
<!-- How was this tested? Which test cases cover it? -->

## Security Impact
<!-- Does this change affect authentication, encryption, enforcement, or anti-circumvention? If yes, explain. -->
```

---

## Adding a New Rule Type

FocusMe supports app rules and URL rules. To add a new rule type (e.g., keyword rules, category rules):

### 1. Database Schema

Add a new migration in `daemon/src/db.rs` → `run_migrations()`:

```sql
CREATE TABLE IF NOT EXISTS new_rules (
    id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL REFERENCES plans(plan_id) ON DELETE CASCADE,
    match_type TEXT NOT NULL,  -- e.g., 'keyword', 'category'
    value TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 1,
    is_allow INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 2. Database CRUD

Add methods to `Database` in `daemon/src/db.rs`:
- `save_new_rules(plan_id: &str, rules: &[NewRule]) -> Result<()>`
- `get_new_rules(plan_id: &str) -> Result<Vec<NewRule>>`

### 3. IPC Handlers

Add message types in `daemon/src/ipc_server.rs`:
- `handle_new_rule_check()` — evaluate if content matches a rule
- Update `handle_plan_create()` / `handle_plan_update()` to include new rules

### 4. Enforcement Integration

- Update `scheduler.rs` `LoadedPlan` struct to include new rule type
- Add matching logic in the appropriate platform enforcement module
- For URL-type rules: update `hosts_manager.rs` or extension DNR rules
- For app-type rules: update `process_monitor.rs`

### 5. UI Integration

- Add rule editor component in `ui/src/components/`
- Update plan wizard to include new rule type step
- Add IPC invoke for the new rule check

### 6. Testing

- Add unit tests in `daemon/src/tests/`
- Add integration test scenario in `docs/bypass_tests.md`
- Update `docs/policy_schema_v1.json` with new rule type schema

### 7. Documentation

- Update `docs/ARCHITECTURE.md` database schema section
- Update `docs/ipc_protocol_v1.md` with new message types
- Add decision record in `docs/decisions.md`
- Log in `docs/session_log.md`

---

## Security Disclosures

**Do NOT open public issues for security vulnerabilities.**

FocusMe is a security-critical application. If you discover a vulnerability:

1. **Email:** security@focusme.com
2. **Subject:** `[SECURITY] Brief description`
3. **Include:**
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested fix (if you have one)

### Response Timeline

| Action | Timeline |
|--------|----------|
| Acknowledgment | Within 48 hours |
| Initial assessment | Within 1 week |
| Fix development | Within 2 weeks (critical) / 4 weeks (high) |
| Disclosure | Coordinated, after fix is released |

### Scope

Security-relevant areas include:
- **Plan protection bypass** — any way to modify/delete a protected plan without the password
- **Forced Mode bypass** — any way to end a locked session early
- **Clock manipulation** — defeating the dual-clock anti-circumvention
- **HOSTS file bypass** — preventing tamper detection from restoring entries
- **IPC injection** — sending unauthorized commands to the daemon
- **Database decryption** — extracting the SQLCipher key
- **Extension bypass** — disabling URL blocking without uninstalling

See `docs/security_review.md` for the full threat model and `docs/bypass_tests.md` for known bypass test scenarios.

---

## License

By contributing, you agree that your contributions will be licensed under the project's license.

See [LICENSE](../LICENSE) for details.
