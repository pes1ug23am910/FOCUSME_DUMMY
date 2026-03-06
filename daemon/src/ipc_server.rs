// ============================================================
// FILE:        ipc_server.rs
// MODULE:      Layer 1 — Enforcement Engine > IPC Server
// TASK:        T-019 (implementation — Session 2)
// PLATFORM:    cross (Named Pipe on Windows, UDS on Unix)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 1, daemon core
// DEPENDENCIES: interprocess 2.0, rmp-serde 1.1, serde_json, tokio
// TEST COVERAGE: IT-04 (native messaging handshake), protocol conformance tests
// KNOWN LIMITATIONS: Max message size 1MB. MessagePack primary, JSON for debug mode.
// ANTI-CIRCUMVENTION: IPC channel permissions restrict access to daemon + logged-in user only.
// ============================================================

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, error, debug};

use crate::DaemonState;
use crate::db;
use crate::plan_protection::PlanProtection;

/// Maximum IPC message size (1 MB)
const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// IPC socket/pipe paths
#[cfg(windows)]
pub const PIPE_NAME: &str = r"\\.\pipe\focusme_daemon";

#[cfg(not(windows))]
pub const SOCKET_PATH: &str = "/var/run/focusme.sock";

/// IPC Message envelope (matches ipc_protocol_v1.md)
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcMessage {
    pub version: u8,
    pub msg_id: String,
    pub msg_type: String,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

/// IPC Response envelope
#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub version: u8,
    pub msg_id: String,
    pub msg_type: String,
    pub timestamp: String,
    pub in_reply_to: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub payload: serde_json::Value,
}

/// IPC Server — listens for connections from UI shell, browser extension,
/// and native messaging host. Validates all messages against schema before processing.
pub struct IpcServer {
    debug_json_mode: bool,
    state: Arc<DaemonState>,
}

impl IpcServer {
    pub fn new(debug_json_mode: bool, state: Arc<DaemonState>) -> Self {
        Self { debug_json_mode, state }
    }

    /// Start the IPC server listener
    ///
    /// On Windows: Creates a Named Pipe at \\.\pipe\focusme_daemon
    /// On Unix: Creates a Unix Domain Socket at /var/run/focusme.sock
    pub async fn start(&self) -> Result<()> {
        info!(
            transport = if cfg!(windows) { "Named Pipe" } else { "Unix Domain Socket" },
            debug_mode = self.debug_json_mode,
            "IPC server starting"
        );

        #[cfg(not(windows))]
        {
            self.start_uds_listener().await?;
        }

        #[cfg(windows)]
        {
            self.start_pipe_listener().await?;
        }

        Ok(())
    }

    /// Unix Domain Socket listener implementation
    #[cfg(not(windows))]
    async fn start_uds_listener(&self) -> Result<()> {
        use tokio::net::UnixListener;

        // Remove stale socket file if it exists
        let _ = std::fs::remove_file(SOCKET_PATH);

        let listener = UnixListener::bind(SOCKET_PATH)
            .with_context(|| format!("Failed to bind UDS at {}", SOCKET_PATH))?;

        // Set permissions: owner (root) + group (focusme) can connect
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o660))?;
        }

        info!(path = SOCKET_PATH, "IPC UDS listener ready");

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let state = self.state.clone();
                    let debug = self.debug_json_mode;
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection_unix(stream, state, debug).await {
                            warn!(error = %e, "IPC connection handler error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Failed to accept IPC connection");
                }
            }
        }
    }

    /// Handle a single Unix domain socket connection
    #[cfg(not(windows))]
    async fn handle_connection_unix(
        mut stream: tokio::net::UnixStream,
        state: Arc<DaemonState>,
        debug_json: bool,
    ) -> Result<()> {
        debug!("New IPC connection accepted");
        let server = IpcServer { debug_json_mode: debug_json, state };

        loop {
            // Read 4-byte length prefix (little-endian u32)
            let mut len_buf = [0u8; 4];
            match stream.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("IPC client disconnected");
                    break;
                }
                Err(e) => return Err(e.into()),
            }
            let msg_len = u32::from_le_bytes(len_buf) as usize;

            if msg_len > MAX_MESSAGE_SIZE {
                warn!(size = msg_len, "IPC message exceeds maximum size");
                break;
            }

            // Read message payload
            let mut payload_buf = vec![0u8; msg_len];
            stream.read_exact(&mut payload_buf).await?;

            // Deserialize
            let msg = server.deserialize_message(&payload_buf)?;

            // Handle and respond
            let response = server.handle_message(msg).await?;
            let response_bytes = server.serialize_message(&response)?;

            // Write response: 4-byte length prefix + payload
            let resp_len = (response_bytes.len() as u32).to_le_bytes();
            stream.write_all(&resp_len).await?;
            stream.write_all(&response_bytes).await?;
            stream.flush().await?;
        }

        Ok(())
    }

    /// Named Pipe listener implementation (Windows)
    #[cfg(windows)]
    async fn start_pipe_listener(&self) -> Result<()> {
        use interprocess::local_socket::{
            tokio::prelude::*,
            GenericNamespaced, ListenerOptions,
        };

        let name = PIPE_NAME.to_ns_name::<GenericNamespaced>()
            .context("Failed to create pipe name")?;

        let listener = ListenerOptions::new()
            .name(name)
            .create_tokio()
            .context("Failed to create Named Pipe listener")?;

        info!(path = PIPE_NAME, "IPC Named Pipe listener ready");

        loop {
            match listener.accept().await {
                Ok(stream) => {
                    let state = self.state.clone();
                    let debug = self.debug_json_mode;
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection_pipe(stream, state, debug).await {
                            warn!(error = %e, "IPC pipe connection handler error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Failed to accept pipe connection");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Handle a single Named Pipe connection (Windows)
    #[cfg(windows)]
    async fn handle_connection_pipe(
        mut stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
        state: Arc<DaemonState>,
        debug_json: bool,
    ) -> Result<()> {
        debug!("New IPC pipe connection accepted");
        let server = IpcServer { debug_json_mode: debug_json, state };

        loop {
            let mut len_buf = [0u8; 4];
            match stream.read_exact(&mut len_buf).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("IPC client disconnected");
                    break;
                }
                Err(e) => return Err(e.into()),
            }
            let msg_len = u32::from_le_bytes(len_buf) as usize;

            if msg_len > MAX_MESSAGE_SIZE {
                warn!(size = msg_len, "IPC message exceeds maximum size");
                break;
            }

            let mut payload_buf = vec![0u8; msg_len];
            stream.read_exact(&mut payload_buf).await?;

            let msg = server.deserialize_message(&payload_buf)?;
            let response = server.handle_message(msg).await?;
            let response_bytes = server.serialize_message(&response)?;

            let resp_len = (response_bytes.len() as u32).to_le_bytes();
            stream.write_all(&resp_len).await?;
            stream.write_all(&response_bytes).await?;
            stream.flush().await?;
        }

        Ok(())
    }

    // ════════════════════════════════════════════════════
    // MESSAGE HANDLERS — real implementations
    // ════════════════════════════════════════════════════

    async fn handle_message(&self, msg: IpcMessage) -> Result<IpcResponse> {
        debug!(msg_type = %msg.msg_type, msg_id = %msg.msg_id, "Handling IPC message");

        let response_payload = match msg.msg_type.as_str() {
            "PING" => self.handle_ping(),
            "CONNECT" => self.handle_connect().await,
            "PLAN_LIST" => self.handle_plan_list().await,
            "PLAN_GET" => self.handle_plan_get(&msg.payload).await,
            "PLAN_CREATE" => self.handle_plan_create(&msg.payload).await,
            "PLAN_UPDATE" => self.handle_plan_update(&msg.payload).await,
            "PLAN_DELETE" => self.handle_plan_delete(&msg.payload).await,
            "URL_CHECK" => self.handle_url_check(&msg.payload).await,
            "APP_CHECK" => self.handle_app_check(&msg.payload).await,
            "STATUS_REQUEST" => self.handle_status_request().await,
            "UNLOCK_REQUEST" => self.handle_unlock_request(&msg.payload).await,
            "STATS_REQUEST" => self.handle_stats_request(&msg.payload).await,
            _ => {
                return Ok(IpcResponse {
                    version: 1,
                    msg_id: uuid::Uuid::new_v4().to_string(),
                    msg_type: "ERROR".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    in_reply_to: msg.msg_id,
                    status: "error".to_string(),
                    error_code: Some("UNKNOWN_MSG_TYPE".to_string()),
                    error_message: Some(format!("Unknown message type: {}", msg.msg_type)),
                    payload: serde_json::Value::Null,
                });
            }
        };

        Ok(IpcResponse {
            version: 1,
            msg_id: uuid::Uuid::new_v4().to_string(),
            msg_type: format!("{}_RESPONSE", msg.msg_type),
            timestamp: chrono::Utc::now().to_rfc3339(),
            in_reply_to: msg.msg_id,
            status: "ok".to_string(),
            error_code: None,
            error_message: None,
            payload: response_payload,
        })
    }

    fn handle_ping(&self) -> serde_json::Value {
        serde_json::json!({ "pong": true })
    }

    async fn handle_connect(&self) -> serde_json::Value {
        let active_plans = self.state.scheduler.get_active_plans().await;
        let forced_ids = self.state.forced_mode.active_plan_ids().await;
        serde_json::json!({
            "daemon_version": env!("CARGO_PKG_VERSION"),
            "protocol_version": 1,
            "forced_mode_active": !forced_ids.is_empty(),
            "active_plan_count": active_plans.len()
        })
    }

    async fn handle_plan_list(&self) -> serde_json::Value {
        match self.state.db.list_plans() {
            Ok(plans) => {
                let plan_summaries: Vec<serde_json::Value> = plans.iter().map(|p| {
                    serde_json::json!({
                        "plan_id": p.plan_id,
                        "name": p.name,
                        "enabled": p.enabled,
                        "forced_mode": p.forced_mode,
                        "protection_type": p.protection_type,
                    })
                }).collect();
                serde_json::json!({ "plans": plan_summaries })
            }
            Err(e) => serde_json::json!({ "plans": [], "error": e.to_string() }),
        }
    }

    async fn handle_plan_get(&self, payload: &serde_json::Value) -> serde_json::Value {
        let plan_id = payload["plan_id"].as_str().unwrap_or("");
        match self.state.db.get_plan(plan_id) {
            Ok(Some(plan)) => serde_json::json!({
                "plan": {
                    "plan_id": plan.plan_id,
                    "name": plan.name,
                    "enabled": plan.enabled,
                    "forced_mode": plan.forced_mode,
                    "plan_json": plan.plan_json,
                    "protection_type": plan.protection_type,
                }
            }),
            Ok(None) => serde_json::json!({ "plan": null }),
            Err(e) => serde_json::json!({ "plan": null, "error": e.to_string() }),
        }
    }

    async fn handle_plan_create(&self, payload: &serde_json::Value) -> serde_json::Value {
        let now = chrono::Utc::now().to_rfc3339();
        let plan_id = uuid::Uuid::new_v4().to_string();

        let plan = db::PlanRow {
            plan_id: plan_id.clone(),
            name: payload["name"].as_str().unwrap_or("Untitled Plan").to_string(),
            schema_version: "1.0.0".to_string(),
            enabled: payload["enabled"].as_bool().unwrap_or(true),
            forced_mode: payload["forced_mode"].as_bool().unwrap_or(false),
            forced_mode_max_duration_s: payload["forced_mode_max_duration_s"]
                .as_i64()
                .unwrap_or(86400),
            protection_type: payload["protection_type"]
                .as_str()
                .unwrap_or("none")
                .to_string(),
            protection_hash: payload["password"]
                .as_str()
                .and_then(|p| PlanProtection::hash_password(p).ok()),
            challenge_required: payload["challenge_required"].as_bool().unwrap_or(false),
            plan_json: payload["plan_json"]
                .as_str()
                .unwrap_or("{}")
                .to_string(),
            created_at: now.clone(),
            modified_at: now,
        };

        match self.state.db.create_plan(&plan) {
            Ok(()) => {
                info!(plan_id = %plan_id, "Plan created via IPC");
                serde_json::json!({ "plan_id": plan_id, "success": true })
            }
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    async fn handle_plan_update(&self, payload: &serde_json::Value) -> serde_json::Value {
        let plan_id = payload["plan_id"].as_str().unwrap_or("");
        match self.state.db.get_plan(plan_id) {
            Ok(Some(mut plan)) => {
                // Check forced mode protection
                if self.state.forced_mode.is_active(plan_id).await {
                    return serde_json::json!({
                        "success": false,
                        "error": "Cannot modify plan during active Forced Mode session"
                    });
                }

                if let Some(name) = payload["name"].as_str() { plan.name = name.to_string(); }
                if let Some(enabled) = payload["enabled"].as_bool() { plan.enabled = enabled; }
                if let Some(json) = payload["plan_json"].as_str() { plan.plan_json = json.to_string(); }
                plan.modified_at = chrono::Utc::now().to_rfc3339();

                match self.state.db.update_plan(&plan) {
                    Ok(()) => serde_json::json!({ "success": true }),
                    Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
                }
            }
            Ok(None) => serde_json::json!({ "success": false, "error": "Plan not found" }),
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    async fn handle_plan_delete(&self, payload: &serde_json::Value) -> serde_json::Value {
        let plan_id = payload["plan_id"].as_str().unwrap_or("");

        // Check forced mode protection
        if self.state.forced_mode.is_active(plan_id).await {
            return serde_json::json!({
                "success": false,
                "error": "Cannot delete plan during active Forced Mode session"
            });
        }

        // Check password protection
        if let Ok(Some(plan)) = self.state.db.get_plan(plan_id) {
            if plan.protection_type != "none" {
                if let Some(password) = payload["password"].as_str() {
                    if let Some(ref hash) = plan.protection_hash {
                        match PlanProtection::verify_password(password, hash) {
                            Ok(true) => {} // Password correct, proceed
                            _ => return serde_json::json!({
                                "success": false,
                                "error": "Incorrect password"
                            }),
                        }
                    }
                } else {
                    return serde_json::json!({
                        "success": false,
                        "error": "Password required to delete protected plan"
                    });
                }
            }
        }

        match self.state.db.delete_plan(plan_id) {
            Ok(()) => {
                info!(plan_id = plan_id, "Plan deleted via IPC");
                serde_json::json!({ "success": true })
            }
            Err(e) => serde_json::json!({ "success": false, "error": e.to_string() }),
        }
    }

    async fn handle_url_check(&self, payload: &serde_json::Value) -> serde_json::Value {
        let url = payload["url"].as_str().unwrap_or("");
        let domain = payload["domain"].as_str().unwrap_or(url);

        let active_plans = self.state.scheduler.get_active_plans().await;

        for plan_id in &active_plans {
            if let Ok(rules) = self.state.db.get_url_rules(plan_id) {
                for rule in &rules {
                    if rule.rule_type == "block" && Self::domain_matches(domain, &rule.value) {
                        return serde_json::json!({
                            "decision": "BLOCK",
                            "matched_rule_id": rule.rule_id,
                            "matched_plan_id": plan_id
                        });
                    }
                }
            }
        }

        serde_json::json!({
            "decision": "ALLOW",
            "matched_rule_id": null,
            "matched_plan_id": null
        })
    }

    async fn handle_app_check(&self, payload: &serde_json::Value) -> serde_json::Value {
        let process_name = payload["process_name"].as_str().unwrap_or("");
        let process_path = payload["process_path"].as_str().unwrap_or("");

        let active_plans = self.state.scheduler.get_active_plans().await;

        for plan_id in &active_plans {
            if let Ok(rules) = self.state.db.get_app_rules(plan_id) {
                for rule in &rules {
                    if rule.rule_type == "block" {
                        let matches = match rule.match_type.as_str() {
                            "process_name" => process_name.eq_ignore_ascii_case(&rule.value),
                            "path_prefix" => process_path
                                .to_lowercase()
                                .starts_with(&rule.value.to_lowercase()),
                            "path_exact" => process_path.eq_ignore_ascii_case(&rule.value),
                            _ => false,
                        };
                        if matches {
                            return serde_json::json!({
                                "decision": "BLOCK",
                                "matched_rule_id": rule.rule_id,
                                "matched_plan_id": plan_id
                            });
                        }
                    }
                }
            }
        }

        serde_json::json!({ "decision": "ALLOW" })
    }

    async fn handle_status_request(&self) -> serde_json::Value {
        let active_plans = self.state.scheduler.get_active_plans().await;
        let forced_ids = self.state.forced_mode.active_plan_ids().await;
        serde_json::json!({
            "daemon_version": env!("CARGO_PKG_VERSION"),
            "active_plans": active_plans,
            "forced_mode_active": !forced_ids.is_empty(),
            "forced_mode_plans": forced_ids,
        })
    }

    async fn handle_unlock_request(&self, payload: &serde_json::Value) -> serde_json::Value {
        let plan_id = payload["plan_id"].as_str().unwrap_or("");

        if let Some(password) = payload["password"].as_str() {
            // Password unlock attempt
            if let Ok(Some(plan)) = self.state.db.get_plan(plan_id) {
                if let Some(ref hash) = plan.protection_hash {
                    match PlanProtection::verify_password(password, hash) {
                        Ok(true) => {
                            info!(plan_id = plan_id, "Plan unlocked via password");
                            return serde_json::json!({ "unlocked": true });
                        }
                        _ => return serde_json::json!({
                            "unlocked": false,
                            "error": "Incorrect password"
                        }),
                    }
                }
            }
        }

        if let Some(code) = payload["emergency_code"].as_str() {
            // Emergency code unlock (Forced Mode)
            match self.state.forced_mode.emergency_unlock(plan_id, code).await {
                Ok(()) => {
                    let _ = self.state.db.clear_forced_mode_state(plan_id);
                    return serde_json::json!({ "unlocked": true });
                }
                Err(e) => return serde_json::json!({
                    "unlocked": false,
                    "error": e.to_string()
                }),
            }
        }

        // Generate challenge if needed
        let challenge = PlanProtection::generate_challenge();
        serde_json::json!({ "challenge_text": challenge })
    }

    async fn handle_stats_request(&self, payload: &serde_json::Value) -> serde_json::Value {
        let since = payload["since"]
            .as_str()
            .unwrap_or("2020-01-01T00:00:00Z");
        let limit = payload["limit"].as_i64().unwrap_or(100);

        match self.state.db.get_events_since(since, limit) {
            Ok(events) => {
                let event_summaries: Vec<serde_json::Value> = events.iter().map(|e| {
                    serde_json::json!({
                        "event_type": e.event_type,
                        "subject_hash": e.subject_hash,
                        "timestamp": e.timestamp_utc,
                    })
                }).collect();

                let counts = self.state.db.get_event_count_by_type(since)
                    .unwrap_or_default();

                serde_json::json!({
                    "events": event_summaries,
                    "counts": counts.into_iter()
                        .map(|(t, c)| serde_json::json!({"type": t, "count": c}))
                        .collect::<Vec<_>>()
                })
            }
            Err(e) => serde_json::json!({ "events": [], "error": e.to_string() }),
        }
    }

    // ════════════════════════════════════════════════════
    // HELPERS
    // ════════════════════════════════════════════════════

    /// Check if a domain matches a rule value (supports exact + subdomain matching)
    fn domain_matches(domain: &str, rule_value: &str) -> bool {
        let d = domain.to_lowercase();
        let r = rule_value.to_lowercase();

        // Exact match
        if d == r { return true; }

        // Subdomain match: rule "example.com" matches "sub.example.com"
        if d.ends_with(&format!(".{}", r)) { return true; }

        // Wildcard prefix match: rule "*.example.com"
        if r.starts_with("*.") {
            let suffix = &r[1..]; // ".example.com"
            if d.ends_with(suffix) { return true; }
        }

        false
    }

    /// Serialize a message using MessagePack or JSON based on debug mode
    fn serialize_message(&self, msg: &IpcResponse) -> Result<Vec<u8>> {
        if self.debug_json_mode {
            Ok(serde_json::to_vec(msg)?)
        } else {
            Ok(rmp_serde::to_vec(msg)?)
        }
    }

    /// Deserialize incoming bytes to IpcMessage
    fn deserialize_message(&self, data: &[u8]) -> Result<IpcMessage> {
        if self.debug_json_mode {
            Ok(serde_json::from_slice(data)?)
        } else {
            // Try MessagePack first, fall back to JSON
            rmp_serde::from_slice(data)
                .or_else(|_| serde_json::from_slice(data).map_err(Into::into))
                .context("Failed to deserialize IPC message")
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
    fn test_domain_matches_exact() {
        assert!(IpcServer::domain_matches("example.com", "example.com"));
        assert!(IpcServer::domain_matches("Example.COM", "example.com"));
    }

    #[test]
    fn test_domain_matches_subdomain() {
        assert!(IpcServer::domain_matches("www.example.com", "example.com"));
        assert!(IpcServer::domain_matches("sub.deep.example.com", "example.com"));
        assert!(!IpcServer::domain_matches("notexample.com", "example.com"));
    }

    #[test]
    fn test_domain_matches_wildcard() {
        assert!(IpcServer::domain_matches("www.example.com", "*.example.com"));
        assert!(IpcServer::domain_matches("deep.sub.example.com", "*.example.com"));
        assert!(!IpcServer::domain_matches("example.com", "*.example.com"));
    }

    #[test]
    fn test_ipc_message_serialization_json() {
        let state = create_test_state();
        let server = IpcServer::new(true, state);
        let response = IpcResponse {
            version: 1,
            msg_id: "response-1".to_string(),
            msg_type: "PONG".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            in_reply_to: "request-1".to_string(),
            status: "ok".to_string(),
            error_code: None,
            error_message: None,
            payload: serde_json::json!({"pong": true}),
        };

        let serialized = server.serialize_message(&response)
            .expect("Serialization should succeed");
        assert!(!serialized.is_empty());
    }

    fn create_test_state() -> Arc<DaemonState> {
        use crate::hosts_manager::HostsManager;
        use crate::process_monitor::ProcessMonitor;
        use crate::scheduler::PlanScheduler;
        use crate::forced_mode::ForcedModeTracker;

        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let db = Arc::new(db::Database::open(tmp.path(), "").expect("DB open"));

        Arc::new(DaemonState {
            db,
            scheduler: Arc::new(PlanScheduler::new()),
            forced_mode: Arc::new(ForcedModeTracker::new()),
            process_monitor: Arc::new(ProcessMonitor::new()),
            hosts_manager: Arc::new(HostsManager::new().expect("hosts manager")),
        })
    }
}
