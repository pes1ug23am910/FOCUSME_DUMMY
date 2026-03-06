// ============================================================
// FILE:        wfp_manager.rs
// MODULE:      Layer 1 — Enforcement Engine > Windows WFP Blocking
// TASK:        T-012 (implementation — Session 3)
// PLATFORM:    windows
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, Session 3 — real WFP FFI implementation
// DEPENDENCIES: windows-sys 0.52 (Win32_NetworkManagement_WindowsFilteringPlatform)
// TEST COVERAGE: IT-01 (WFP blocks redirected IPs)
// KNOWN LIMITATIONS: Requires admin privileges. Dynamic session = auto-cleanup
//                    on process exit. WFP callout for deep packet inspection
//                    is post-MVP (requires kernel driver).
// ANTI-CIRCUMVENTION: Defends against BT-04 (alternate browser),
//                     BT-05 (DoH bypass — partial via IP blocking),
//                     BT-06 (VPN bypass — partial via outbound IP block)
// ============================================================

use anyhow::{Context, Result, bail};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::mem;
use std::ptr;
use tracing::{info, warn, error, debug};

use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::*;

// ═══════════════════════════════════════════════════════════════
// Stable GUIDs for FocusMe WFP objects
// ═══════════════════════════════════════════════════════════════

/// FocusMe WFP Provider — {7F0C5501-E0A1-4B9A-8D2E-1A2B3C4D5E6F}
const FOCUSME_PROVIDER_KEY: GUID = GUID {
    data1: 0x7F0C5501,
    data2: 0xE0A1,
    data3: 0x4B9A,
    data4: [0x8D, 0x2E, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F],
};

/// FocusMe WFP SubLayer — {7F0C5502-E0A1-4B9A-8D2E-1A2B3C4D5E6F}
const FOCUSME_SUBLAYER_KEY: GUID = GUID {
    data1: 0x7F0C5502,
    data2: 0xE0A1,
    data3: 0x4B9A,
    data4: [0x8D, 0x2E, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F],
};

// ── Well-known WFP layer & condition GUIDs ──

/// FWPM_LAYER_ALE_AUTH_CONNECT_V4 — {c38d57d1-05a7-4c33-904f-7fbceee60e82}
const WFP_LAYER_ALE_CONNECT_V4: GUID = GUID {
    data1: 0xc38d57d1,
    data2: 0x05a7,
    data3: 0x4c33,
    data4: [0x90, 0x4f, 0x7f, 0xbc, 0xee, 0xe6, 0x0e, 0x82],
};

/// FWPM_LAYER_ALE_AUTH_CONNECT_V6 — {4a72393b-319f-44bc-84c3-ba54dcb3b6b4}
const WFP_LAYER_ALE_CONNECT_V6: GUID = GUID {
    data1: 0x4a72393b,
    data2: 0x319f,
    data3: 0x44bc,
    data4: [0x84, 0xc3, 0xba, 0x54, 0xdc, 0xb3, 0xb6, 0xb4],
};

/// FWPM_CONDITION_IP_REMOTE_ADDRESS — {b235ae9a-1d64-49b8-a44c-5ff3d9095045}
const WFP_COND_REMOTE_ADDR: GUID = GUID {
    data1: 0xb235ae9a,
    data2: 0x1d64,
    data3: 0x49b8,
    data4: [0xa4, 0x4c, 0x5f, 0xf3, 0xd9, 0x09, 0x50, 0x45],
};

// ── WFP API constants (defined locally to avoid version-gating issues) ──
const FM_SESSION_FLAG_DYNAMIC: u32 = 0x00000001;
const FM_FWP_ACTION_BLOCK: u32 = 0x00001001;
const FM_FWP_MATCH_EQUAL: i32 = 0;
const FM_FWP_DATA_UINT32: i32 = 3;
const FM_FWP_DATA_BYTE_ARRAY16: i32 = 11;
const FM_RPC_C_AUTHN_DEFAULT: u32 = 0xFFFFFFFF;
const FM_FWP_E_ALREADY_EXISTS: u32 = 0x80320009;

/// Known DoH (DNS-over-HTTPS) provider IP addresses.
/// Blocking these at the IP level mitigates BT-05 (DoH bypass).
/// See S-001 for context.
const DOH_PROVIDER_IPS: &[&str] = &[
    // Google DNS
    "8.8.8.8", "8.8.4.4",
    "2001:4860:4860::8888", "2001:4860:4860::8844",
    // Cloudflare DNS
    "1.1.1.1", "1.0.0.1",
    "2606:4700:4700::1111", "2606:4700:4700::1001",
    // Quad9
    "9.9.9.9", "149.112.112.112",
    // OpenDNS
    "208.67.222.222", "208.67.220.220",
    // NextDNS
    "45.90.28.0", "45.90.30.0",
];

// ═══════════════════════════════════════════════════════════════
// Helper
// ═══════════════════════════════════════════════════════════════

/// Encode a Rust &str as a null-terminated UTF-16 wide string for Win32 APIs
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0u16)).collect()
}

// ═══════════════════════════════════════════════════════════════
// WfpManager — real WFP engine wrapper
// ═══════════════════════════════════════════════════════════════

/// WfpManager manages Windows Filtering Platform (WFP) filters for network-level
/// IP blocking. Uses a **dynamic session** so all filters are automatically removed
/// when the daemon process exits (clean crash recovery).
///
/// ## Defense-in-Depth
/// Complements the HOSTS file approach (T-011):
/// - **HOSTS** blocks DNS resolution → browsers get NXDOMAIN
/// - **WFP** blocks IP connections → even hardcoded IPs are blocked
/// - **DoH IP block** → browsers can't bypass local DNS via encrypted DNS
pub struct WfpManager {
    /// Handle to the WFP engine (from FwpmEngineOpen0)
    engine_handle: HANDLE,
    /// Map of blocked IP → WFP filter ID (for selective removal)
    active_filter_ids: HashMap<IpAddr, u64>,
    /// Currently blocked IP set (for diffing on updates)
    blocked_ips: HashSet<IpAddr>,
    /// Filter IDs for DoH provider blocks (tracked separately)
    doh_filter_ids: Vec<u64>,
}

impl WfpManager {
    /// Initialize the WFP engine, register the FocusMe filter provider,
    /// and create a sublayer for all FocusMe filters.
    ///
    /// Uses `FWPM_SESSION_FLAG_DYNAMIC` so all objects are auto-removed on exit.
    /// Requires administrator privileges.
    pub fn new() -> Result<Self> {
        let engine_handle = Self::open_engine()
            .context("Failed to open WFP engine — is the daemon running as admin?")?;

        // Register provider + sublayer (idempotent — ignores ALREADY_EXISTS)
        Self::register_provider(engine_handle)?;
        Self::register_sublayer(engine_handle)?;

        info!("WFP engine initialized — dynamic session active");

        Ok(Self {
            engine_handle,
            active_filter_ids: HashMap::new(),
            blocked_ips: HashSet::new(),
            doh_filter_ids: Vec::new(),
        })
    }

    // ────────────────────────────────────────────────────
    // Engine lifecycle
    // ────────────────────────────────────────────────────

    /// Open a WFP engine handle with a dynamic session
    fn open_engine() -> Result<HANDLE> {
        unsafe {
            let mut session: FWPM_SESSION0 = mem::zeroed();
            session.flags = FM_SESSION_FLAG_DYNAMIC;

            let mut handle: HANDLE = 0;
            let rc = FwpmEngineOpen0(
                ptr::null(),            // local machine
                FM_RPC_C_AUTHN_DEFAULT, // default authentication
                ptr::null(),            // use process identity
                &session as *const _,
                &mut handle,
            );

            if rc != 0 {
                bail!("FwpmEngineOpen0 failed: HRESULT 0x{:08X}", rc);
            }

            debug!(handle = handle, "WFP engine opened");
            Ok(handle)
        }
    }

    /// Register FocusMe as a WFP filter provider
    fn register_provider(handle: HANDLE) -> Result<()> {
        let name_w = to_wide("FocusMe Enforcement");
        let desc_w = to_wide("FocusMe URL/IP blocking provider");

        unsafe {
            let mut provider: FWPM_PROVIDER0 = mem::zeroed();
            provider.providerKey = FOCUSME_PROVIDER_KEY;
            provider.displayData.name = name_w.as_ptr() as *mut u16;
            provider.displayData.description = desc_w.as_ptr() as *mut u16;

            let rc = FwpmProviderAdd0(handle, &provider as *const _, ptr::null_mut());
            match rc {
                0 => {
                    debug!("WFP provider registered");
                    Ok(())
                }
                rc if rc == FM_FWP_E_ALREADY_EXISTS => {
                    debug!("WFP provider already registered (reusing)");
                    Ok(())
                }
                _ => bail!("FwpmProviderAdd0 failed: 0x{:08X}", rc),
            }
        }
    }

    /// Register the FocusMe filter sublayer (priority 0x8000 = mid-high)
    fn register_sublayer(handle: HANDLE) -> Result<()> {
        let name_w = to_wide("FocusMe Blocking Layer");
        let desc_w = to_wide("Contains all FocusMe IP block filters");

        unsafe {
            let mut sublayer: FWPM_SUBLAYER0 = mem::zeroed();
            sublayer.subLayerKey = FOCUSME_SUBLAYER_KEY;
            sublayer.displayData.name = name_w.as_ptr() as *mut u16;
            sublayer.displayData.description = desc_w.as_ptr() as *mut u16;
            let mut pk = FOCUSME_PROVIDER_KEY;
            sublayer.providerKey = &mut pk;
            sublayer.weight = 0x8000; // Mid-high priority

            let rc = FwpmSubLayerAdd0(handle, &sublayer as *const _, ptr::null_mut());
            match rc {
                0 => {
                    debug!("WFP sublayer registered");
                    Ok(())
                }
                rc if rc == FM_FWP_E_ALREADY_EXISTS => {
                    debug!("WFP sublayer already registered (reusing)");
                    Ok(())
                }
                _ => bail!("FwpmSubLayerAdd0 failed: 0x{:08X}", rc),
            }
        }
    }

    // ────────────────────────────────────────────────────
    // Filter management
    // ────────────────────────────────────────────────────

    /// Add a WFP BLOCK filter for an IPv4 address on the ALE_AUTH_CONNECT_V4 layer.
    /// Returns the WFP-assigned filter ID.
    fn add_ipv4_block_filter(&self, ip: Ipv4Addr) -> Result<u64> {
        let name_w = to_wide(&format!("FocusMe Block {}", ip));
        let desc_w = to_wide("Blocks outbound connections to this IPv4 address");

        unsafe {
            // Condition: remote IP == target (host byte order per WFP convention)
            let mut condition: FWPM_FILTER_CONDITION0 = mem::zeroed();
            condition.fieldKey = WFP_COND_REMOTE_ADDR;
            condition.matchType = FM_FWP_MATCH_EQUAL;
            condition.conditionValue.r#type = FM_FWP_DATA_UINT32;
            condition.conditionValue.Anonymous.uint32 = u32::from(ip);

            // Filter struct
            let mut filter: FWPM_FILTER0 = mem::zeroed();
            filter.displayData.name = name_w.as_ptr() as *mut u16;
            filter.displayData.description = desc_w.as_ptr() as *mut u16;
            filter.layerKey = WFP_LAYER_ALE_CONNECT_V4;
            filter.subLayerKey = FOCUSME_SUBLAYER_KEY;
            filter.action.r#type = FM_FWP_ACTION_BLOCK;
            filter.numFilterConditions = 1;
            filter.filterCondition = &mut condition;
            let mut pk = FOCUSME_PROVIDER_KEY;
            filter.providerKey = &mut pk;
            // weight left zeroed (FWP_EMPTY) → default auto-weight

            let mut filter_id: u64 = 0;
            let rc = FwpmFilterAdd0(
                self.engine_handle,
                &filter as *const _,
                ptr::null_mut(),
                &mut filter_id,
            );

            if rc != 0 {
                bail!("FwpmFilterAdd0 (IPv4 {}) failed: 0x{:08X}", ip, rc);
            }

            debug!(ip = %ip, filter_id, "IPv4 block filter added");
            Ok(filter_id)
        }
    }

    /// Add a WFP BLOCK filter for an IPv6 address on the ALE_AUTH_CONNECT_V6 layer.
    fn add_ipv6_block_filter(&self, ip: Ipv6Addr) -> Result<u64> {
        let name_w = to_wide(&format!("FocusMe Block {}", ip));
        let desc_w = to_wide("Blocks outbound connections to this IPv6 address");

        unsafe {
            let mut byte_array = FWP_BYTE_ARRAY16 {
                byteArray16: ip.octets(),
            };

            let mut condition: FWPM_FILTER_CONDITION0 = mem::zeroed();
            condition.fieldKey = WFP_COND_REMOTE_ADDR;
            condition.matchType = FM_FWP_MATCH_EQUAL;
            condition.conditionValue.r#type = FM_FWP_DATA_BYTE_ARRAY16;
            condition.conditionValue.Anonymous.byteArray16 = &mut byte_array;

            let mut filter: FWPM_FILTER0 = mem::zeroed();
            filter.displayData.name = name_w.as_ptr() as *mut u16;
            filter.displayData.description = desc_w.as_ptr() as *mut u16;
            filter.layerKey = WFP_LAYER_ALE_CONNECT_V6;
            filter.subLayerKey = FOCUSME_SUBLAYER_KEY;
            filter.action.r#type = FM_FWP_ACTION_BLOCK;
            filter.numFilterConditions = 1;
            filter.filterCondition = &mut condition;
            let mut pk = FOCUSME_PROVIDER_KEY;
            filter.providerKey = &mut pk;

            let mut filter_id: u64 = 0;
            let rc = FwpmFilterAdd0(
                self.engine_handle,
                &filter as *const _,
                ptr::null_mut(),
                &mut filter_id,
            );

            if rc != 0 {
                bail!("FwpmFilterAdd0 (IPv6 {}) failed: 0x{:08X}", ip, rc);
            }

            debug!(ip = %ip, filter_id, "IPv6 block filter added");
            Ok(filter_id)
        }
    }

    /// Block outbound connections to a set of IP addresses.
    ///
    /// Performs a diff against the current state:
    /// - New IPs → add filters
    /// - Removed IPs → delete filters
    /// - Existing IPs → no change
    ///
    /// Uses WFP transactions for atomicity.
    pub fn block_ips(&mut self, new_ips: HashSet<IpAddr>) -> Result<()> {
        let to_add: HashSet<_> = new_ips.difference(&self.blocked_ips).cloned().collect();
        let to_remove: HashSet<_> = self.blocked_ips.difference(&new_ips).cloned().collect();

        if to_add.is_empty() && to_remove.is_empty() {
            return Ok(());
        }

        info!(
            adding = to_add.len(),
            removing = to_remove.len(),
            total = new_ips.len(),
            "Updating WFP IP block filters"
        );

        // Begin transaction for atomic batch update
        let rc = unsafe { FwpmTransactionBegin0(self.engine_handle, 0) };
        let in_transaction = rc == 0;
        if !in_transaction {
            warn!(rc, "FwpmTransactionBegin0 failed — proceeding without transaction");
        }

        // Remove obsolete filters
        for ip in &to_remove {
            if let Some(filter_id) = self.active_filter_ids.remove(ip) {
                let rc = unsafe { FwpmFilterDeleteById0(self.engine_handle, filter_id) };
                if rc != 0 {
                    warn!(ip = %ip, filter_id, rc, "Failed to remove WFP filter");
                } else {
                    debug!(ip = %ip, filter_id, "WFP filter removed");
                }
            }
        }

        // Add new filters
        for ip in &to_add {
            let result = match ip {
                IpAddr::V4(v4) => self.add_ipv4_block_filter(*v4),
                IpAddr::V6(v6) => self.add_ipv6_block_filter(*v6),
            };
            match result {
                Ok(filter_id) => {
                    self.active_filter_ids.insert(*ip, filter_id);
                }
                Err(e) => {
                    error!(ip = %ip, error = %e, "Failed to add WFP block filter");
                    // Continue with remaining IPs — partial success is better than none
                }
            }
        }

        // Commit transaction
        if in_transaction {
            let rc = unsafe { FwpmTransactionCommit0(self.engine_handle) };
            if rc != 0 {
                error!(rc, "FwpmTransactionCommit0 failed");
                let _ = unsafe { FwpmTransactionAbort0(self.engine_handle) };
            }
        }

        self.blocked_ips = new_ips;
        info!(active_filters = self.active_filter_ids.len(), "WFP filters updated");
        Ok(())
    }

    /// Remove all FocusMe WFP filters (domain-based + DoH).
    pub fn clear_filters(&mut self) -> Result<()> {
        info!(
            domain_filters = self.active_filter_ids.len(),
            doh_filters = self.doh_filter_ids.len(),
            "Clearing all WFP filters"
        );

        for (ip, filter_id) in self.active_filter_ids.drain() {
            unsafe {
                let rc = FwpmFilterDeleteById0(self.engine_handle, filter_id);
                if rc != 0 {
                    warn!(ip = %ip, filter_id, rc, "Failed to delete WFP filter");
                }
            }
        }

        for filter_id in self.doh_filter_ids.drain(..) {
            unsafe {
                let rc = FwpmFilterDeleteById0(self.engine_handle, filter_id);
                if rc != 0 {
                    warn!(filter_id, rc, "Failed to delete DoH WFP filter");
                }
            }
        }

        self.blocked_ips.clear();
        info!("All WFP filters cleared");
        Ok(())
    }

    /// Block known DoH provider IP addresses to prevent DNS-over-HTTPS bypass.
    ///
    /// ANTI-CIRCUMVENTION: Partial defense against BT-05 (DoH bypass).
    /// Blocks outbound connections to known public DoH resolvers so browsers
    /// cannot bypass local HOSTS file / DNS blocking via encrypted DNS.
    ///
    /// Addresses: S-001 (extension network-stack blocking)
    pub fn block_doh_providers(&mut self) -> Result<()> {
        let mut doh_ips = Vec::new();
        for ip_str in DOH_PROVIDER_IPS {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                doh_ips.push(ip);
            }
        }

        info!(count = doh_ips.len(), "Blocking known DoH provider IPs");

        for ip in &doh_ips {
            let result = match ip {
                IpAddr::V4(v4) => self.add_ipv4_block_filter(*v4),
                IpAddr::V6(v6) => self.add_ipv6_block_filter(*v6),
            };
            match result {
                Ok(filter_id) => {
                    self.doh_filter_ids.push(filter_id);
                    debug!(ip = %ip, filter_id, "DoH provider IP blocked");
                }
                Err(e) => {
                    warn!(ip = %ip, error = %e, "Failed to block DoH provider IP");
                }
            }
        }

        info!(
            blocked = self.doh_filter_ids.len(),
            "DoH provider blocking complete"
        );
        Ok(())
    }

    /// Resolve blocked domain names to IP addresses for WFP filtering.
    ///
    /// Performs async DNS resolution via tokio. Non-resolvable domains
    /// are logged and skipped (they are still blocked via HOSTS/DNS).
    pub async fn resolve_domains_to_ips(
        &self,
        domains: &HashSet<String>,
    ) -> Result<HashSet<IpAddr>> {
        let mut ips = HashSet::new();

        for domain in domains {
            match tokio::net::lookup_host(format!("{}:80", domain)).await {
                Ok(addrs) => {
                    for addr in addrs {
                        ips.insert(addr.ip());
                    }
                }
                Err(e) => {
                    warn!(
                        domain = %domain,
                        error = %e,
                        "DNS resolution failed for WFP (will rely on HOSTS)"
                    );
                }
            }
        }

        debug!(domains = domains.len(), resolved_ips = ips.len(), "Domain resolution complete");
        Ok(ips)
    }

    /// Graceful shutdown: remove all filters and close the WFP engine handle.
    ///
    /// Note: With `FWPM_SESSION_FLAG_DYNAMIC`, filters are auto-removed on
    /// engine close, but explicit cleanup provides better logging.
    pub fn shutdown(&mut self) -> Result<()> {
        info!("WFP manager shutting down");

        let _ = self.clear_filters();

        if self.engine_handle != 0 {
            let rc = unsafe { FwpmEngineClose0(self.engine_handle) };
            if rc != 0 {
                warn!(rc, "FwpmEngineClose0 returned non-zero");
            }
            self.engine_handle = 0;
            info!("WFP engine closed");
        }

        Ok(())
    }

    /// Get the count of currently active WFP filters
    pub fn active_filter_count(&self) -> usize {
        self.active_filter_ids.len() + self.doh_filter_ids.len()
    }

    /// Check if a specific IP is currently blocked by WFP
    pub fn is_ip_blocked(&self, ip: &IpAddr) -> bool {
        self.blocked_ips.contains(ip)
    }
}

/// Auto-cleanup: close engine handle and remove all filters on drop
impl Drop for WfpManager {
    fn drop(&mut self) {
        if self.engine_handle != 0 {
            if let Err(e) = self.shutdown() {
                error!(error = %e, "Error during WfpManager drop");
            }
        }
    }
}

// ============================================================
// UNIT TESTS
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_wide_null_terminated() {
        let wide = to_wide("Hello");
        assert_eq!(wide.len(), 6); // 5 chars + null terminator
        assert_eq!(wide[5], 0);
    }

    #[test]
    fn test_doh_ips_all_parse() {
        for ip_str in DOH_PROVIDER_IPS {
            let parsed = ip_str.parse::<IpAddr>();
            assert!(parsed.is_ok(), "Failed to parse DoH IP: {}", ip_str);
        }
    }

    #[test]
    fn test_guid_constants_non_zero() {
        assert_ne!(FOCUSME_PROVIDER_KEY.data1, 0);
        assert_ne!(FOCUSME_SUBLAYER_KEY.data1, 0);
        assert_ne!(WFP_LAYER_ALE_CONNECT_V4.data1, 0);
        assert_ne!(WFP_LAYER_ALE_CONNECT_V6.data1, 0);
        assert_ne!(WFP_COND_REMOTE_ADDR.data1, 0);
    }

    #[test]
    fn test_ipv4_host_byte_order() {
        let ip = Ipv4Addr::new(8, 8, 8, 8);
        let val = u32::from(ip);
        // 8.8.8.8 in host byte order = 0x08080808
        assert_eq!(val, 0x08080808);
    }

    #[test]
    fn test_ipv6_octets() {
        let ip: Ipv6Addr = "2001:4860:4860::8888".parse().unwrap();
        let octets = ip.octets();
        assert_eq!(octets.len(), 16);
        assert_eq!(octets[0], 0x20);
        assert_eq!(octets[1], 0x01);
    }

    // NOTE: Functional WFP tests require admin privileges and run in
    // the CI integration test suite (tests/it_wfp.rs), not here.
}
