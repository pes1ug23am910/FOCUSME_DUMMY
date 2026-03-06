# FocusMe — Bypass Test Matrix (BT-01 through BT-12)

> **FILE:** docs/bypass_tests.md
> **TASK:** T-050
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **PURPOSE:** Executable test procedures for all 12 bypass scenarios from build plan §6.3.
> **USAGE:** Execute each test on real hardware or VMs. Fill "Actual Result" during QA.

---

## Overview

These 12 tests verify that FocusMe's enforcement mechanisms resist common bypass attempts. Some bypasses are **documented limitations** (hardware-level escape vectors that no userspace software can prevent). These are marked explicitly.

**Test environment requirements:**
- Windows 10/11 Pro x64 with FocusMe MSI installed
- macOS 13+ with FocusMe PKG installed (ESF entitlement active)
- Ubuntu 22.04 LTS with FocusMe .deb installed
- Android 12+ device with FocusMe APK installed
- Chrome/Firefox/Edge with FocusMe extension loaded

**Tester account:** Standard (non-admin) user unless otherwise specified.

---

## BT-01: Windows Safe Mode

| Field | Value |
|-------|-------|
| **Test ID** | BT-01 |
| **Name** | Windows Safe Mode Bypass |
| **Platform(s)** | Windows 10/11 |
| **Category** | Documented Limitation |

### Setup
- Windows 10/11 Pro with FocusMe daemon running as a Windows Service
- At least one blocking plan active (block `spotify.exe`)
- Standard user account (non-admin)
- Verify `focusme-daemon` service is running: `sc query focusme-daemon`

### Steps
1. Open an elevated Command Prompt (admin)
2. Run: `bcdedit /set {current} safeboot minimal`
3. Restart the computer
4. After boot into Safe Mode, open Task Manager
5. Check if `focusme-daemon` service is running
6. Attempt to launch `spotify.exe`
7. Exit Safe Mode: `bcdedit /deletevalue {current} safeboot` → reboot

### Expected Result
- **Safe Mode disables non-Microsoft services.** FocusMe daemon will NOT be running.
- Spotify will launch successfully.
- This is a **documented limitation** — Safe Mode is a hardware-level escape.
- After normal reboot, FocusMe service resumes automatically.

### Actual Result
- [ ] PASS (daemon not running in Safe Mode = expected)
- [ ] FAIL — describe unexpected behavior

### Mitigation Notes
- Recommend BIOS/UEFI password to prevent Safe Mode boot by casual users
- Document in user FAQ: "Safe Mode bypasses all third-party software"
- Enterprise: GPO can restrict Safe Mode access (requires domain admin)

---

## BT-02: Process Kill via Task Manager

| Field | Value |
|-------|-------|
| **Test ID** | BT-02 |
| **Name** | Process Kill via Task Manager (Standard User) |
| **Platform(s)** | Windows 10/11 |
| **Category** | Must Block |

### Setup
- FocusMe daemon running as SYSTEM service
- Standard user account logged in (no admin elevation)
- At least one active blocking plan
- Verify daemon is running: `tasklist /FI "IMAGENAME eq focusme-daemon.exe"`

### Steps
1. Open Task Manager (Ctrl+Shift+Esc)
2. Go to "Details" tab
3. Find `focusme-daemon.exe`
4. Right-click → "End task"
5. Observe the result
6. If process terminated, wait 10 seconds and check if it auto-restarts
7. Repeat with `taskkill /F /IM focusme-daemon.exe` from non-elevated CMD

### Expected Result
- Task Manager shows **"Access Denied"** or **"Unable to terminate process"**
- `taskkill` returns: `ERROR: Access is denied.`
- The daemon process continues running throughout
- Service recovery (sc.exe failure action) would restart it even if kill succeeded

### Actual Result
- [ ] PASS — Access Denied for standard user
- [ ] FAIL — describe what happened

### Notes
- Admin users CAN kill the service (this is expected and acceptable)
- Windows Service Control Manager auto-restarts on failure (configured in WiX)

---

## BT-03: HOSTS File Edit by Non-Admin

| Field | Value |
|-------|-------|
| **Test ID** | BT-03 |
| **Name** | HOSTS File Tampering Detection |
| **Platform(s)** | Windows 10/11, Linux |
| **Category** | Must Detect + Restore |

### Setup
- FocusMe daemon running with active DNS blocking plan (block `facebook.com`)
- Verify HOSTS file contains FocusMe entries:
  - Windows: `type C:\Windows\System32\drivers\etc\hosts`
  - Linux: `cat /etc/hosts`
- Note the SHA-256 hash the daemon is monitoring

### Steps
1. **Non-admin attempt:**
   - Try to edit HOSTS file with notepad (non-elevated): `notepad C:\Windows\System32\drivers\etc\hosts`
   - Attempt to save changes
   - Observe result

2. **Admin tampering (detection test):**
   - Open elevated notepad / `sudo nano /etc/hosts`
   - Remove FocusMe blocking entries
   - Save the file
   - Wait 5 seconds (daemon polls every 2s)
   - Check HOSTS file again

### Expected Result
1. Non-admin: Save fails with "Access Denied" (Windows) or "Permission denied" (Linux)
2. Admin edit detected:
   - Daemon detects SHA-256 hash mismatch within 2-5 seconds
   - Daemon **automatically restores** the HOSTS file with blocking entries
   - Tracing log shows: `"HOSTS file tampering detected — restoring"`
   - `facebook.com` remains blocked

### Actual Result
- [ ] PASS — Non-admin blocked + admin edit auto-restored within 5s
- [ ] FAIL — describe what happened

### Notes
- The 2-second poll interval is governed by `hosts_manager.rs` (D-010)
- A brief window (up to 2s) exists where HOSTS is tampered — acceptable

---

## BT-04: Alternate Browser

| Field | Value |
|-------|-------|
| **Test ID** | BT-04 |
| **Name** | Alternate Browser Bypass Attempt |
| **Platform(s)** | Windows, macOS, Linux |
| **Category** | Must Block |

### Setup
- FocusMe blocking plan active for `facebook.com`, `reddit.com`
- FocusMe extension installed in Chrome
- Install an alternate browser WITHOUT FocusMe extension (e.g., Brave, Vivaldi, curl)
- WFP (Windows), DNS proxy (macOS), or DNS blocker (Linux) active

### Steps
1. Open Chrome → navigate to `facebook.com` → verify blocked
2. Open alternate browser (Brave/Vivaldi) → navigate to `facebook.com`
3. Run from terminal: `curl -I https://facebook.com`
4. Run: `nslookup facebook.com`
5. On Windows: try `Invoke-WebRequest https://facebook.com` in PowerShell

### Expected Result
- **All browsers and HTTP clients are blocked** — enforcement is at the network layer
- Windows: WFP filter drops packets to resolved IPs. HOSTS returns 0.0.0.0
- macOS: DNS proxy returns NXDOMAIN for blocked domains
- Linux: Unbound RPZ returns NXDOMAIN, or HOSTS returns 0.0.0.0
- `curl` should fail with connection refused or DNS resolution failure
- `nslookup` should return 0.0.0.0 (HOSTS) or NXDOMAIN (DNS proxy/Unbound)

### Actual Result
- [ ] PASS — all alternate browsers/clients blocked
- [ ] FAIL — describe which client bypassed

### Notes
- The extension provides UI (block page). Network-layer blocking provides enforcement.
- Without extension: user sees generic browser error (e.g., ERR_NAME_NOT_RESOLVED) instead of styled block page

---

## BT-05: DNS-over-HTTPS (DoH) Bypass

| Field | Value |
|-------|-------|
| **Test ID** | BT-05 |
| **Name** | DNS-over-HTTPS Bypass Attempt |
| **Platform(s)** | Windows (WFP), macOS (DNS proxy), Linux (iptables/WFP) |
| **Category** | Must Block |

### Setup
- FocusMe blocking plan active for `facebook.com`
- WFP manager loaded (Windows) — verify 14 DoH provider IPs are blocked:
  - Google: `8.8.8.8`, `8.8.4.4`
  - Cloudflare: `1.1.1.1`, `1.0.0.1`
  - Quad9: `9.9.9.9`, `149.112.112.112`
  - OpenDNS: `208.67.222.222`, `208.67.220.220`
  - And 6 more from `wfp_manager.rs`

### Steps
1. Open Chrome → Settings → Privacy → Security → "Use secure DNS"
2. Select "Customized" → enter `https://dns.google/dns-query`
3. Navigate to `facebook.com`
4. Open Firefox → Settings → Network → "Enable DNS over HTTPS"
5. Navigate to `facebook.com`
6. From terminal: `curl --doh-url https://1.1.1.1/dns-query https://facebook.com`
7. Try a lesser-known DoH provider (e.g., `https://dns.adguard.com/dns-query`)

### Expected Result
1-6: **Blocked** — WFP drops outbound TCP/443 to the 14 known DoH provider IPs
7: **May succeed** if the DoH provider IP is not in the blocked list — this is a **known limitation**

### Actual Result
- [ ] PASS — all 14 known DoH providers blocked
- [ ] FAIL — describe which provider bypassed

### Notes
- S-001 (resolved): WFP blocks 14 DoH IPs. New providers require update.
- Enterprise: can extend the list via policy schema `doh_blocked_ips[]`
- macOS: DNS proxy intercepts all DNS before browser DoH activates (if System Extension approved)

---

## BT-06: VPN Bypass

| Field | Value |
|-------|-------|
| **Test ID** | BT-06 |
| **Name** | VPN Tunnel Bypass Attempt |
| **Platform(s)** | Windows, macOS, Linux |
| **Category** | Conditional — depends on enforcement layer |

### Setup
- FocusMe blocking plan active for `facebook.com`
- Install a VPN client (e.g., WireGuard, OpenVPN, NordVPN)
- Do not connect VPN yet

### Steps
1. Verify `facebook.com` is blocked without VPN
2. Connect to VPN
3. Navigate to `facebook.com`
4. Check if DNS queries go through VPN tunnel
5. Run `nslookup facebook.com` — check which DNS server responds
6. Disconnect VPN and verify blocking resumes

### Expected Result
**Windows:**
- WFP filters operate at the WFP layer BEFORE VPN encapsulation (sublayer FWPM_SUBLAYER_UNIVERSAL)
- HOSTS file blocking works regardless of VPN (local resolution happens first)
- **facebook.com should remain blocked even with VPN active**

**macOS:**
- DNS proxy intercepts before VPN tunnel (if System Extension loads first)
- Order-dependent: if VPN loads before FocusMe, DNS may leak through tunnel

**Linux:**
- HOSTS file blocking works regardless of VPN
- Unbound on localhost resolves before VPN DNS

### Actual Result
- [ ] PASS — blocked even with VPN active
- [ ] FAIL — VPN bypassed blocking (describe which layer failed)

### Notes
- Some VPN clients replace DNS settings entirely — HOSTS-based blocking is the most resilient layer
- WFP operates below VPN in the network stack on Windows
- **Document as conditional limitation** — effectiveness varies by VPN client and OS

---

## BT-07: macOS Recovery Mode

| Field | Value |
|-------|-------|
| **Test ID** | BT-07 |
| **Name** | macOS Recovery Mode Bypass |
| **Platform(s)** | macOS 13+ |
| **Category** | Documented Limitation |

### Setup
- macOS with FocusMe installed and LaunchDaemon active
- Active blocking plan for `facebook.com`
- Note: Recovery Mode requires Intel: Cmd+R at boot, Apple Silicon: hold Power button

### Steps
1. Restart Mac and boot into Recovery Mode
2. Open Terminal from Utilities menu
3. Attempt to edit HOSTS file: `nano /Volumes/Macintosh\ HD/etc/hosts`
4. Remove FocusMe entries and save
5. Attempt to unload the LaunchDaemon plist
6. Reboot into normal mode
7. Check if FocusMe daemon restores HOSTS entries on startup

### Expected Result
- **Recovery Mode bypasses all third-party software** — this is a documented limitation
- User CAN edit HOSTS and unload daemons from Recovery Mode
- After normal reboot: FocusMe daemon starts, detects HOSTS tampering, restores entries within 2s
- Net result: temporary bypass during Recovery session only

### Actual Result
- [ ] PASS (Recovery Mode bypass expected; daemon restores on reboot)
- [ ] FAIL — daemon did not restore HOSTS after reboot

### Mitigation Notes
- **FileVault encryption**: prevents mounting the volume in Recovery without password
- **Firmware password** (Intel) / **MDM lock** (Apple Silicon): prevents Recovery Mode access
- Document in user FAQ: "Recovery Mode is a hardware-level escape vector"

---

## BT-08: Android Safe Mode

| Field | Value |
|-------|-------|
| **Test ID** | BT-08 |
| **Name** | Android Safe Mode Bypass |
| **Platform(s)** | Android 12+ |
| **Category** | Documented Limitation |

### Setup
- FocusMe app installed with AccessibilityService and VPN service active
- Active blocking plan for apps (e.g., block Instagram)
- Verify blocking works in normal mode

### Steps
1. Long-press the Power button
2. Long-press "Power off" until "Reboot to safe mode" appears
3. Confirm Safe Mode reboot
4. After boot, verify "Safe mode" watermark appears on screen
5. Open Instagram — observe if it launches
6. Check Settings → Accessibility — verify FocusMe service state
7. Reboot normally and verify blocking resumes

### Expected Result
- **Safe Mode disables all third-party apps** — FocusMe will not be running
- AccessibilityService is disabled in Safe Mode
- VPN service is disabled in Safe Mode
- Instagram launches without obstruction
- After normal reboot: FocusMe services resume automatically (RECEIVE_BOOT_COMPLETED intent)

### Actual Result
- [ ] PASS (Safe Mode bypasses all third-party apps = expected)
- [ ] FAIL — describe unexpected behavior

### Mitigation Notes
- Android Device Admin (deprecated) or Device Owner (MDM) can restrict Safe Mode
- Document in user FAQ and Play Store listing
- **Device Owner mode:** `dpm set-device-owner` can prevent Safe Mode — but requires enterprise provisioning

---

## BT-09: Process Injection (Windows)

| Field | Value |
|-------|-------|
| **Test ID** | BT-09 |
| **Name** | Remote Process Injection Against Daemon |
| **Platform(s)** | Windows 10/11 |
| **Category** | Must Block |

### Setup
- FocusMe daemon running as SYSTEM service
- Standard user account
- Tool: compile a simple DLL injector (e.g., `CreateRemoteThread` + `LoadLibrary` approach)
- Alternative: use a ready-made tool like `Process Hacker` (admin) or `InjectDLL` (if available)

### Steps
1. Get the PID of `focusme-daemon.exe`: `tasklist /FI "IMAGENAME eq focusme-daemon.exe"`
2. Attempt injection from standard user:
   ```
   inject.exe <daemon_pid> payload.dll
   ```
3. Observe result
4. Attempt injection from elevated (admin) command prompt
5. Observe result

### Expected Result
- **Standard user:** `OpenProcess()` returns ERROR_ACCESS_DENIED (daemon runs as SYSTEM)
- **Admin user:** If ACG (Arbitrary Code Guard) mitigation policy is set:
  - `VirtualAllocEx` or `WriteProcessMemory` should fail
  - `SetProcessMitigationPolicy(ProcessDynamicCodePolicy)` prevents dynamic code injection
- Without ACG: admin injection may succeed — this is acceptable (admin can do anything)

### Actual Result
- [ ] PASS — standard user gets Access Denied; admin blocked by ACG policy
- [ ] FAIL — describe what happened

### Notes
- ACG mitigation is applied in `main.rs` during daemon initialization
- Windows SYSTEM services have strong default protections against non-admin injection
- Admin injection is an accepted limitation (admin can simply uninstall the software)

---

## BT-10: Clock Rollback Against Forced Mode

| Field | Value |
|-------|-------|
| **Test ID** | BT-10 |
| **Name** | System Clock Rollback During Forced Mode Session |
| **Platform(s)** | Windows, macOS, Linux, Android |
| **Category** | Must Block |

### Setup
- Create a Forced Mode plan: block `reddit.com` for 2 hours
- Start the Forced Mode session
- Verify the countdown timer is running (via UI or IPC query)
- Note the monotonic clock value (if accessible via debug)

### Steps
1. Start Forced Mode — 2 hour session
2. Wait 5 minutes (timer should show ~1h55m remaining)
3. **Roll back system clock by 3 hours:**
   - Windows: `Set-Date -Date (Get-Date).AddHours(-3)` (admin)
   - Linux: `sudo date -s "3 hours ago"`
   - macOS: System Settings → Date & Time → uncheck automatic → set manually
4. Check FocusMe timer — observe remaining time
5. Wait 2 more minutes
6. Check timer again
7. Reset clock to correct time

### Expected Result
- **Timer continues from the correct position** — uses monotonic clock (not wall clock)
- After clock rollback of 3 hours, timer should still show ~1h53m (real elapsed = 7min)
- The dual-clock strategy in `forced_mode.rs` tracks both:
  - `Instant::now()` (monotonic) — primary timer
  - `Utc::now()` (wall clock) — cross-reference / display only
- Timer is NOT extended or reset by clock manipulation

### Actual Result
- [ ] PASS — monotonic clock unaffected by wall clock change
- [ ] FAIL — describe timer behavior after clock change

### Notes
- Android: `SystemClock.elapsedRealtime()` is used (monotonic, survives sleep)
- If both clocks disagree, daemon uses the one showing MORE elapsed time (anti-rollback)

---

## BT-11: Boot from USB / Live OS

| Field | Value |
|-------|-------|
| **Test ID** | BT-11 |
| **Name** | Boot from USB Live OS to Bypass |
| **Platform(s)** | Windows, Linux (BIOS-level) |
| **Category** | Documented Limitation |

### Setup
- Computer with FocusMe installed on internal drive
- USB drive with bootable Linux Live distro (e.g., Ubuntu Live)
- BIOS/UEFI accessible

### Steps
1. Restart computer
2. Enter BIOS/UEFI boot menu (typically F12 or Del)
3. Select USB drive as boot device
4. Boot into Linux Live environment
5. Mount internal drive: `sudo mount /dev/sda2 /mnt`
6. Browse freely using Live OS browser
7. Optionally: edit HOSTS file on mounted drive
8. Reboot back to normal OS

### Expected Result
- **FocusMe has no control over a USB-booted OS** — documented limitation
- User can browse freely in the Live OS
- Any HOSTS edits are detected and restored when FocusMe daemon starts on normal boot

### Actual Result
- [ ] PASS (USB boot bypasses = expected; HOSTS restored on normal boot)
- [ ] FAIL — HOSTS not restored after USB tampering

### Mitigation Notes
- **BIOS/UEFI password:** Prevent access to boot menu
- **Secure Boot:** Prevents booting unsigned OS images (if properly configured)
- **Full disk encryption (BitLocker/LUKS):** USB Live OS cannot mount encrypted partitions
- Document in FAQ: "Physical access to hardware = bypass possible"

---

## BT-12: Alternate Admin Account

| Field | Value |
|-------|-------|
| **Test ID** | BT-12 |
| **Name** | Uninstall from Alternate Admin Account |
| **Platform(s)** | Windows, macOS, Linux |
| **Category** | Conditional — Challenge Code Required |

### Setup
- FocusMe installed with active plans and Forced Mode session
- Know the admin password for a different admin account on the machine
- Plan protection enabled (challenge code active)

### Steps
1. Log in as alternate admin account
2. Attempt to stop the FocusMe service:
   - Windows: `sc stop focusme-daemon`
   - Linux: `sudo systemctl stop focusme`
   - macOS: `sudo launchctl unload /Library/LaunchDaemons/com.focusme.daemon.plist`
3. Attempt to uninstall:
   - Windows: Control Panel → Programs → Uninstall FocusMe
   - Linux: `sudo apt remove focusme`
   - macOS: Drag FocusMe.app to Trash
4. Observe if challenge code dialog appears
5. If prompted: enter incorrect code and observe
6. If prompted: enter correct code and observe

### Expected Result
- **Service stop:** Succeeds (admin has OS-level authority) — but service restarts on failure via SCM/systemd
- **Uninstall attempt:** FocusMe uninstaller presents **challenge code dialog**
  - Incorrect code → uninstall blocked
  - Correct code → uninstall proceeds
- **Forced Mode active:** Even with correct challenge code, Forced Mode timer must expire first (or Argon2id emergency unlock)
- Admin CAN force-remove by deleting files directly — this is a **documented limitation** (admin has full system access)

### Actual Result
- [ ] PASS — challenge code prompted on uninstall; brute-force blocked
- [ ] FAIL — describe what happened

### Notes
- Challenge code: 8-character random string → derive 8-digit numeric code (plan_protection.rs)
- Admin force-removal is accepted: "If you have admin access to your own computer, you can always remove software"
- Enterprise GPO/MDM can prevent uninstall by standard users

---

## Test Summary Matrix

| Test | Platform | Category | Pass Criteria | Status |
|------|----------|----------|---------------|--------|
| BT-01 | Windows | Documented Limitation | Daemon not running in Safe Mode | 🔲 |
| BT-02 | Windows | Must Block | Access Denied for standard user | 🔲 |
| BT-03 | Windows/Linux | Must Detect+Restore | HOSTS restored within 5s | 🔲 |
| BT-04 | All Desktop | Must Block | All browsers/clients blocked at network layer | 🔲 |
| BT-05 | All Desktop | Must Block | 14 known DoH providers blocked | 🔲 |
| BT-06 | All Desktop | Conditional | DNS/HOSTS blocking persists through VPN | 🔲 |
| BT-07 | macOS | Documented Limitation | Recovery bypass expected; HOSTS restored on reboot | 🔲 |
| BT-08 | Android | Documented Limitation | Safe Mode disables third-party apps | 🔲 |
| BT-09 | Windows | Must Block | Process injection fails for standard user | 🔲 |
| BT-10 | All | Must Block | Monotonic clock unaffected by wall clock change | 🔲 |
| BT-11 | Desktop | Documented Limitation | USB boot bypass expected; BIOS password mitigates | 🔲 |
| BT-12 | All Desktop | Conditional | Challenge code required for uninstall | 🔲 |

**Legend:** 🔲 Not tested | ✅ PASS | ❌ FAIL

---

## Sign-Off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| QA Lead | | | |
| Security Engineer | | | |
| Product Owner | | | |

---

*Generated Session 5. Execute on real hardware/VMs before release.*
