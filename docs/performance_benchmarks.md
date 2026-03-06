# FocusMe Performance Benchmarks

> **Document:** `docs/performance_benchmarks.md`
> **Task:** T-051
> **Author:** FocusMe Co-Pilot (Claude Opus)
> **Session:** 4
> **Status:** Template — measurements to be filled during QA testing
> **Last Updated:** Session 4

---

## 1. Benchmark Targets

Performance targets established in the build plan. All measurements should meet
these thresholds on minimum-spec hardware (see §2).

| Metric | Target | Critical Threshold | Notes |
|--------|--------|-------------------|-------|
| **Daemon startup** | < 500 ms | < 1000 ms | Time to first IPC response |
| **CPU idle (daemon)** | < 1% | < 3% | Daemon with no active plans |
| **CPU active (daemon)** | < 5% | < 10% | Daemon enforcing 5 plans |
| **Memory (daemon)** | < 30 MB RSS | < 50 MB | Steady-state RSS |
| **Memory (UI shell)** | < 80 MB RSS | < 150 MB | Tauri + WebView2 |
| **IPC round-trip** | < 5 ms | < 20 ms | GET_STATUS request/response |
| **DNS block latency** | < 2 ms | < 10 ms | NXDOMAIN response time |
| **Browser ext popup** | < 100 ms | < 300 ms | Popup open to content rendered |
| **Extension DNR update** | < 50 ms | < 200 ms | Rule update applied |
| **Android app launch** | < 1 s | < 2 s | Cold start to first frame |
| **Android battery drain** | < 2%/hr | < 5%/hr | VPN + Accessibility active |
| **Installer size (Windows)** | < 25 MB | < 50 MB | MSI without WebView2 |
| **Installer size (Linux)** | < 15 MB | < 30 MB | .deb package |
| **Installer size (macOS)** | < 20 MB | < 40 MB | .pkg file |
| **APK size** | < 15 MB | < 30 MB | AAB compressed |

---

## 2. Test Hardware

### 2.1 Minimum-Spec Targets

| Platform | Hardware | OS | RAM | Storage |
|----------|----------|------|------|---------|
| Windows (min) | Intel i5-8250U (2018) | Windows 10 21H2 | 8 GB | SSD |
| Windows (target) | Intel i7-12700 / Ryzen 7 | Windows 11 23H2 | 16 GB | NVMe |
| macOS (min) | Apple M1 (2020) | macOS 13 Ventura | 8 GB | SSD |
| macOS (target) | Apple M2 Pro | macOS 14 Sonoma | 16 GB | SSD |
| Linux (min) | Intel i5 / Ryzen 5 | Ubuntu 22.04 LTS | 8 GB | SSD |
| Android (min) | Snapdragon 665 | Android 10 (API 29) | 4 GB | eMMC |
| Android (target) | Snapdragon 8 Gen 2 | Android 14 (API 34) | 8 GB | UFS 4.0 |

### 2.2 Test Environment Requirements

- All background apps closed (except OS services)
- Wi-Fi connected (for DNS tests)
- Battery at 80%+ (Android battery drain tests)
- 3 warm-up runs before measurement
- 10 measurement runs per test (report: median, p95, p99)

---

## 3. Benchmark Results

### 3.1 Windows

| Metric | Min-Spec | Target-Spec | Target | Status |
|--------|----------|-------------|--------|--------|
| Daemon startup | ___ ms | ___ ms | < 500 ms | 🔲 |
| CPU idle | ___ % | ___ % | < 1% | 🔲 |
| CPU active (5 plans) | ___ % | ___ % | < 5% | 🔲 |
| Memory (daemon RSS) | ___ MB | ___ MB | < 30 MB | 🔲 |
| Memory (UI RSS) | ___ MB | ___ MB | < 80 MB | 🔲 |
| IPC round-trip (p50) | ___ ms | ___ ms | < 5 ms | 🔲 |
| IPC round-trip (p95) | ___ ms | ___ ms | < 10 ms | 🔲 |
| DNS block latency (WFP) | ___ ms | ___ ms | < 2 ms | 🔲 |
| MSI installer size | ___ MB | — | < 25 MB | 🔲 |
| Install time (silent) | ___ s | ___ s | < 30 s | 🔲 |

**Tools:** `Measure-Command`, Performance Monitor, WPR/WPA, `hyperfine`

### 3.2 macOS

| Metric | M1 | M2 Pro | Target | Status |
|--------|-----|--------|--------|--------|
| Daemon startup | ___ ms | ___ ms | < 500 ms | 🔲 |
| CPU idle | ___ % | ___ % | < 1% | 🔲 |
| CPU active (5 plans) | ___ % | ___ % | < 5% | 🔲 |
| Memory (daemon RSS) | ___ MB | ___ MB | < 30 MB | 🔲 |
| Memory (UI RSS) | ___ MB | ___ MB | < 80 MB | 🔲 |
| DNS block (NEDNSProxy) | ___ ms | ___ ms | < 2 ms | 🔲 |
| .pkg installer size | ___ MB | — | < 20 MB | 🔲 |

**Tools:** `time`, Instruments, Activity Monitor, `hyperfine`

### 3.3 Linux

| Metric | Min-Spec | Target-Spec | Target | Status |
|--------|----------|-------------|--------|--------|
| Daemon startup | ___ ms | ___ ms | < 500 ms | 🔲 |
| CPU idle | ___ % | ___ % | < 1% | 🔲 |
| CPU active (5 plans) | ___ % | ___ % | < 5% | 🔲 |
| Memory (daemon RSS) | ___ MB | ___ MB | < 30 MB | 🔲 |
| Memory (UI RSS) | ___ MB | ___ MB | < 80 MB | 🔲 |
| DNS block (Unbound RPZ) | ___ ms | ___ ms | < 2 ms | 🔲 |
| eBPF attach time | ___ ms | ___ ms | < 100 ms | 🔲 |
| .deb package size | ___ MB | — | < 15 MB | 🔲 |

**Tools:** `perf stat`, `systemd-analyze`, `valgrind --tool=massif`, `hyperfine`

### 3.4 Android

| Metric | SD 665 | SD 8 Gen 2 | Target | Status |
|--------|---------|-----------|--------|--------|
| Cold start | ___ ms | ___ ms | < 1000 ms | 🔲 |
| Warm start | ___ ms | ___ ms | < 300 ms | 🔲 |
| CPU idle (service) | ___ % | ___ % | < 1% | 🔲 |
| Memory (service RSS) | ___ MB | ___ MB | < 40 MB | 🔲 |
| Battery drain (%/hr) | ___ % | ___ % | < 2% | 🔲 |
| VPN throughput | ___ Mbps | ___ Mbps | > 100 Mbps | 🔲 |
| Plan switch latency | ___ ms | ___ ms | < 100 ms | 🔲 |
| APK size | ___ MB | — | < 15 MB | 🔲 |

**Tools:** Android Studio Profiler, `adb shell dumpsys meminfo`, Battery Historian

### 3.5 Browser Extension

| Metric | Chrome | Firefox | Edge | Target | Status |
|--------|--------|---------|------|--------|--------|
| Popup open → render | ___ ms | ___ ms | ___ ms | < 100 ms | 🔲 |
| DNR rule update | ___ ms | ___ ms | ___ ms | < 50 ms | 🔲 |
| NMH connect time | ___ ms | ___ ms | ___ ms | < 100 ms | 🔲 |
| Content script inject | ___ ms | ___ ms | ___ ms | < 20 ms | 🔲 |
| Background memory | ___ MB | ___ MB | ___ MB | < 10 MB | 🔲 |

**Tools:** DevTools Performance tab, `chrome://extensions` process manager

---

## 4. Stress Tests

### 4.1 Large Rule Set Performance

Test with increasing rule counts to find degradation threshold.

| Rule Count | DNR Update (ms) | DNS Lookup (ms) | Memory Delta (MB) | Status |
|-----------|----------------|----------------|-------------------|--------|
| 100 | ___ | ___ | ___ | 🔲 |
| 500 | ___ | ___ | ___ | 🔲 |
| 1,000 | ___ | ___ | ___ | 🔲 |
| 5,000 (MAX_DNR) | ___ | ___ | ___ | 🔲 |
| 10,000 (HOSTS) | ___ | ___ | ___ | 🔲 |

### 4.2 Concurrent Plan Stress

| Active Plans | CPU (%) | Memory (MB) | IPC Latency (ms) | Status |
|-------------|---------|-------------|-------------------|--------|
| 1 | ___ | ___ | ___ | 🔲 |
| 5 | ___ | ___ | ___ | 🔲 |
| 10 | ___ | ___ | ___ | 🔲 |
| 25 | ___ | ___ | ___ | 🔲 |
| 50 | ___ | ___ | ___ | 🔲 |

### 4.3 Long-Running Stability (24-hour soak test)

| Metric | Start | +6 hr | +12 hr | +24 hr | Status |
|--------|-------|-------|--------|--------|--------|
| Daemon RSS (MB) | ___ | ___ | ___ | ___ | 🔲 |
| Handle count (Win) | ___ | ___ | ___ | ___ | 🔲 |
| FD count (Linux) | ___ | ___ | ___ | ___ | 🔲 |
| IPC latency p99 (ms) | ___ | ___ | ___ | ___ | 🔲 |
| Crash count | ___ | ___ | ___ | ___ | 🔲 |

---

## 5. Benchmark Scripts

### 5.1 Daemon Startup (Linux/macOS)

```bash
#!/usr/bin/env bash
# Measure daemon cold-start time
hyperfine \
    --warmup 3 \
    --runs 10 \
    --prepare 'systemctl stop focusme' \
    'systemctl start focusme && \
     timeout 5 bash -c "until echo GET_STATUS | socat - UNIX:/var/run/focusme.sock; do sleep 0.01; done"'
```

### 5.2 IPC Round-Trip (all platforms)

```bash
# Send GET_STATUS and measure response time
hyperfine \
    --warmup 5 \
    --runs 100 \
    'echo '"'"'{"type":"GET_STATUS"}'"'"' | socat - UNIX:/var/run/focusme.sock'
```

### 5.3 DNS Block Latency

```bash
# Measure NXDOMAIN response time for a blocked domain
hyperfine \
    --warmup 5 \
    --runs 50 \
    'dig +short +time=1 reddit.com @127.0.0.1'
```

### 5.4 Memory Snapshot

```bash
# Linux: RSS of daemon process
ps -o rss= -p $(pgrep focusme-daemon) | awk '{print $1/1024 " MB"}'

# Windows (PowerShell):
# (Get-Process focusme-daemon).WorkingSet64 / 1MB
```

---

## 6. Regression Tracking

Track benchmark results across releases to detect performance regressions.

| Version | Daemon Startup | CPU Idle | Memory (RSS) | IPC Latency | Status |
|---------|---------------|----------|-------------|-------------|--------|
| 0.1.0-alpha | ___ ms | ___ % | ___ MB | ___ ms | 🔲 |
| 0.2.0-beta | ___ ms | ___ % | ___ MB | ___ ms | 🔲 |
| 1.0.0 | ___ ms | ___ % | ___ MB | ___ ms | 🔲 |

**CI Integration:** Run benchmarks in GitHub Actions nightly workflow. Alert on > 20% regression.

---

## 7. Notes

- All memory measurements use RSS (Resident Set Size), not VSZ
- CPU measurements averaged over 60-second windows
- Battery drain tests: airplane mode off, screen off, VPN + Accessibility active
- DNS tests require Unbound running locally with FocusMe RPZ zone loaded
- WFP tests require admin PowerShell with WFP callout driver loaded
