// ============================================================
// FILE:        lib.rs
// MODULE:      Layer 3 — Desktop UI Shell (Tauri backend library)
// TASK:        T-028
// PLATFORM:    cross
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// ============================================================

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// IPC request to the daemon
#[derive(Debug, Serialize, Deserialize)]
struct DaemonRequest {
    msg_type: String,
    payload: serde_json::Value,
}

/// IPC response from the daemon
#[derive(Debug, Serialize, Deserialize)]
struct DaemonResponse {
    status: String,
    payload: serde_json::Value,
}

/// Connect to the daemon IPC socket and send a request.
///
/// Protocol: 4-byte LE length prefix + MessagePack body (matching daemon/src/ipc_server.rs).
/// Falls back to JSON if rmp-serde is not available.
fn connect_and_send(request: &DaemonRequest) -> Result<DaemonResponse, String> {
    // Serialize request to MessagePack
    let body = rmp_serde::to_vec_named(request)
        .map_err(|e| format!("Serialize error: {}", e))?;

    // Connect to daemon
    #[cfg(windows)]
    let mut stream = {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        // Named pipe path (must match daemon/src/ipc_server.rs)
        let pipe_path = r"\\.\pipe\focusme_daemon";

        // Windows named pipe opened as a file
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(pipe_path)
            .map_err(|e| format!("Cannot connect to daemon at {}: {}", pipe_path, e))?
    };

    #[cfg(not(windows))]
    let mut stream = {
        use std::os::unix::net::UnixStream;
        let sock_path = "/var/run/focusme.sock";
        UnixStream::connect(sock_path)
            .map_err(|e| format!("Cannot connect to daemon at {}: {}", sock_path, e))?
    };

    // Send: 4-byte LE length + body
    let len_bytes = (body.len() as u32).to_le_bytes();
    stream.write_all(&len_bytes)
        .map_err(|e| format!("Write length error: {}", e))?;
    stream.write_all(&body)
        .map_err(|e| format!("Write body error: {}", e))?;

    // Read response: 4-byte LE length + body
    let mut resp_len_buf = [0u8; 4];
    stream.read_exact(&mut resp_len_buf)
        .map_err(|e| format!("Read response length error: {}", e))?;
    let resp_len = u32::from_le_bytes(resp_len_buf) as usize;

    if resp_len > 10 * 1024 * 1024 {
        return Err(format!("Response too large: {} bytes", resp_len));
    }

    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)
        .map_err(|e| format!("Read response body error: {}", e))?;

    // Deserialize response from MessagePack
    let response: DaemonResponse = rmp_serde::from_slice(&resp_buf)
        .map_err(|e| format!("Deserialize response error: {}", e))?;

    Ok(response)
}

/// Tauri command: send a message to the FocusMe daemon via IPC
#[tauri::command]
async fn send_to_daemon(msg_type: String, payload: serde_json::Value) -> Result<DaemonResponse, String> {
    let request = DaemonRequest { msg_type, payload };

    // Run IPC on a blocking thread to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || connect_and_send(&request))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
}

/// Tauri command: get daemon connection status
#[tauri::command]
async fn get_daemon_status() -> Result<DaemonResponse, String> {
    send_to_daemon("STATUS_REQUEST".into(), serde_json::json!({})).await
}

/// Tauri command: list all plans
#[tauri::command]
async fn list_plans() -> Result<DaemonResponse, String> {
    send_to_daemon("PLAN_LIST".into(), serde_json::json!({})).await
}

/// Tauri command: create a new plan
#[tauri::command]
async fn create_plan(plan: serde_json::Value) -> Result<DaemonResponse, String> {
    send_to_daemon("PLAN_CREATE".into(), plan).await
}

/// Tauri command: update an existing plan
#[tauri::command]
async fn update_plan(plan: serde_json::Value) -> Result<DaemonResponse, String> {
    send_to_daemon("PLAN_UPDATE".into(), plan).await
}

/// Tauri command: delete a plan
#[tauri::command]
async fn delete_plan(plan_id: String) -> Result<DaemonResponse, String> {
    send_to_daemon("PLAN_DELETE".into(), serde_json::json!({ "plan_id": plan_id })).await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            send_to_daemon,
            get_daemon_status,
            list_plans,
            create_plan,
            update_plan,
            delete_plan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
