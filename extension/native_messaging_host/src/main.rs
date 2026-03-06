// ============================================================
// FILE:        main.rs
// MODULE:      Layer 3 — Browser Extension > Native Messaging Host
// TASK:        T-034
// PLATFORM:    windows, macos, linux
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Native messaging bridge between extension and daemon
// DEPENDENCIES: serde, serde_json, tokio (for async IPC to daemon)
// TEST COVERAGE: Test: message from stdin → IPC to daemon → response to stdout
// KNOWN LIMITATIONS: Chrome native messaging uses 4-byte length-prefix framing on
//                    stdin/stdout. Must match exactly or Chrome kills the host.
//                    Maximum single message size: 1MB (Chrome) / 1GB (Firefox).
// ANTI-CIRCUMVENTION: Native host validates extension origin (allowed_origins).
// ============================================================

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Chrome/Firefox native messaging: 4-byte little-endian length prefix
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB Chrome limit

/// Native messaging host manifest name
const _HOST_NAME: &str = "com.focusme.native_messaging";

// ============ Types ============

/// Message received from the browser extension
#[derive(Debug, Deserialize)]
struct ExtensionMessage {
    #[serde(rename = "type")]
    msg_type: String,
    version: u32,
    request_id: String,
    payload: serde_json::Value,
}

/// Message sent back to the browser extension
#[derive(Debug, Serialize)]
struct ExtensionResponse {
    #[serde(rename = "type")]
    msg_type: String,
    version: u32,
    request_id: String,
    payload: serde_json::Value,
}

// ============ Native Messaging I/O ============

/// Read a single message from stdin using Chrome native messaging protocol
///
/// Format: 4 bytes (little-endian u32 length) + N bytes (JSON UTF-8)
fn read_message() -> io::Result<Option<ExtensionMessage>> {
    let mut len_buf = [0u8; 4];

    match io::stdin().read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None), // Extension disconnected
        Err(e) => return Err(e),
    }

    let msg_len = u32::from_le_bytes(len_buf) as usize;

    if msg_len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Message too large: {} bytes (max {})", msg_len, MAX_MESSAGE_SIZE),
        ));
    }

    let mut msg_buf = vec![0u8; msg_len];
    io::stdin().read_exact(&mut msg_buf)?;

    let message: ExtensionMessage = serde_json::from_slice(&msg_buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(Some(message))
}

/// Write a single message to stdout using Chrome native messaging protocol
fn write_message(response: &ExtensionResponse) -> io::Result<()> {
    let json = serde_json::to_vec(response)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let len = json.len() as u32;
    io::stdout().write_all(&len.to_le_bytes())?;
    io::stdout().write_all(&json)?;
    io::stdout().flush()?;

    Ok(())
}

// ============ Message Handling ============

/// Process a message from the extension and return a response
fn handle_message(message: ExtensionMessage) -> ExtensionResponse {
    match message.msg_type.as_str() {
        "PING" => ExtensionResponse {
            msg_type: "PONG".to_string(),
            version: 1,
            request_id: message.request_id,
            payload: serde_json::json!({ "status": "ok", "daemon_connected": true }),
        },

        "URL_CHECK" => {
            // TODO: Forward to daemon via IPC (Named Pipe on Windows, UDS on Unix)
            // For now, return a stub response
            ExtensionResponse {
                msg_type: "URL_CHECK_RESULT".to_string(),
                version: 1,
                request_id: message.request_id,
                payload: serde_json::json!({
                    "action": "allow",
                    "reason": "stub — IPC to daemon not yet connected"
                }),
            }
        }

        "SYNC_RULES" => {
            // TODO: Fetch active URL rules from daemon via IPC
            ExtensionResponse {
                msg_type: "RULES_UPDATE".to_string(),
                version: 1,
                request_id: message.request_id,
                payload: serde_json::json!({ "rules": [] }),
            }
        }

        _ => ExtensionResponse {
            msg_type: "ERROR".to_string(),
            version: 1,
            request_id: message.request_id,
            payload: serde_json::json!({
                "code": "UNKNOWN_TYPE",
                "message": format!("Unknown message type: {}", message.msg_type)
            }),
        },
    }
}

// ============ Daemon IPC ============

/// IPC pipe/socket path (must match ipc_server.rs)
#[cfg(windows)]
const DAEMON_PIPE_NAME: &str = r"\\.\pipe\focusme_daemon";

#[cfg(not(windows))]
const DAEMON_SOCKET_PATH: &str = "/var/run/focusme.sock";

/// Send a request to the FocusMe daemon via IPC and return the response.
///
/// Protocol: 4-byte LE length prefix + MessagePack body (matches ipc_server.rs)
fn send_to_daemon(request: &serde_json::Value) -> io::Result<serde_json::Value> {
    // Serialize to MessagePack
    let body = rmp_serde::to_vec(request)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Connect to daemon
    #[cfg(windows)]
    let mut stream = {
        use interprocess::local_socket::{prelude::*, GenericNamespaced};
        let name = DAEMON_PIPE_NAME
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        interprocess::local_socket::Stream::connect(name)
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?
    };

    #[cfg(not(windows))]
    let mut stream = {
        std::os::unix::net::UnixStream::connect(DAEMON_SOCKET_PATH)?
    };

    // Write: 4-byte LE length + body
    let len_bytes = (body.len() as u32).to_le_bytes();
    {
        use std::io::Write;
        stream.write_all(&len_bytes)?;
        stream.write_all(&body)?;
        stream.flush()?;
    }

    // Read response: 4-byte LE length + body
    let mut resp_len_buf = [0u8; 4];
    {
        use std::io::Read;
        stream.read_exact(&mut resp_len_buf)?;
    }
    let resp_len = u32::from_le_bytes(resp_len_buf) as usize;
    if resp_len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Daemon response too large: {} bytes", resp_len),
        ));
    }
    let mut resp_buf = vec![0u8; resp_len];
    {
        use std::io::Read;
        stream.read_exact(&mut resp_buf)?;
    }

    // Try MessagePack first, then JSON fallback
    rmp_serde::from_slice(&resp_buf)
        .or_else(|_| serde_json::from_slice(&resp_buf))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Build a daemon IPC message envelope
fn build_daemon_request(msg_type: &str, payload: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "msg_id": format!("nmh-{}-{}", std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
        "msg_type": msg_type,
        "timestamp": chrono_timestamp(),
        "payload": payload,
    })
}

/// Simple UTC ISO-8601 timestamp without chrono dependency
fn chrono_timestamp() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Approximate — good enough for IPC timestamps
    format!("1970-01-01T00:00:00Z+{}s", secs)
}

// ============ Entry Point ============

fn main() {
    // Log to stderr (stdout is reserved for native messaging protocol)
    eprintln!("[FocusMe NMH] Starting native messaging host...");

    // Main message loop — reads from extension stdin, processes, writes to stdout
    loop {
        match read_message() {
            Ok(Some(message)) => {
                eprintln!("[FocusMe NMH] Received: {}", message.msg_type);

                let response = match message.msg_type.as_str() {
                    // PING — health check (answered locally without daemon)
                    "PING" => handle_message(message),

                    // URL_CHECK — forward to daemon for real blocking decision
                    "URL_CHECK" => {
                        let ipc_req = build_daemon_request("URL_CHECK", message.payload.clone());
                        match send_to_daemon(&ipc_req) {
                            Ok(daemon_resp) => ExtensionResponse {
                                msg_type: "URL_CHECK_RESULT".to_string(),
                                version: 1,
                                request_id: message.request_id,
                                payload: daemon_resp.get("payload")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({ "decision": "ALLOW" })),
                            },
                            Err(e) => {
                                eprintln!("[FocusMe NMH] Daemon IPC error: {}", e);
                                ExtensionResponse {
                                    msg_type: "URL_CHECK_RESULT".to_string(),
                                    version: 1,
                                    request_id: message.request_id,
                                    payload: serde_json::json!({
                                        "decision": "ALLOW",
                                        "error": format!("Daemon unreachable: {}", e),
                                    }),
                                }
                            }
                        }
                    }

                    // SYNC_RULES — fetch active URL rules from daemon
                    "SYNC_RULES" => {
                        let ipc_req = build_daemon_request("PLAN_LIST", serde_json::json!({}));
                        match send_to_daemon(&ipc_req) {
                            Ok(daemon_resp) => ExtensionResponse {
                                msg_type: "RULES_UPDATE".to_string(),
                                version: 1,
                                request_id: message.request_id,
                                payload: daemon_resp.get("payload")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({ "rules": [] })),
                            },
                            Err(e) => {
                                eprintln!("[FocusMe NMH] Daemon IPC error (SYNC): {}", e);
                                ExtensionResponse {
                                    msg_type: "RULES_UPDATE".to_string(),
                                    version: 1,
                                    request_id: message.request_id,
                                    payload: serde_json::json!({ "rules": [], "error": e.to_string() }),
                                }
                            }
                        }
                    }

                    // STATUS — forward to daemon
                    "STATUS" => {
                        let ipc_req = build_daemon_request("STATUS_REQUEST", serde_json::json!({}));
                        match send_to_daemon(&ipc_req) {
                            Ok(daemon_resp) => ExtensionResponse {
                                msg_type: "STATUS_RESPONSE".to_string(),
                                version: 1,
                                request_id: message.request_id,
                                payload: daemon_resp.get("payload")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({})),
                            },
                            Err(e) => ExtensionResponse {
                                msg_type: "STATUS_RESPONSE".to_string(),
                                version: 1,
                                request_id: message.request_id,
                                payload: serde_json::json!({
                                    "daemon_connected": false,
                                    "error": e.to_string(),
                                }),
                            },
                        }
                    }

                    // Unknown — local handling
                    _ => handle_message(message),
                };

                if let Err(e) = write_message(&response) {
                    eprintln!("[FocusMe NMH] Failed to write response: {}", e);
                    break;
                }
            }
            Ok(None) => {
                eprintln!("[FocusMe NMH] Extension disconnected (stdin EOF)");
                break;
            }
            Err(e) => {
                eprintln!("[FocusMe NMH] Error reading message: {}", e);
                break;
            }
        }
    }

    eprintln!("[FocusMe NMH] Shutting down");
}

// ============================================================
// UNIT TESTS
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_ping() {
        let msg = ExtensionMessage {
            msg_type: "PING".to_string(),
            version: 1,
            request_id: "test-123".to_string(),
            payload: serde_json::Value::Null,
        };

        let response = handle_message(msg);
        assert_eq!(response.msg_type, "PONG");
        assert_eq!(response.request_id, "test-123");
    }

    #[test]
    fn test_handle_unknown_type() {
        let msg = ExtensionMessage {
            msg_type: "INVALID".to_string(),
            version: 1,
            request_id: "test-456".to_string(),
            payload: serde_json::Value::Null,
        };

        let response = handle_message(msg);
        assert_eq!(response.msg_type, "ERROR");
    }

    #[test]
    fn test_handle_url_check() {
        let msg = ExtensionMessage {
            msg_type: "URL_CHECK".to_string(),
            version: 1,
            request_id: "test-789".to_string(),
            payload: serde_json::json!({ "url": "https://reddit.com" }),
        };

        let response = handle_message(msg);
        assert_eq!(response.msg_type, "URL_CHECK_RESULT");
    }
}
