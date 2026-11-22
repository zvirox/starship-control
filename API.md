# API.md — Starship Control IPC & Protocol Contracts

**Version:** 1.0 · **Date:** 2026-11-17  
**Authority chain:** `PRD.md` (functional requirements) → `ARCHITECTURE.md` (process boundaries, transports) → **this file (contract law)**  
**Rule:** Defines all typed request/response commands, real-time push events, error codes, and the `starship-plugin` MCP wire protocol. No product code is written as part of this document.

---

## 1. Standard Error Format

Every error returned by a command or emitted over an error event strictly matches this JSON structure:

```json
{
  "kind": "infer",
  "code": "infer.model_out_of_memory",
  "retryable": false,
  "userMessage": "Local model weights exceed available VRAM. Try choosing a smaller quantization (e.g. Q4_K_M) or offloading fewer GPU layers.",
  "details": {
    "requiredBytes": 8589934592,
    "availableBytes": 6442450944,
    "backend": "vulkan"
  }
}
```

### 1.1 Error Envelope Schema
| Field | Type | Description |
|---|---|---|
| `kind` | `string` | Error family: `'infer'`, `'mcp'`, `'safety'`, `'db'`, `'app'`, `'settings'`, `'ipc'`, `'plan'`. |
| `code` | `string` | Stable, machine-readable identifier in dot notation (`<domain>.<cause>`). |
| `retryable` | `boolean` | Indicates whether the UI should provide an immediate Retry action. |
| `userMessage` | `string` | Plain-English message formatted strictly from DESIGN.md vocabulary. Raw errors are logged to disk only. |
| `details` | `object \| null` | Optional structured metadata (bytes missing, PID, socket path, HTTP status). |

### 1.2 Canonical Error Codes
- **Inference (`infer.*`):** `infer.model_not_found`, `infer.model_out_of_memory`, `infer.download_failed`, `infer.checksum_mismatch`, `infer.cloud_auth_failed`, `infer.cloud_rate_limited`, `infer.cloud_endpoint_unreachable`, `infer.context_length_exceeded`.
- **MCP & Bridge (`mcp.*`):** `mcp.socket_unreachable`, `mcp.so_peercred_failed`, `mcp.handshake_timeout`, `mcp.tool_not_found`, `mcp.execution_failed`, `mcp.app_spawn_failed`, `mcp.lease_expired`, `mcp.manifest_untrusted`, `mcp.peer_exe_mismatch`, `mcp.checkpoint_unsupported`.
- **Safety & Permissions (`safety.*`):** `safety.approval_rejected`, `safety.permission_denied`, `safety.invalid_signature_token`, `safety.tier_violation`.
- **Database & Storage (`db.*`):** `db.read_failed`, `db.write_failed`, `db.constraint_violation`, `db.migration_failed`, `db.audit_tampered`.
- **IPC & Window (`ipc.*`):** `ipc.invalid_payload`, `ipc.bad_state`, `ipc.window_not_found`, `ipc.cycle_detected`.
- **Planner & Schema (`plan.*`):** `plan.invalid_schema`, `plan.cycle_detected`, `plan.unknown_tool`, `plan.unknown_app`, `plan.args_invalid`, `plan.duplicate_step_index`, `plan.self_dependency`, `plan.missing_dependency`, `plan.max_steps_exceeded`, `plan.max_tier3_exceeded`, `plan.invalid_risk_tier`, `plan.empty_plan`, `plan.empty_goal`, `planner.unsupported_version`.
- **Executor & Recovery (`exec.*`):** `exec.step_timeout`, `exec.stalled`, `exec.replan_exhausted`, `exec.rollback_failed`.

---

## 2. Tauri IPC Commands (`cmd://...`)

Commands invoked by the frontend via `invoke("command_name", payload)`.

### 2.1 Mission Management

#### `cmd://mission_create`
Creates a new mission session.
* **Payload:**
  ```json
  {
    "title": "Clean Podcast Episode 12",
    "userGoal": "Remove noise in ALP, trim dead time in EEP",
    "modelId": "qwen2.5-coder-7b-q4"
  }
  ```
* **Success Output:**
  ```json
  {
    "missionId": 42,
    "uuid": "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
    "createdAt": "2026-11-17T11:45:00Z"
  }
  ```
* **Errors:** `db.write_failed`, `infer.model_not_found`.

#### `cmd://mission_send_prompt`
Dispatches a new user message or instruction to an active mission.
* **Payload:**
  ```json
  {
    "missionId": 42,
    "prompt": "Proceed with trimming 14 dead air segments in EEP"
  }
  ```
* **Success Output:**
  ```json
  {
    "messageId": 105,
    "status": "thinking"
  }
  ```
* **Errors:** `ipc.bad_state`, `infer.cloud_auth_failed`, `infer.model_out_of_memory`.

#### `cmd://mission_abort`
Immediately halts execution, cancels running tools, and releases project locks.
* **Payload:** `{"missionId": 42}`
* **Success Output:** `{"aborted": true}`

---

### 2.2 Plan & Safety Approvals

#### `cmd://plan_get_active`
Fetches the current task sheet and steps for an active mission.
* **Payload:** `{"missionId": 42}`
* **Success Output:**
  ```json
  {
    "planId": 12,
    "version": 1,
    "status": "pending_approval",
    "steps": [
      {
        "stepId": 101,
        "stepIndex": 0,
        "appId": "alp",
        "toolName": "audio.cleanup",
        "riskTier": 2,
        "approvalStatus": "pending",
        "diffSummary": "Applies Voice Cleanup preset to track 'Vocal-Main'",
        "arguments": {"preset": "voice_clean", "trackId": 1}
      }
    ]
  }
  ```

#### `cmd://plan_approve_step`
User signs an approval gate on the Cockpit or HUD.
* **Payload:**
  ```json
  {
    "stepId": 101,
    "signatureToken": "usr_sig_8f3a9e21"
  }
  ```
* **Success Output:** `{"executed": true, "status": "running"}`
* **Errors:** `safety.invalid_signature_token`, `mcp.execution_failed`.

#### `cmd://plan_reject_step`
User declines a proposed mutating step.
* **Payload:**
  ```json
  {
    "stepId": 101,
    "reason": "Do not modify the vocal track yet"
  }
  ```
* **Success Output:** `{"status": "rejected"}`

#### `cmd://pipeline_screencast`
Generates and activates a multi-app screencast pipeline (SSP $\to$ ALP $\to$ EEP) with automatic silence ripple cuts from CET telemetry.
* **Payload:**
  ```json
  {
    "missionId": 42,
    "recordingPath": "/home/user/Videos/demo-01.mkv",
    "cetPath": "/home/user/Videos/demo-01.cet.jsonl",
    "cetContent": null,
    "cleanAudio": true,
    "audioPreset": "voice_isolate",
    "introScript": "Welcome to Starship Control.",
    "targetTimelineTrack": 1
  }
  ```
* **Success Output:** `PlanActiveResponse` (TaskSheet with 5 stages, pending approval gate, and diff summary).

#### `cmd://pipeline_diagnostics`
Generates and activates a cross-app system health inspection and disk cleanup pipeline (NWP $\to$ ATP) with Tier 3 destructive approval gate.
* **Payload:**
  ```json
  {
    "missionId": 42,
    "liveConnectionsCount": 14,
    "totalBandwidthKbps": 450.0,
    "diskUsedGb": 120.5,
    "diskFreeGb": 34.2,
    "cacheBloatMb": 4300.0,
    "reclaimableItems": ["~/.cache/cargo/registry", "~/.cache/npm"]
  }
  ```
* **Success Output:** `PlanActiveResponse` (TaskSheet with parallel inspections in Stage 0 and Tier 3 purge gate in Stage 1).

---

### 2.3 Model Control & Hugging Face

#### `cmd://models_list`
Lists all local weights and configured cloud models.
* **Payload:** `{}`
* **Success Output:**
  ```json
  {
    "models": [
      {
        "id": "qwen2.5-coder-7b-q4",
        "name": "Qwen 2.5 Coder 7B (Q4_K_M)",
        "providerType": "local_gguf",
        "filePath": "/home/user/.local/share/starship-control/models/qwen2.5-7b.gguf",
        "fileSizeBytes": 4680000000,
        "downloadStatus": "ready",
        "isDefault": true,
        "vramRequiredMb": 5120
      },
      {
        "id": "claude-3-7-sonnet",
        "name": "Claude 3.7 Sonnet",
        "providerType": "cloud_anthropic",
        "downloadStatus": "remote_only",
        "isDefault": false
      }
    ]
  }
  ```

#### `cmd://model_hf_download`
Starts downloading GGUF weights directly from Hugging Face with progress tracking.
* **Payload:**
  ```json
  {
    "repoId": "Qwen/Qwen2.5-Coder-7B-Instruct-GGUF",
    "filename": "qwen2.5-coder-7b-instruct-q4_k_m.gguf",
    "expectedSha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  }
  ```
* **Success Output:** `{"downloadJobId": "dl_98f12"}`
* **Errors:** `infer.download_failed`, `disk.no_space`.

#### `cmd://provider_config_set`
Saves cloud provider credentials.
* **Payload:**
  ```json
  {
    "providerId": "anthropic",
    "baseUrl": "https://api.anthropic.com/v1",
    "apiKey": "sk-ant-api03-...",
    "isEnabled": true
  }
  ```
* **Success Output:** `{"saved": true}`

---

### 2.4 App Topology & Control

#### `cmd://topology_list_apps`
Lists all discovered applications from manifests and live sockets.
* **Payload:** `{}`
* **Success Output:**
  ```json
  {
    "apps": [
      {
        "appId": "eep",
        "displayName": "Easy Edits Pro",
        "version": "0.1.2-alpha.7",
        "mode": "live_gui",
        "socketPath": "/run/user/1000/starship/bridges/eep.sock",
        "latencyMs": 0.4
      },
      {
        "appId": "atp",
        "displayName": "Agentic Tuneup Pro",
        "version": "0.1.0",
        "mode": "offline",
        "socketPath": null,
        "latencyMs": null
      }
    ]
  }
  ```

#### `cmd://topology_launch_app`
Wakes up an offline application into GUI mode.
* **Payload:** `{"appId": "eep"}`
* **Success Output:** `{"spawned": true, "pid": 48291}`

---

### 2.5 Floating HUD Management

#### `cmd://hud_toggle`
Toggles HUD window visibility.
* **Payload:** `{}`
* **Success Output:** `{"visible": true}`

#### `cmd://hud_set_pinned`
Configures always-on-top behavior.
* **Payload:** `{"pinned": true}`
* **Success Output:** `{"pinned": true}`

---

### 2.6 System Health & Overload Governor

#### `cmd://health_get_snapshot`
Obtains instantaneous system metrics, health state, and governor directive.
* **Payload:** `{}`
* **Success Output:**
  ```json
  {
    "sample": {
      "timestampMs": 1700000000000,
      "cpu": {"globalUtilPct": 42.5, "coreUtilPct": [40.0, 45.0], "loadAvg1m": 1.2, "loadAvg5m": 1.0, "loadAvg15m": 0.8, "avgFreqMhz": 3200},
      "memory": {"totalBytes": 16000000000, "usedBytes": 8000000000, "availableBytes": 8000000000, "availablePct": 50.0, "swapTotalBytes": 0, "swapUsedBytes": 0, "swapInBytesPerSec": 0.0},
      "topProcesses": []
    },
    "state": "normal",
    "governorDirective": {
      "currentHealthState": "normal",
      "maxParallelism": 2,
      "allowHeavySteps": true,
      "allowLocalInference": true,
      "allowBackgroundPrefetch": true,
      "shouldPause": false,
      "shouldResume": false,
      "shouldUnloadIdleModel": false,
      "actionMessage": "Normal system health: standard execution"
    }
  }
  ```

#### `cmd://health_get_history`
Fetches historical system health samples (15m in-memory ring buffer) or rollups (SQLite).
* **Payload:**
  ```json
  {
    "range": "15m",
    "resolution": "1s"
  }
  ```
* **Success Output:**
  ```json
  {
    "range": "15m",
    "samples": [],
    "rollups": []
  }
  ```

#### `cmd://health_get_thresholds`
Fetches current overload detector thresholds.
* **Payload:** `{}`
* **Success Output:** `OverloadThresholds` JSON object.

#### `cmd://health_set_thresholds`
Updates overload detector thresholds.
* **Payload:** `OverloadThresholds` JSON object.
* **Success Output:** Updated `OverloadThresholds` JSON object.

---

## 3. Real-Time Push Events (`evt://...`)

Events emitted from the Rust backend to the Tauri webview via `app.emit("event_name", payload)`.

### 3.1 `evt://token/stream`
Pushed when an inference engine outputs completion tokens.
```json
{
  "missionId": 42,
  "chunk": " I will now inspect the timeline in Easy Edits Pro..."
}
```

### 3.2 `evt://thought/stream`
Pushed when reasoning/thought tokens are generated.
```json
{
  "missionId": 42,
  "thoughtChunk": "Evaluating audio cleanup presets: voice_clean vs dehum..."
}
```

### 3.3 `evt://plan/approval_needed`
Emitted when a Tier 2 or Tier 3 action requires human consent. Expands the Floating HUD.
```json
{
  "missionId": 42,
  "planId": 12,
  "stepId": 101,
  "appId": "eep",
  "riskTier": 2,
  "actionTitle": "Ripple Cut 18 Dead-Time Segments",
  "diffSummary": "Deletes 01:24 of silence based on CET telemetry",
  "checkpointName": "#bk-04"
}
```

### 3.4 `evt://download/progress`
Reports Hugging Face GGUF transfer telemetry at 4 Hz.
```json
{
  "downloadJobId": "dl_98f12",
  "downloadedBytes": 2147483648,
  "totalBytes": 4680000000,
  "speedBytesPerSec": 45200000,
  "etaSeconds": 56,
  "progressFraction": 0.458
}
```

### 3.5 `evt://topology/app_state`
Emitted when an application launches, exits, or registers a socket.
```json
{
  "appId": "ssp",
  "mode": "live_gui",
  "socketPath": "/run/user/1000/starship/bridges/ssp.sock"
}
```

### 3.6 `evt://health/sample`
Pushed at 1 Hz max with live system metrics snapshot for Cockpit gauges and HUD.
```json
{
  "timestampMs": 1700000000000,
  "cpu": {"globalUtilPct": 42.5},
  "memory": {"availablePct": 48.0},
  "topProcesses": []
}
```

### 3.7 `evt://health/state_changed`
Emitted when the system health state transitions between Normal, Elevated, Constrained, and Critical.
```json
{
  "previousState": "normal",
  "currentState": "elevated",
  "directive": {
    "currentHealthState": "elevated",
    "maxParallelism": 2,
    "allowHeavySteps": true,
    "allowBackgroundPrefetch": false,
    "shouldPause": false
  }
}
```

### 3.8 `evt://health/alert`
Emitted whenever an individual sensor trips an overload threshold or recovery rule.
```json
{
  "signal": "cpu",
  "level": "elevated",
  "value": 82.5,
  "action": "warn"
}
```

---

## 4. `starship-plugin` MCP Wire Contract

The wire protocol between Starship Control and sub-applications (EEP, SSP, ALP, etc.) runs over **Unix Domain Sockets** using standard **JSON-RPC 2.0** framing.

### 4.1 Handshake (`starship/hello`)
Immediately upon socket connection, Starship Control initiates the handshake:

* **Request (Starship Control $\to$ Sub-App):**
  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "starship/hello",
    "params": {
      "starshipVersion": "1.0.0",
      "supportedMcpVersion": "2024-11-05"
    }
  }
  ```

* **Response (Sub-App $\to$ Starship Control):**
  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
      "appId": "eep",
      "name": "Easy Edits Pro",
      "version": "0.1.2-alpha.7",
      "tools": [
        {
          "name": "timeline.cut_interval",
          "description": "Performs a ripple cut between two timestamps",
          "riskTier": 2,
          "parameters": {
            "type": "object",
            "properties": {
              "startMs": {"type": "integer"},
              "endMs": {"type": "integer"}
            },
            "required": ["startMs", "endMs"]
          }
        }
      ]
    }
  }
  ```

### 4.2 Tool Invocation (`tools/call`)

* **Request (Starship Control $\to$ Sub-App):**
  ```json
  {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "timeline.cut_interval",
      "arguments": {
        "startMs": 4200,
        "endMs": 8500
      },
      "leaseTimeoutMs": 10000
    }
  }
  ```

* **Response (Sub-App $\to$ Starship Control):**
  ```json
  {
    "jsonrpc": "2.0",
    "id": 2,
    "result": {
      "status": "success",
      "editsApplied": 1,
      "newDurationMs": 284000
    }
  }
  ```

### 4.3 Task Cancellation (`tools/cancel`)
If a user hits emergency stop or aborts a mission:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/cancel",
  "params": {
    "callId": 2,
    "reason": "user_aborted"
  }
}
```
Sub-apps must immediately drop ongoing tasks and release any acquired timeline or media locks.
