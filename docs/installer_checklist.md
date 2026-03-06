# FocusMe — Pre-Release Installer Checklist (Appendix B)

> **FILE:** docs/installer_checklist.md
> **TASK:** Phase 4 — QA / Packaging
> **AUTHOR:** FocusMe Co-Pilot (Claude Opus)
> **SESSION:** 5
> **PURPOSE:** QA sign-off form for all platform installers before release.
> **USAGE:** Complete every checkbox. All must pass before publishing.

---

## Windows MSI Checklist (10 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| W-01 | Silent install succeeds | `msiexec /i FocusMe_x64.msi /qn SILENT_INSTALL=1 /l*v install.log` | Exit code 0, service running | 🔲 |
| W-02 | Service starts on boot | `sc query focusme-daemon` after reboot | STATE: RUNNING, START_TYPE: AUTO_START | 🔲 |
| W-03 | EV code signing valid | `signtool verify /pa /v FocusMe_x64.msi` | "Successfully verified" + timestamp | 🔲 |
| W-04 | Timestamp counter-signature present | `signtool verify /pa /all FocusMe_x64.msi` | RFC3161 timestamp from DigiCert/Sectigo | 🔲 |
| W-05 | SmartScreen clear (no warning) | Double-click MSI on fresh Windows VM | No "Windows protected your PC" dialog | 🔲 |
| W-06 | Uninstall removes cleanly | Control Panel → Uninstall → verify paths removed | `C:\Program Files\FocusMe` deleted, service removed, registry cleaned | 🔲 |
| W-07 | Enterprise MST transform works | `msiexec /i FocusMe_x64.msi TRANSFORMS=enterprise.mst /qn` | Silent, no reboot prompt, ARPNOREMOVE=1 | 🔲 |
| W-08 | NMH registry entries created | `reg query HKLM\SOFTWARE\Google\Chrome\NativeMessagingHosts\com.focusme.nmh` | REG_SZ → path to com.focusme.nmh.json | 🔲 |
| W-09 | WFP filters active after install | `netsh wfp show filters` + grep focusme | Filter with HOSTS IPs + 14 DoH IPs present | 🔲 |
| W-10 | Process blocking active | Block `notepad.exe` in plan → try to launch | notepad.exe terminates immediately | 🔲 |

---

## macOS PKG Checklist (8 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| M-01 | Notarization accepted | `xcrun stapler validate FocusMe.pkg` | "The validate action worked!" | 🔲 |
| M-02 | Universal Binary (x86_64 + arm64) | `lipo -info /Applications/FocusMe.app/Contents/MacOS/focusme-daemon` | "x86_64 arm64" | 🔲 |
| M-03 | LaunchDaemon loads at boot | `sudo launchctl list \| grep focusme` | PID present, exit status 0 | 🔲 |
| M-04 | System Extension visible | System Settings → Privacy → Security → System Extensions | FocusMe listed and enabled | 🔲 |
| M-05 | Network Extension prompts | First launch → system dialog for DNS proxy | "FocusMe would like to filter network content" prompt appears | 🔲 |
| M-06 | Sparkle update works | Set SUFeedURL to test server → trigger check | Update dialog appears, download + install succeeds | 🔲 |
| M-07 | ESF exec blocking active | Block `Spotify.app`, attempt launch | Process killed + notification shown | 🔲 |
| M-08 | DNS proxy resolves correctly | Block `facebook.com` → `nslookup facebook.com` | NXDOMAIN response | 🔲 |

---

## Linux .deb Checklist (8 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| L-01 | apt install clean | `sudo apt install ./focusme_amd64.deb` | "Setting up focusme..." → exit 0, no errors | 🔲 |
| L-02 | systemd unit active | `systemctl status focusme` | "active (running)" | 🔲 |
| L-03 | eBPF pinned (if available) | `ls /sys/fs/bpf/focusme/` | `exec_block`, `blocked_paths` files present | 🔲 |
| L-04 | GPG signature verifies | `dpkg-sig --verify focusme_amd64.deb` | "GOODSIG" | 🔲 |
| L-05 | apt remove clean | `sudo apt remove focusme` → check residuals | Service stopped, binary removed, configs remain (purge to delete) | 🔲 |
| L-06 | resolv.conf restored on remove | `cat /etc/resolv.conf` after apt remove | Original nameserver restored from `.focusme.bak` | 🔲 |
| L-07 | HOSTS cleaned on remove | `grep focusme /etc/hosts` after apt remove | No FocusMe entries remain | 🔲 |
| L-08 | Fanotify fallback if no eBPF | Boot with `lsm=landlock,lockdown,yama` (no bpf) → check logs | "Using FanotifyBlocker fallback" in journalctl | 🔲 |

---

## Browser Extension Checklist (6 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| E-01 | Loads in Chrome | chrome://extensions → Load unpacked / Install from CWS | Extension appears, service worker active | 🔲 |
| E-02 | Loads in Firefox | about:addons → Install from file / AMO | Extension listed, background script running | 🔲 |
| E-03 | Loads in Edge | edge://extensions → Load unpacked / Install from CWS | Extension appears, same as Chrome | 🔲 |
| E-04 | NMH connects | Open extension popup → status indicator | "Connected" status, no backoff errors in console | 🔲 |
| E-05 | BLOCK redirect fires | Block `reddit.com` → navigate to it | Redirected to `blocked.html` with domain + plan info | 🔲 |
| E-06 | DNR rules applied | Block 100 domains → `chrome.declarativeNetRequest.getDynamicRules()` | Rules array length matches, ≤5000 rules | 🔲 |

---

## Android APK Checklist (6 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| A-01 | AccessibilityService permission granted | Settings → Accessibility → FocusMe | Service toggle ON, no crash | 🔲 |
| A-02 | VPN service starts | Enable DNS blocking in FocusMe → VPN key icon | VPN icon in status bar, `adb shell dumpsys netstats` shows tun0 | 🔲 |
| A-03 | Quota enforced | Set 30min daily limit for Instagram → use 30min | Overlay appears: "Time limit reached" | 🔲 |
| A-04 | Overlay appears on blocked app | Block Instagram → switch to Instagram | Full-screen overlay with app name and countdown | 🔲 |
| A-05 | Boot persistence | Reboot device → check FocusMe state | DaemonService starts via RECEIVE_BOOT_COMPLETED, plans resume | 🔲 |
| A-06 | Play Store data safety accurate | Compare data safety form to actual data collection | All declarations match actual behavior | 🔲 |

---

## General Release Checklist (5 items)

| # | Check | Test Command / Method | Expected Output | Status |
|---|-------|----------------------|-----------------|--------|
| G-01 | SBOM generated | `cargo cyclonedx --format json` + `npx @cyclonedx/cyclonedx-npm --output-format json` | `bom.json` files for Rust + Node.js | 🔲 |
| G-02 | Version number bumped | Check Cargo.toml, package.json, tauri.conf.json, build.gradle, manifest.json versions | All match release version (e.g., 1.0.0) | 🔲 |
| G-03 | CHANGELOG updated | Review CHANGELOG.md | Current version section with all notable changes | 🔲 |
| G-04 | CI green | GitHub Actions → latest workflow run | All jobs pass (build, test, lint) | 🔲 |
| G-05 | Secrets rotated | Verify CI secrets are current | `APPLE_CERT_*`, `WINDOWS_EV_*`, `PLAY_STORE_*` all valid | 🔲 |

---

## Summary

| Platform | Items | Passed | Failed | Blocked | Status |
|----------|-------|--------|--------|---------|--------|
| Windows MSI | 10 | | | | 🔲 |
| macOS PKG | 8 | | | | 🔲 |
| Linux .deb | 8 | | | | 🔲 |
| Browser Extension | 6 | | | | 🔲 |
| Android APK | 6 | | | | 🔲 |
| General Release | 5 | | | | 🔲 |
| **TOTAL** | **43** | | | | 🔲 |

### Sign-Off

| Role | Name | Date | All Items Pass? |
|------|------|------|-----------------|
| QA Engineer | | | ☐ Yes |
| DevOps / Release Eng. | | | ☐ Yes |
| Product Manager | | | ☐ Yes |
| Security Lead | | | ☐ Yes |

**Release gate:** All 43 items must be ✅ PASS (or explicitly waived with justification) before publishing to any store or distribution channel.

---

*Generated Session 5. Fill during QA testing on real devices.*
