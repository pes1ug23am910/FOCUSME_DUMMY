// ============================================================
// FILE:        process_monitor.rs
// MODULE:      Layer 1 — Enforcement Engine > Process Blocking
// TASK:        T-013
// PLATFORM:    windows (macOS uses ESF, Linux uses eBPF LSM)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, Windows daemon core
// DEPENDENCIES: windows-sys 0.52 (ToolHelp32, OpenProcess, TerminateProcess)
// TEST COVERAGE: IT-01 (blocked process killed within 2s)
// KNOWN LIMITATIONS: User-mode only; admin can use Task Manager.
//                    500ms poll interval means up to 500ms window before kill.
// ANTI-CIRCUMVENTION: Defends against APP-01 (system-level app blocking).
//                     BT-02 (process kill) protection is separate (service hardening).
// ============================================================

use anyhow::Result;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Poll interval for process enumeration (per build plan: 500ms)
const POLL_INTERVAL_MS: u64 = 500;

/// Represents a rule for matching processes to block
#[derive(Debug, Clone)]
pub enum ProcessRule {
    /// Match by process executable name (e.g., "Spotify.exe")
    ProcessName(String),
    /// Match by path prefix (e.g., "C:\Games\")
    PathPrefix(String),
    /// Match by exact path
    PathExact(String),
    /// Match by macOS bundle ID (e.g., "com.spotify.client")
    BundleId(String),
}

/// ProcessMonitor continuously scans running processes and terminates
/// any that match the active block list.
pub struct ProcessMonitor {
    /// Active rules loaded from enabled plans
    block_rules: Arc<RwLock<Vec<ProcessRule>>>,
    /// Whether monitoring is active
    running: Arc<RwLock<bool>>,
}

impl ProcessMonitor {
    /// Create a new ProcessMonitor
    pub fn new() -> Self {
        Self {
            block_rules: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Update the active block rules from the scheduler
    pub async fn update_rules(&self, rules: Vec<ProcessRule>) {
        let mut current = self.block_rules.write().await;
        info!(count = rules.len(), "Process monitor rules updated");
        *current = rules;
    }

    /// Start the process monitoring loop
    ///
    /// Polls every POLL_INTERVAL_MS (500ms) and terminates blocked processes.
    /// Uses CreateToolhelp32Snapshot + Process32Next on Windows.
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        info!(
            interval_ms = POLL_INTERVAL_MS,
            "Process monitor started"
        );

        let rules = self.block_rules.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            loop {
                {
                    let is_running = running.read().await;
                    if !*is_running {
                        info!("Process monitor stopping");
                        break;
                    }
                }

                let current_rules = rules.read().await;
                if !current_rules.is_empty() {
                    if let Err(e) = Self::scan_and_kill(&current_rules).await {
                        warn!(error = %e, "Process scan failed");
                    }
                }
                drop(current_rules);

                tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            }
        });

        Ok(())
    }

    /// Stop the process monitoring loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Process monitor stopped");
    }

    /// Scan all running processes and kill any that match block rules
    #[cfg(windows)]
    async fn scan_and_kill(rules: &[ProcessRule]) -> Result<()> {
        use std::mem;
        use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
            PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess,
            PROCESS_TERMINATE, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        debug!(rules = rules.len(), "Scanning processes against block rules");

        // Take a snapshot of all running processes
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            anyhow::bail!("CreateToolhelp32Snapshot failed");
        }

        let mut entry: PROCESSENTRY32W = unsafe { mem::zeroed() };
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut success = unsafe { Process32FirstW(snapshot, &mut entry) };

        while success != 0 {
            // Convert wide char exe name to Rust String
            let exe_name_len = entry.szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let exe_name = String::from_utf16_lossy(&entry.szExeFile[..exe_name_len]);
            let pid = entry.th32ProcessID;

            // Skip system processes (PID 0 and 4)
            if pid > 4 {
                // Try to get full process path for path-based matching
                let process_path = Self::get_process_path(pid).unwrap_or_default();

                if Self::matches_rule(&exe_name, &process_path, rules) {
                    info!(
                        pid = pid,
                        process = %exe_name,
                        "Terminating blocked process"
                    );

                    let proc_handle = unsafe {
                        OpenProcess(PROCESS_TERMINATE, 0, pid)
                    };

                    if proc_handle != 0 {
                        let result = unsafe { TerminateProcess(proc_handle, 1) };
                        unsafe { CloseHandle(proc_handle) };

                        if result != 0 {
                            info!(pid = pid, process = %exe_name, "Blocked process terminated");
                        } else {
                            warn!(pid = pid, process = %exe_name, "Failed to terminate blocked process");
                        }
                    } else {
                        warn!(pid = pid, process = %exe_name, "Cannot open process for termination (access denied)");
                    }
                }
            }

            success = unsafe { Process32NextW(snapshot, &mut entry) };
        }

        unsafe { CloseHandle(snapshot) };

        Ok(())
    }

    /// Get the full path of a process by PID (Windows)
    #[cfg(windows)]
    fn get_process_path(pid: u32) -> Option<String> {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        };
        use windows_sys::Win32::Foundation::CloseHandle;

        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle == 0 {
            return None;
        }

        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;

        let result = unsafe {
            QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size)
        };

        unsafe { CloseHandle(handle) };

        if result != 0 && size > 0 {
            Some(String::from_utf16_lossy(&buf[..size as usize]))
        } else {
            None
        }
    }

    /// Non-Windows stub — process blocking is handled by platform-specific subsystems
    #[cfg(not(windows))]
    async fn scan_and_kill(rules: &[ProcessRule]) -> Result<()> {
        // Linux: handled by eBPF LSM (T-016)
        // macOS: handled by ESF (T-014)
        debug!(rules = rules.len(), "Process scan skipped on non-Windows (see eBPF/ESF)");
        Ok(())
    }

    /// Check if a process matches any of the given rules
    fn matches_rule(process_name: &str, process_path: &str, rules: &[ProcessRule]) -> bool {
        for rule in rules {
            match rule {
                ProcessRule::ProcessName(name) => {
                    if process_name.eq_ignore_ascii_case(name) {
                        return true;
                    }
                }
                ProcessRule::PathPrefix(prefix) => {
                    if process_path
                        .to_lowercase()
                        .starts_with(&prefix.to_lowercase())
                    {
                        return true;
                    }
                }
                ProcessRule::PathExact(path) => {
                    if process_path.eq_ignore_ascii_case(path) {
                        return true;
                    }
                }
                ProcessRule::BundleId(_) => {
                    // macOS only — not applicable on Windows
                }
            }
        }
        false
    }
}

// ============================================================
// UNIT TESTS
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_process_name() {
        let rules = vec![ProcessRule::ProcessName("Spotify.exe".to_string())];
        assert!(ProcessMonitor::matches_rule("Spotify.exe", "", &rules));
        assert!(ProcessMonitor::matches_rule("spotify.exe", "", &rules)); // case-insensitive
        assert!(!ProcessMonitor::matches_rule("Chrome.exe", "", &rules));
    }

    #[test]
    fn test_matches_path_prefix() {
        let rules = vec![ProcessRule::PathPrefix("C:\\Games\\".to_string())];
        assert!(ProcessMonitor::matches_rule(
            "game.exe",
            "C:\\Games\\SomeGame\\game.exe",
            &rules
        ));
        assert!(!ProcessMonitor::matches_rule(
            "chrome.exe",
            "C:\\Program Files\\Chrome\\chrome.exe",
            &rules
        ));
    }

    #[test]
    fn test_matches_path_exact() {
        let rules = vec![ProcessRule::PathExact(
            "C:\\Games\\game.exe".to_string(),
        )];
        assert!(ProcessMonitor::matches_rule("game.exe", "C:\\Games\\game.exe", &rules));
        assert!(!ProcessMonitor::matches_rule("game.exe", "C:\\Other\\game.exe", &rules));
    }

    #[test]
    fn test_no_match_returns_false() {
        let rules = vec![ProcessRule::ProcessName("Spotify.exe".to_string())];
        assert!(!ProcessMonitor::matches_rule("Chrome.exe", "C:\\Chrome\\chrome.exe", &rules));
    }
}
