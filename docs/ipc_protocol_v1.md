# FocusMe IPC Protocol Specification v1.0

> **Task:** T-006  
> **Author:** FocusMe Co-Pilot  
> **Status:** Draft  
> **Build Plan Ref:** Section 2.3 — IPC & Data Flow

---

## 1. Transport

| Platform | Transport | Path/Name |
|----------|-----------|-----------|
| Windows  | Named Pipe | `\\.\pipe\focusme_daemon` |
| macOS    | Unix Domain Socket | `/var/run/focusme.sock` |
| Linux    | Unix Domain Socket | `/var/run/focusme.sock` |

### Permissions
- **Owner:** Daemon process (root/SYSTEM)
- **Access:** Daemon user + logged-in user only
- macOS/Linux: Socket permissions `0660`, group = `focusme`
- Windows: Named Pipe DACL allows SYSTEM + interactive user SID

---

## 2. Framing Protocol

All messages use **length-prefixed framing**:

```
┌──────────────┬───────────────────┐
│ Length (4B)   │ Payload (N bytes) │
│ Little-endian│ MessagePack/JSON  │
│ uint32       │                   │
└──────────────┴───────────────────┘
```

- **Length field:** 4 bytes, little-endian unsigned 32-bit integer
- **Max message size:** 1 MB (1,048,576 bytes)
- **Encoding:** MessagePack (default) or JSON (when `debug_mode: true` in daemon config)

---

## 3. Message Envelope

Every message follows this envelope structure:

```json
{
  "version": 1,
  "msg_id": "<UUIDv4>",
  "msg_type": "<string>",
  "timestamp": "<ISO-8601 UTC>",
  "payload": { ... }
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | uint8 | Yes | Protocol version (currently `1`) |
| `msg_id` | string (UUID) | Yes | Unique message identifier for correlation |
| `msg_type` | string | Yes | Message type identifier (see Section 4) |
| `timestamp` | string (ISO-8601) | Yes | UTC timestamp of message creation |
| `payload` | object | Yes | Type-specific payload data |

### Response Envelope

```json
{
  "version": 1,
  "msg_id": "<UUIDv4>",
  "msg_type": "<response_type>",
  "timestamp": "<ISO-8601 UTC>",
  "in_reply_to": "<original msg_id>",
  "status": "ok" | "error",
  "error_code": "<string, if status=error>",
  "error_message": "<string, if status=error>",
  "payload": { ... }
}
```

---

## 4. Message Types

### 4.1 Health & Connection

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `PING` | Client → Daemon | Heartbeat check |
| `PONG` | Daemon → Client | Heartbeat response |
| `CONNECT` | Client → Daemon | Initial handshake with client metadata |
| `CONNECT_ACK` | Daemon → Client | Connection accepted, daemon version info |

#### CONNECT Payload
```json
{
  "client_type": "ui_shell" | "browser_extension" | "native_messaging_host",
  "client_version": "1.0.0",
  "platform": "windows" | "macos" | "linux"
}
```

#### CONNECT_ACK Payload
```json
{
  "daemon_version": "1.0.0",
  "protocol_version": 1,
  "forced_mode_active": false,
  "active_plan_count": 3
}
```

### 4.2 Plan Management (UI Shell → Daemon)

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `PLAN_LIST` | Client → Daemon | Request list of all plans |
| `PLAN_LIST_RESPONSE` | Daemon → Client | Array of plan summaries |
| `PLAN_GET` | Client → Daemon | Get full plan by ID |
| `PLAN_GET_RESPONSE` | Daemon → Client | Full plan object |
| `PLAN_CREATE` | Client → Daemon | Create a new plan |
| `PLAN_UPDATE` | Client → Daemon | Update an existing plan |
| `PLAN_DELETE` | Client → Daemon | Delete a plan |
| `PLAN_ACTIVATE` | Client → Daemon | Manually activate a plan |
| `PLAN_DEACTIVATE` | Client → Daemon | Manually deactivate a plan |
| `PLAN_MUTATION_RESPONSE` | Daemon → Client | Response to create/update/delete/activate/deactivate |

#### PLAN_LIST_RESPONSE Payload
```json
{
  "plans": [
    {
      "plan_id": "<UUID>",
      "name": "Deep Work",
      "enabled": true,
      "forced_mode": true,
      "active": true,
      "next_activation": "2025-01-15T09:00:00Z",
      "rule_count": 12
    }
  ]
}
```

#### PLAN_CREATE Payload
```json
{
  "plan": { /* Full policy JSON per policy_schema_v1.json */ }
}
```

#### PLAN_MUTATION_RESPONSE Payload
```json
{
  "plan_id": "<UUID>",
  "action": "created" | "updated" | "deleted" | "activated" | "deactivated",
  "success": true
}
```

### 4.3 URL/App Check (Browser Extension / NMH → Daemon)

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `URL_CHECK` | Client → Daemon | Check if a URL should be blocked |
| `URL_CHECK_RESPONSE` | Daemon → Client | ALLOW / BLOCK / QUOTA_EXCEEDED |
| `APP_CHECK` | Client → Daemon | Check if a process/app should be blocked |
| `APP_CHECK_RESPONSE` | Daemon → Client | ALLOW / BLOCK |

#### URL_CHECK Payload
```json
{
  "url": "https://reddit.com/r/programming",
  "domain": "reddit.com",
  "tab_id": 42
}
```

#### URL_CHECK_RESPONSE Payload
```json
{
  "decision": "ALLOW" | "BLOCK" | "QUOTA_EXCEEDED",
  "matched_rule_id": "<UUID | null>",
  "matched_plan_id": "<UUID | null>",
  "quota_remaining_s": 600,
  "block_page_url": "chrome-extension://.../block.html"
}
```

#### APP_CHECK Payload
```json
{
  "process_name": "Spotify.exe",
  "process_path": "C:\\Program Files\\Spotify\\Spotify.exe",
  "pid": 12345
}
```

### 4.4 Status & Events (Daemon → Client)

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `STATUS_REQUEST` | Client → Daemon | Request current daemon status |
| `STATUS_RESPONSE` | Daemon → Client | Full daemon status |
| `EVENT` | Daemon → Client | Real-time event notification (push) |

#### STATUS_RESPONSE Payload
```json
{
  "daemon_version": "1.0.0",
  "uptime_s": 86400,
  "active_plans": ["<UUID>", "<UUID>"],
  "forced_mode_active": true,
  "forced_mode_remaining_s": 3600,
  "blocked_app_count_today": 42,
  "blocked_url_count_today": 128,
  "quota_usage": [
    {
      "target": "youtube.com",
      "used_s": 1200,
      "limit_s": 1800
    }
  ]
}
```

#### EVENT Payload
```json
{
  "event_type": "PLAN_STARTED" | "PLAN_STOPPED" | "APP_BLOCKED" | "URL_BLOCKED" | "QUOTA_REACHED" | "FORCED_MODE_STARTED" | "FORCED_MODE_ENDED" | "PLAN_UPDATED",
  "plan_id": "<UUID>",
  "subject_hash": "<SHA-256 of blocked item>",
  "timestamp": "<ISO-8601 UTC>",
  "details": { /* event-specific data */ }
}
```

### 4.5 Plan Protection (Unlock Challenge)

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `UNLOCK_REQUEST` | Client → Daemon | Request to unlock a protected plan |
| `UNLOCK_CHALLENGE` | Daemon → Client | Challenge (random chars to type) |
| `UNLOCK_RESPONSE` | Client → Daemon | Password/challenge answer |
| `UNLOCK_RESULT` | Daemon → Client | Success/failure of unlock attempt |

#### UNLOCK_REQUEST Payload
```json
{
  "plan_id": "<UUID>",
  "action": "deactivate" | "delete" | "modify"
}
```

#### UNLOCK_CHALLENGE Payload
```json
{
  "challenge_text": "Type the following: x7Km9pQ2",
  "challenge_id": "<UUID>",
  "expires_at": "<ISO-8601 UTC>"
}
```

#### UNLOCK_RESPONSE Payload
```json
{
  "challenge_id": "<UUID>",
  "password": "<user-entered password>",
  "challenge_answer": "x7Km9pQ2"
}
```

### 4.6 Stats & Quota (UI Shell → Daemon)

| msg_type | Direction | Description |
|----------|-----------|-------------|
| `STATS_REQUEST` | Client → Daemon | Request usage statistics |
| `STATS_RESPONSE` | Daemon → Client | Usage data for display |
| `QUOTA_STATUS` | Client → Daemon | Request current quota usage |
| `QUOTA_STATUS_RESPONSE` | Daemon → Client | Quota usage per target |

#### STATS_REQUEST Payload
```json
{
  "date_from": "2025-01-01",
  "date_to": "2025-01-31",
  "granularity": "daily" | "hourly",
  "categories": ["apps", "domains"]
}
```

---

## 5. Error Codes

| Code | Description |
|------|-------------|
| `UNKNOWN_MSG_TYPE` | Unrecognized message type |
| `INVALID_PAYLOAD` | Payload failed schema validation |
| `PLAN_NOT_FOUND` | Referenced plan_id does not exist |
| `PLAN_LOCKED` | Plan is in Forced Mode and cannot be modified |
| `UNLOCK_FAILED` | Incorrect password or challenge answer |
| `UNLOCK_EXPIRED` | Challenge has expired |
| `RATE_LIMITED` | Too many requests (brute-force protection) |
| `INTERNAL_ERROR` | Unexpected daemon error |
| `VERSION_MISMATCH` | Client protocol version not supported |

---

## 6. Rate Limiting

- **Unlock attempts:** Max 5 per minute per plan. After 5 failures, lock out for 5 minutes.
- **URL_CHECK:** No rate limit (must respond within 20ms per performance requirement).
- **PLAN_CREATE/UPDATE/DELETE:** Max 30 per minute.

---

## 7. Versioning Strategy

- Protocol version is included in every message envelope (`version` field).
- Backward-compatible changes increment minor version.
- Breaking changes increment major version.
- Daemon MUST reject messages with unsupported major version.
- Daemon SHOULD handle messages with newer minor version by ignoring unknown fields.
