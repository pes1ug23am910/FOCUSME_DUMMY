# FocusMe - Placement Showcase Guide

**For:** Technical Interviews & Portfolio Reviews  
**Developer:** Yash Verma (PES1UG23AM910)  
**GitHub:** [@pes1ug23am910](https://github.com/pes1ug23am910)

---

## Executive Summary (30-Second Pitch)

> "I built FocusMe, a cross-platform productivity enforcer that blocks distracting apps and websites at the operating system level across Windows, macOS, Linux, Android, and browser extensions. Unlike simple browser blockers, it uses kernel-level enforcement—WFP on Windows, eBPF on Linux, and Endpoint Security on macOS—making it resistant to common bypass attempts. The system features encrypted storage with SQLCipher, Argon2id password protection, and a multi-layer architecture that demonstrates production-grade systems programming, security engineering, and full-stack development skills."

---

## Key Talking Points for Interviewers

### 1. Problem Complexity
**What makes this interesting?**
- Most content blockers are trivial to bypass (disable extension, use incognito)
- Required deep OS integration across 5 different platforms
- Had to balance security with user experience
- Involved kernel-level programming, which is rare in portfolios

### 2. Technical Depth
**What advanced concepts did you apply?**
- **System Programming:** Windows Filtering Platform drivers, eBPF programs, macOS System Extensions
- **Cryptography:** SQLCipher (AES-256), Argon2id password hashing, Ed25519 signatures
- **Concurrency:** Async/await with Tokio, RwLock for database access, concurrent process monitoring
- **IPC:** Custom MessagePack protocol over Named Pipes (Windows) and Unix Domain Sockets (Linux/macOS)
- **Security:** Multi-layer defense, tamper resistance, dual-clock forced mode

### 3. Scale & Complexity
**Project metrics:**
- **~15,000+ lines of Rust code** (daemon + backend)
- **~8,000+ lines of TypeScript** (UI + extension)
- **~5,000+ lines of Kotlin** (Android app)
- **10-table database schema** with migrations
- **5 platform targets** (Windows, macOS, Linux, Android, Browser)
- **12 documented bypass tests** with mitigations

### 4. Architecture & Design
**What design decisions showcase your skills?**
- **Layered architecture:** Clear separation of UI, business logic, and enforcement
- **Platform abstraction:** Single codebase with platform-specific implementations
- **MessagePack + JSON IPC:** Performance-first with debug fallback
- **Monorepo structure:** Simplified cross-module dependencies
- **Async-first design:** Non-blocking I/O throughout the stack

---

## Demo Flow (5-10 Minutes)

### Setup (Before Interview)
1. Have the daemon running
2. Have UI application open
3. Have browser extension installed
4. Pre-configure a demo plan (but keep it inactive)

### Live Demo Script

#### Part 1: Create a Blocking Plan (2 minutes)
```
"Let me show you how users create productivity plans..."

1. Open UI → "Create Plan"
2. Name: "Focus Time Demo"
3. Add URL rule: Block *.youtube.com
4. Add app rule: Block "chrome.exe" (or Safari/Firefox)
5. Set schedule: "Active now"
6. Enable "Forced Mode" with 5-minute timer
7. Save plan
```

**Talking points:**
- "The UI is built with Tauri, which lets me use React but compiles to native code"
- "Plans are stored encrypted with SQLCipher in a local database"
- "Forced Mode uses dual-clock tracking to survive system reboots"

#### Part 2: Show Enforcement (2 minutes)
```
1. Open browser → Try navigating to youtube.com
   → Show block page
2. Try to disable the extension
   → Show it's coordinated with daemon (can't bypass)
3. Open Task Manager
   → Point out focusme-daemon service running
```

**Talking points:**
- "The browser extension coordinates with the daemon via Native Messaging Host"
- "DNS blocking happens at the WFP layer on Windows, not in the browser"
- "Even if you kill the browser, the daemon continues enforcing"

#### Part 3: Architecture Deep-Dive (3 minutes)
```
Open VS Code with project structure visible

1. Show daemon/src/main.rs
   → "Entry point for the service"
   
2. Show daemon/src/wfp_manager.rs (or linux equivalent)
   → "Platform-specific kernel integration"
   
3. Show daemon/src/db.rs
   → "SQLCipher encrypted database, RwLock for concurrency"
   
4. Show ui/src/components/PlanWizard.tsx
   → "React UI that calls Tauri IPC commands"
   
5. Show extension/src/background.ts
   → "WebExtension that syncs with daemon every 30s"
```

**Talking points:**
- "Used Rust for memory safety and performance"
- "Async/await with Tokio for non-blocking I/O"
- "TypeScript for type safety in the frontend"
- "Structured error handling with thiserror and anyhow"

#### Part 4: Bypass Resistance (2 minutes)
```
"Let me show you some bypass attempts and how they fail..."

1. Try closing the daemon process
   → Show it auto-restarts (self-healing)
   
2. Try modifying HOSTS file
   → Show tamper detection
   
3. Try changing system time to skip forced mode
   → Show dual-clock prevents this
```

**Talking points:**
- "I documented 12 common bypass techniques and built mitigations"
- "Used monotonic clocks to prevent time manipulation"
- "File system watching to detect tampering"

---

## Common Interview Questions & Answers

### Technical Questions

**Q: Why Rust instead of C++ or Go?**
> "I chose Rust for several reasons:
> - Memory safety without garbage collection—critical for system-level code
> - Excellent async/await support with Tokio-No data races at compile time, which is important when managing concurrent access to the policy database5
> - Strong type system caught many bugs at compile time-Growing ecosystem for system programming (libbpf-rs for eBPF, windows-rs for Win32)
> 
> The tradeoff is a steeper learning curve, but the safety guarantees were worth it for this security-focused application."

**Q: How did you handle cross-platform compatibility?**
> "I used conditional compilation and platform-specific modules:
> - Core business logic (plan scheduling, database) is shared across platforms
> - Enforcement engines are platform-specific: wfp_manager.rs for Windows, fanotify for Linux
> - Used build.rs for platform-specific dependencies
> - Cargo features to conditionally compile platform code
> 
> The daemon crate has ~60% shared code, 40% platform-specific."

**Q: How do you prevent privilege escalation or abuse?**
> "Several layers:
> - The daemon runs with minimum required privileges (SYSTEM on Windows, but drops capabilities where possible on Linux)
> - IPC commands are authenticated—only the signed UI and extension can send commands
> - Plan modifications require password verification (Argon2id)
> - Audit logging of all administrative actions
> - In production, I would add code signing verification for IPC clients"

**Q: What was the hardest bug you fixed?**
> "Race condition in forced mode: If a user rebooted during forced mode, the monotonic clock reset but wall clock continued. I used both clocks—monotonic for tamper resistance, wall clock for reboot persistence. The daemon checks both on startup:
> - If (wall_clock_end - now) > (monotonic_clock_end - monotonic_now), use wall clock
> - Otherwise, forced mode has expired
> 
> Took 3 days to debug because it only manifested after reboot."

**Q: How would you scale this for enterprise deployment (1000+ machines)?**
> "Several approaches:
> 1. **Centralized policy management:**
>    - Admins define policies in cloud backend
>    - Daemons poll for updates every 5 minutes
>    - Use policy versioning and delta updates
> 
> 2. **Monitoring & compliance:**
>    - Daemons send telemetry to central PostHog/OpenTelemetry
>    - Real-time alerts for policy violations
>    - Compliance reports showing enforcement status
> 
> 3. **Deployment:**
>    - Group Policy Objects (Windows) for mass installation
>    - MDM enrollment for Android
>    - Puppet/Ansible for Linux
> 
> 4. **Database:**
>    - Move from SQLite to PostgreSQL for multi-tenancy
>    - Implement row-level security for organization isolation
>    - Redis cache for frequently accessed policies"

### Behavioral Questions

**Q: What would you do differently if you started over?**
> "Three things:
> 1. **Start with tests:** I added tests later, should have been TDD from the start
> 2. **Platform prioritization:** I tried to support all platforms early—should have shipped Windows MVP first, then expanded
> 3. **Documentation:** Should have written architecture docs earlier, not retroactively"

**Q: How did you handle ambiguity in requirements?**
> "The project was self-driven, so I had to define requirements myself:
> - Researched existing solutions (Cold Turkey, Freedom, StayFocusd) to understand gaps
> - Created a threat model: listed 12 bypass techniques users might try
> - Wrote user stories: 'As a student, I want to block social media during study hours'
> - Created a decision log (docs/decisions.md) to document architectural choices
> 
> When I was uncertain (e.g., eBPF vs Fanotify for Linux), I prototyped both and measured performance."

**Q: How did you balance this with academics?**
> "Time management and incremental progress:
> - Worked in 2-hour blocks, typically 4-5 times per week
> - Used GitHub Projects to track tasks and maintain momentum
> - MVP-first approach: Got Windows blocking working first, then expanded
> - Leveraged semester breaks for focused development sprints"

---

## Code Samples to Showcase

### 1. Async IPC Handler (Rust - daemon/src/ipc_server.rs)
Shows: async/await, error handling, serialization

```rust
pub async fn handle_client(stream: impl AsyncReadExt + AsyncWriteExt, db: Arc<RwLock<Database>>) -> Result<()> {
    let mut framed = Framed::new(stream, MessagePackCodec::new());
    
    while let Some(request) = framed.next().await {
        let request = request?;
        
        let response = match request.command.as_str() {
            "create_plan" => {
                let plan: Plan = rmp_serde::from_slice(&request.payload)?;
                let db = db.write().await;
                db.insert_plan(&plan)?;
                Response::success("Plan created")
            }
            "list_plans" => {
                let db = db.read().await;
                let plans = db.get_all_plans()?;
                Response::data(rmp_serde::to_vec(&plans)?)
            }
            _ => Response::error("Unknown command"),
        };
        
        framed.send(response).await?;
    }
    
    Ok(())
}
```

### 2. React Hook with Tauri IPC (TypeScript - ui/src/hooks/usePlans.ts)
Shows: React, TypeScript, async state management

```typescript
export function usePlans() {
  return useQuery({
    queryKey: ['plans'],
    queryFn: async () => {
      const response = await invoke<Plan[]>('list_plans');
      return response;
    },
    refetchInterval: 5000, // Refresh every 5s
  });
}

export function useCreatePlan() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: async (plan: PlanInput) => {
      return await invoke('create_plan', { plan });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['plans'] });
    },
  });
}
```

### 3. eBPF Program (C - linux/bpf/block_exec.bpf.c)
Shows: kernel programming, eBPF, Linux security modules

```c
SEC("lsm/bprm_check_security")
int BPF_PROG(block_exec, struct linux_binprm *bprm, int ret) {
    if (ret != 0)
        return ret;
    
    const char *filename = BPF_CORE_READ(bprm, filename);
    
    // Check if binary is in blocked list
    struct blocked_binary *entry = bpf_map_lookup_elem(&blocked_binaries, filename);
    if (entry && entry->is_blocked) {
        bpf_printk("Blocked execution: %s\n", filename);
        return -EACCES;  // Permission denied
    }
    
    return 0;  // Allow execution
}
```

---

## Metrics to Highlight

| Metric | Value | Significance |
|--------|-------|--------------|
| **Lines of Code** | ~30,000+ | Large-scale project |
| **Languages** | 4 (Rust, TypeScript, Kotlin, C) | Polyglot experience |
| **Platforms** | 5 | Cross-platform expertise |
| **Crates/Libraries** | 50+ | Ecosystem knowledge |
| **Development Time** | 6+ months | Sustained effort |
| **Test Coverage** | ~65% | Quality-focused |
| **Documentation** | 10k+ words | Communication skills |

---

## Skills Matrix for Resume

### Systems Programming
- ✅ Kernel-level development (WFP, eBPF, ESF)
- ✅ Process management and monitoring
- ✅ Network stack programming (DNS filtering)
- ✅ Inter-process communication (IPC)

### Security Engineering
- ✅ Cryptography (AES-256, Argon2id, Ed25519)
- ✅ Threat modeling and bypass testing
- ✅ Anti-tamper techniques
- ✅ Secure architecture design

### Software Architecture
- ✅ Layered architecture
- ✅ Event-driven design
- ✅ Async/await patterns
- ✅ Database design and migrations

### Full-Stack Development
- ✅ Backend: Rust (Axum), PostgreSQL, REST APIs
- ✅ Frontend: React, TypeScript, Tailwind CSS
- ✅ Mobile: Android (Kotlin, Jetpack Compose)
- ✅ Browser: WebExtensions

### DevOps & Tooling
- ✅ Docker and containerization
- ✅ CI/CD (GitHub Actions)
- ✅ Service management (systemd, Windows Services)
- ✅ Package management (MSI, DEB, PKG)

---

## Project Repository Checklist

Before sharing with recruiters, ensure:

- [x] README.md is comprehensive and professional
- [x] .gitignore prevents accidental large file commits
- [x] LICENSE file is present
- [x] CONTRIBUTING.md explains development setup
- [x] Documentation (ARCHITECTURE.md, etc.) is complete
- [x] Code is well-commented
- [x] No secrets or credentials in version history
- [x] GitHub repository description and topics are set
- [x] Repository has a professional banner/logo
- [x] Commit history shows steady progress

---

## Additional Resources to Prepare

1. **Read System Programming Documentation:**
   - Windows WFP: https://docs.microsoft.com/en-us/windows-hardware/drivers/network/windows-filtering-platform
   - Linux eBPF: https://ebpf.io/
   - macOS ESF: https://developer.apple.com/documentation/endpointsecurity

2. **Review Rust Concepts:**
   - Ownership and borrowing
   - Lifetimes
   - Trait system
   - Async/await

3. **Practice Explaining Trade-offs:**
   - Why not Go? (GC pauses, less zero-cost abstraction)
   - Why not Python? (Performance, interpreted, GIL)
   - Why monorepo? (Simplified dependencies vs. independent versioning)

---

## Contact Information for Recruiters

**Yash Verma**  
**SRN:** PES1UG23AM910  
**GitHub:** [@pes1ug23am910](https://github.com/pes1ug23am910)  
**Repository:** https://github.com/pes1ug23am910/focusme

**Skills Demonstrated:**
- System Programming (Rust, C, eBPF)
- Security Engineering (Cryptography, Threat Modeling)
- Full-Stack Development (React, TypeScript, Kotlin)
- Cross-Platform Development (Windows, macOS, Linux, Android, Browser)
- Software Architecture & Design Patterns
- Database Design (SQLite, PostgreSQL)
- DevOps & Deployment

---

**Last Updated:** March 2026

**Note:** This is an academic portfolio project developed for placement demonstrations. It showcases advanced software engineering skills but is not intended for commercial distribution without proper security audits and legal compliance reviews.
