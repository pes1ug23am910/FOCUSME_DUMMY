// ============================================================
// FILE:        hosts_manager.rs
// MODULE:      Layer 1 — Enforcement Engine > Windows URL Blocking
// TASK:        T-011
// PLATFORM:    windows
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, Windows daemon core
// DEPENDENCIES: windows-sys 0.52, tokio (FileSystemWatcher)
// TEST COVERAGE: IT-01, BT-03 (HOSTS tamper detection)
// KNOWN LIMITATIONS: Admin user can edit HOSTS directly; re-applied within 2s.
//                    Does NOT block DoH-enabled browsers — see OQ-04.
// ANTI-CIRCUMVENTION: Defends against BT-03 (HOSTS file edit by non-admin)
//                     and BT-04 (alternate browser) at DNS level.
// ============================================================

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn, error};

/// Managed HOSTS file entries — blocked domains map to 0.0.0.0
const BLOCK_IP: &str = "0.0.0.0";
const BLOCK_IPV6: &str = "::0";

/// Marker comments for FocusMe-managed entries
const FOCUSME_START_MARKER: &str = "# >>> FocusMe Managed Block List — DO NOT EDIT <<<";
const FOCUSME_END_MARKER: &str = "# >>> FocusMe End Block List <<<";

/// HostsManager manages the system HOSTS file for URL/domain blocking
pub struct HostsManager {
    hosts_path: PathBuf,
    blocked_domains: Arc<RwLock<HashSet<String>>>,
    // TODO: FileSystemWatcher handle for tamper detection
}

impl HostsManager {
    /// Create a new HostsManager instance
    ///
    /// # Platform
    /// Windows: %SystemRoot%\System32\drivers\etc\hosts
    /// Linux: /etc/hosts
    /// macOS: /etc/hosts (but DNS blocking uses NEDNSProxyProvider instead)
    pub fn new() -> Result<Self> {
        let hosts_path = Self::get_hosts_path();
        info!(path = %hosts_path.display(), "HostsManager initialized");

        Ok(Self {
            hosts_path,
            blocked_domains: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Get platform-specific HOSTS file path
    fn get_hosts_path() -> PathBuf {
        #[cfg(windows)]
        {
            let system_root = std::env::var("SystemRoot")
                .unwrap_or_else(|_| "C:\\Windows".to_string());
            PathBuf::from(system_root)
                .join("System32")
                .join("drivers")
                .join("etc")
                .join("hosts")
        }

        #[cfg(not(windows))]
        {
            PathBuf::from("/etc/hosts")
        }
    }

    /// Update the blocked domains list and rewrite the HOSTS file
    pub async fn update_blocked_domains(&self, domains: HashSet<String>) -> Result<()> {
        let mut current = self.blocked_domains.write().await;
        *current = domains;
        self.write_hosts_file(&current)?;
        info!(count = current.len(), "HOSTS file updated with blocked domains");
        Ok(())
    }

    /// Add a single domain to the block list
    pub async fn add_blocked_domain(&self, domain: String) -> Result<()> {
        let mut current = self.blocked_domains.write().await;
        current.insert(domain.clone());
        self.write_hosts_file(&current)?;
        info!(domain = %domain, "Domain added to HOSTS block list");
        Ok(())
    }

    /// Remove a single domain from the block list
    pub async fn remove_blocked_domain(&self, domain: &str) -> Result<()> {
        let mut current = self.blocked_domains.write().await;
        current.remove(domain);
        self.write_hosts_file(&current)?;
        info!(domain = %domain, "Domain removed from HOSTS block list");
        Ok(())
    }

    /// Write the HOSTS file with FocusMe-managed entries
    /// Preserves existing non-FocusMe entries
    fn write_hosts_file(&self, domains: &HashSet<String>) -> Result<()> {
        let content = std::fs::read_to_string(&self.hosts_path)
            .unwrap_or_default();

        // Remove existing FocusMe block
        let cleaned = Self::remove_focusme_entries(&content);

        // Build new FocusMe block
        let mut focusme_block = String::new();
        if !domains.is_empty() {
            focusme_block.push_str(FOCUSME_START_MARKER);
            focusme_block.push('\n');

            let mut sorted_domains: Vec<&String> = domains.iter().collect();
            sorted_domains.sort();

            for domain in sorted_domains {
                focusme_block.push_str(&format!("{} {}\n", BLOCK_IP, domain));
                focusme_block.push_str(&format!("{} {}\n", BLOCK_IPV6, domain));
                // Also block www subdomain if not already a wildcard
                if !domain.starts_with("www.") {
                    focusme_block.push_str(&format!("{} www.{}\n", BLOCK_IP, domain));
                    focusme_block.push_str(&format!("{} www.{}\n", BLOCK_IPV6, domain));
                }
            }
            focusme_block.push_str(FOCUSME_END_MARKER);
            focusme_block.push('\n');
        }

        let final_content = format!("{}\n{}", cleaned.trim(), focusme_block);
        std::fs::write(&self.hosts_path, final_content)?;

        Ok(())
    }

    /// Remove FocusMe-managed entries from HOSTS content
    fn remove_focusme_entries(content: &str) -> String {
        let mut result = String::new();
        let mut inside_block = false;

        for line in content.lines() {
            if line.trim() == FOCUSME_START_MARKER {
                inside_block = true;
                continue;
            }
            if line.trim() == FOCUSME_END_MARKER {
                inside_block = false;
                continue;
            }
            if !inside_block {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    /// Start tamper detection — watches HOSTS file for unauthorized changes
    /// and restores FocusMe entries within 2 seconds (per BT-03)
    pub async fn start_tamper_detection(&self) -> Result<()> {
        // ANTI-CIRCUMVENTION: BT-03 defense
        // Poll-based approach for cross-platform compatibility.
        // Checks HOSTS file integrity every 2s and restores if tampered.

        let hosts_path = self.hosts_path.clone();
        let blocked_domains = self.blocked_domains.clone();

        tokio::spawn(async move {
            info!(path = %hosts_path.display(), "HOSTS tamper detection started (2s polling)");

            // Capture initial hash of HOSTS content
            let mut last_hash = Self::hash_hosts_content(&hosts_path);

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;

                let current_hash = Self::hash_hosts_content(&hosts_path);

                // If file changed, check if FocusMe entries are intact
                if current_hash != last_hash {
                    let domains = blocked_domains.read().await;
                    if !domains.is_empty() {
                        let content = std::fs::read_to_string(&hosts_path)
                            .unwrap_or_default();

                        // Check if FocusMe markers are present
                        if !content.contains(FOCUSME_START_MARKER) {
                            warn!("HOSTS file tampered — FocusMe entries missing, restoring");

                            // Remove any partial FocusMe entries and rebuild
                            let cleaned = Self::remove_focusme_entries(&content);
                            let mut focusme_block = String::new();
                            focusme_block.push_str(FOCUSME_START_MARKER);
                            focusme_block.push('\n');

                            let mut sorted: Vec<&String> = domains.iter().collect();
                            sorted.sort();
                            for domain in sorted {
                                focusme_block.push_str(&format!("{} {}\n", BLOCK_IP, domain));
                                focusme_block.push_str(&format!("{} {}\n", BLOCK_IPV6, domain));
                                if !domain.starts_with("www.") {
                                    focusme_block.push_str(
                                        &format!("{} www.{}\n", BLOCK_IP, domain),
                                    );
                                    focusme_block.push_str(
                                        &format!("{} www.{}\n", BLOCK_IPV6, domain),
                                    );
                                }
                            }
                            focusme_block.push_str(FOCUSME_END_MARKER);
                            focusme_block.push('\n');

                            let final_content = format!("{}\n{}", cleaned.trim(), focusme_block);
                            if let Err(e) = std::fs::write(&hosts_path, &final_content) {
                                error!(error = %e, "Failed to restore HOSTS after tamper");
                            } else {
                                info!("HOSTS entries restored after tamper detection");
                            }
                        }
                    }

                    last_hash = Self::hash_hosts_content(&hosts_path);
                }
            }
        });

        Ok(())
    }

    /// Compute a simple hash of the HOSTS file content for change detection
    fn hash_hosts_content(path: &std::path::Path) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Restore all FocusMe entries (called on tamper detection or daemon restart)
    pub async fn restore_entries(&self) -> Result<()> {
        let domains = self.blocked_domains.read().await;
        if !domains.is_empty() {
            self.write_hosts_file(&domains)?;
            info!(count = domains.len(), "HOSTS entries restored after tamper/restart");
        }
        Ok(())
    }

    /// Remove all FocusMe entries from HOSTS file (called on clean shutdown/uninstall)
    pub async fn cleanup(&self) -> Result<()> {
        let content = std::fs::read_to_string(&self.hosts_path)
            .unwrap_or_default();
        let cleaned = Self::remove_focusme_entries(&content);
        std::fs::write(&self.hosts_path, cleaned)?;
        info!("FocusMe HOSTS entries cleaned up");
        Ok(())
    }
}

// ============================================================
// UNIT TESTS
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_focusme_entries_preserves_other_entries() {
        let content = format!(
            "127.0.0.1 localhost\n{}\n0.0.0.0 reddit.com\n{}\n::1 localhost\n",
            FOCUSME_START_MARKER, FOCUSME_END_MARKER
        );
        let result = HostsManager::remove_focusme_entries(&content);
        assert!(result.contains("127.0.0.1 localhost"));
        assert!(result.contains("::1 localhost"));
        assert!(!result.contains("reddit.com"));
    }

    #[test]
    fn test_remove_focusme_entries_handles_no_markers() {
        let content = "127.0.0.1 localhost\n::1 localhost\n";
        let result = HostsManager::remove_focusme_entries(content);
        assert!(result.contains("127.0.0.1 localhost"));
    }

    #[test]
    fn test_get_hosts_path_returns_valid_path() {
        let path = HostsManager::get_hosts_path();
        assert!(path.to_str().is_some());
    }
}
