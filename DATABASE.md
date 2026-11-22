# DATABASE.md — Starship Control Data Architecture

**Version:** 1.0 · **Date:** 2026-11-17  
**Authority chain:** `PRD.md` (scope, safety requirements) → `ARCHITECTURE.md` (§7 data principles) → **this file (schema & persistence law)**  
**Rule:** Defines all SQLite schemas, migrations, table definitions, foreign keys, constraints, and query patterns. No product code is written as part of this document.

---

## 1. Engine & Storage Decisions

| Decision | Specification | Rationale |
|---|---|---|
| **Database Engine** | **SQLite 3 via `rusqlite` (bundled)** | Zero-admin, embedded ACID engine, zero external daemon dependencies. |
| **File Location** | `$XDG_DATA_HOME/starship-control/library.db` | Conforms strictly to XDG Base Directory specification (`~/.local/share/starship-control/`). |
| **Permissions** | `0600` (User read/write only) | Security requirement: protects local mission history and provider configs from unauthorized local users. |
| **Journal Mode** | `PRAGMA journal_mode = WAL;` | Write-Ahead Logging allows non-blocking concurrent reads during streaming writes. |
| **Integrity Checks** | `PRAGMA foreign_keys = ON;` | Strict referential integrity enforcement on all cascading relations. |
| **Busy Timeout** | `PRAGMA busy_timeout = 5000;` | Prevents transient write-lock collisions between worker threads. |
| **Migrations** | `PRAGMA user_version` runner | Embedded versioned SQL scripts (`sql/0001_initial.sql`, etc.) executed atomically within a transaction. |

---

## 2. Entity-Relationship Topology

```
┌─────────────────┐       ┌──────────────────────┐       ┌─────────────────┐
│     models      │       │       missions       │       │provider_configs │
├─────────────────┤       ├──────────────────────┤       ├─────────────────┤
│ id (PK)         │       │ id (PK)              │       │ provider_id(PK) │
│ provider_type   │       │ uuid                 │       │ base_url        │
│ hf_repo_id      │       │ active_model_id (FK) ┼──────►│ api_key_enc     │
│ file_path       │       │ status               │       └─────────────────┘
└─────────────────┘       └──────────┬───────────┘
                                     │ 1
                                     │
                         ┌───────────┴───────────┐
                       N │                     N │
                         ▼                       ▼
              ┌─────────────────────┐ ┌─────────────────────┐
              │  mission_messages   │ │        plans        │
              ├─────────────────────┤ ├─────────────────────┤
              │ id (PK)             │ │ id (PK)             │
              │ mission_id (FK)     │ │ mission_id (FK)     │
              │ role                │ │ version             │
              │ content             │ │ raw_tasksheet_json  │
              │ thought_process     │ │ status              │
              └─────────────────────┘ └──────────┬──────────┘
                                                 │ 1
                                                 │
                                               N ▼
                                      ┌─────────────────────┐
                                      │     plan_steps      │
                                      ├─────────────────────┤
                                      │ id (PK)             │
                                      │ plan_id (FK)        │
                                      │ app_id (FK)         │
                                      │ tool_name           │
                                      │ risk_tier (1, 2, 3) │
                                      │ approval_status     │
                                      │ execution_status    │
                                      └─────────────────────┘

┌─────────────────┐       ┌──────────────────────┐
│  app_registry   │       │      audit_log       │
├─────────────────┤       ├──────────────────────┤
│ app_id (PK)     │       │ id (PK)              │
│ binary_path     │       │ mission_id (Nullable)│
│ socket_path     │       │ app_id               │
│ capabilities    │       │ action_type          │
│ is_active_gui   │       │ risk_tier            │
└─────────────────┘       │ signature_token      │
                          └──────────────────────┘
```

---

## 3. Schema & Table Definitions

### 3.1 `missions` — Strategic Goals & Agent Sessions
Represents an ongoing or completed user mission.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Internal row ID. |
| `uuid` | `TEXT` | `NOT NULL UNIQUE` | UUIDv4 string for external references. |
| `title` | `TEXT` | `NOT NULL` | Human-readable title (e.g. "Tutorial Screencast Assembly"). |
| `user_goal` | `TEXT` | `NOT NULL` | Original natural-language prompt provided by the user. |
| `active_model_id`| `TEXT` | `NOT NULL` | Model ID active during this mission (references `models.id`). |
| `provider_type` | `TEXT` | `NOT NULL` | `'local_gguf'` / `'cloud_openai'` / `'cloud_anthropic'` / `'cloud_gemini'` / `'cloud_openrouter'`. |
| `status` | `TEXT` | `NOT NULL DEFAULT 'active'` | Check: `status IN ('active', 'completed', 'failed', 'paused', 'aborted')`. |
| `total_input_tokens`| `INTEGER` | `NOT NULL DEFAULT 0` | Cumulative prompt tokens consumed. |
| `total_output_tokens`| `INTEGER`| `NOT NULL DEFAULT 0` | Cumulative completion tokens generated. |
| `cost_usd` | `REAL` | `NOT NULL DEFAULT 0.0` | Estimated USD cost (0.0 for local models). |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | ISO-8601 UTC timestamp. |
| `updated_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Last interaction timestamp. |
| `archived_at` | `TEXT` | `NULL` | Non-null indicates mission is archived from active sidebar. |

---

### 3.2 `mission_messages` — Chronological Dialogue & Thought Logs
Stores conversational history, token generation, tool call invocations, and reasoning traces.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Message ID. |
| `mission_id` | `INTEGER` | `NOT NULL REFERENCES missions(id) ON DELETE CASCADE` | Parent mission ID. |
| `role` | `TEXT` | `NOT NULL` | Check: `role IN ('user', 'assistant', 'system', 'tool')`. |
| `content` | `TEXT` | `NOT NULL` | Visible textual response or prompt. |
| `thought_process` | `TEXT` | `NULL` | Model chain-of-thought or internal reasoning trace. |
| `tool_calls_json` | `TEXT` | `NULL` | JSON array of proposed tool calls (`[{id, tool, args}]`). |
| `tool_call_id` | `TEXT` | `NULL` | Matching tool invocation ID when `role = 'tool'`. |
| `tokens_used` | `INTEGER` | `NULL` | Tokens consumed in this specific turn. |
| `latency_ms` | `INTEGER` | `NULL` | Execution latency in milliseconds. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | ISO-8601 UTC timestamp. |

---

### 3.3 `plans` — Task Sheets (`tasksheet/1`)
Versioned execution plans decomposed from user goals.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Plan ID. |
| `mission_id` | `INTEGER` | `NOT NULL REFERENCES missions(id) ON DELETE CASCADE` | Parent mission ID. |
| `version` | `INTEGER` | `NOT NULL DEFAULT 1` | Increments when an agent revises an unexecuted plan. |
| `title` | `TEXT` | `NOT NULL` | Short title of the planned workflow. |
| `summary` | `TEXT` | `NOT NULL` | Plain-English description of what the plan achieves. |
| `raw_tasksheet_json`| `TEXT` | `NOT NULL` | Canonical JSON task sheet adhering to `tasksheet/1`. |
| `status` | `TEXT` | `NOT NULL DEFAULT 'draft'` | Check: `status IN ('draft', 'pending_approval', 'executing', 'completed', 'rejected', 'failed')`. |
| `approved_by` | `TEXT` | `NULL` | `'user'` (manual HUD/Cockpit signature) or `'auto_safe'`. |
| `approved_at` | `TEXT` | `NULL` | Timestamp of user consent. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Plan creation timestamp. |
| `updated_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Status update timestamp. |

---

### 3.4 `plan_steps` — Individual Action Units
Discrete tool execution steps within a task sheet.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Step ID. |
| `plan_id` | `INTEGER` | `NOT NULL REFERENCES plans(id) ON DELETE CASCADE` | Parent plan ID. |
| `step_index` | `INTEGER` | `NOT NULL` | Execution order index (0, 1, 2...). |
| `app_id` | `TEXT` | `NOT NULL` | Target application (`'eep'`, `'ssp'`, `'alp'`, `'nwp'`, `'atp'`). |
| `tool_name` | `TEXT` | `NOT NULL` | MCP tool identifier (e.g. `timeline.cut_interval`). |
| `arguments_json` | `TEXT` | `NOT NULL` | Serialized tool parameters. |
| `risk_tier` | `INTEGER` | `NOT NULL` | Check: `risk_tier IN (1, 2, 3)` (1=Safe, 2=Mutating, 3=Destructive). |
| `approval_status`| `TEXT` | `NOT NULL DEFAULT 'pending'` | Check: `approval_status IN ('auto', 'pending', 'approved', 'rejected')`. |
| `execution_status`| `TEXT`| `NOT NULL DEFAULT 'pending'` | Check: `execution_status IN ('pending', 'running', 'success', 'failed', 'skipped')`. |
| `diff_summary` | `TEXT` | `NULL` | Human-readable explanation of impact (e.g. "Cuts 18 clips"). |
| `result_json` | `TEXT` | `NULL` | Result returned by sub-app MCP server. |
| `error_message` | `TEXT` | `NULL` | Error details if execution failed. |
| `started_at` | `TEXT` | `NULL` | Timestamp when tool execution commenced. |
| `finished_at` | `TEXT` | `NULL` | Timestamp when tool execution completed. |

---

### 3.5 `app_registry` — Discovered Applications & Plugin State
Maintains live discovery and socket connections for all ecosystem applications.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `app_id` | `TEXT` | `PRIMARY KEY` | Canonical ID (`'eep'`, `'ssp'`, `'alp'`, `'nwp'`, `'atp'`). |
| `display_name` | `TEXT` | `NOT NULL` | Human-readable name ("Easy Edits Pro"). |
| `version` | `TEXT` | `NOT NULL` | Application version reported in manifest. |
| `binary_path` | `TEXT` | `NOT NULL` | Absolute path to CLI executable. |
| `manifest_path`| `TEXT` | `NOT NULL` | Path to `$XDG_DATA_HOME/starship/plugins/<id>.json`. |
| `socket_path` | `TEXT` | `NULL` | Active bridge socket under `$XDG_RUNTIME_DIR`. |
| `capabilities_json`| `TEXT` | `NOT NULL` | Advertised feature set and tool profiles. |
| `is_active_gui`| `INTEGER` | `NOT NULL DEFAULT 0` | 1 if live GUI socket verified via `SO_PEERCRED`. |
| `last_seen_at` | `TEXT` | `NOT NULL` | Timestamp of last heartbeat or socket ping. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Discovery registration timestamp. |

---

### 3.6 `models` — Local GGUF Weights & Cloud Model Registry
Tracks both local weights downloaded from Hugging Face and cloud provider endpoints.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `TEXT` | `PRIMARY KEY` | Unique ID (e.g. `"qwen2.5-coder-7b-q4"`, `"claude-3-7-sonnet"`). |
| `name` | `TEXT` | `NOT NULL` | Display name ("Qwen 2.5 Coder 7B (Q4_K_M)"). |
| `provider_type` | `TEXT` | `NOT NULL` | Check: `provider_type IN ('local_gguf', 'cloud_openai', 'cloud_anthropic', 'cloud_gemini', 'cloud_openrouter')`. |
| `hf_repo_id` | `TEXT` | `NULL` | Hugging Face repo ID (e.g. `"Qwen/Qwen2.5-Coder-7B-Instruct-GGUF"`). |
| `file_path` | `TEXT` | `NULL` | Path on disk if local (`$XDG_DATA_HOME/starship-control/models/...`). |
| `file_size_bytes`| `INTEGER` | `NULL` | Weight file size. |
| `quantization` | `TEXT` | `NULL` | Quant tier (`'Q4_K_M'`, `'Q8_0'`, `'f16'`). |
| `sha256_hash` | `TEXT` | `NULL` | Verified checksum of downloaded weights. |
| `download_status`| `TEXT` | `NOT NULL DEFAULT 'ready'` | Check: `download_status IN ('ready', 'downloading', 'failed', 'remote_only')`. |
| `context_window`| `INTEGER` | `NOT NULL DEFAULT 8192` | Supported token window. |
| `gpu_layers` | `INTEGER` | `NOT NULL DEFAULT 0` | Number of layers offloaded to Vulkan/ROCm. |
| `is_default` | `INTEGER` | `NOT NULL DEFAULT 0` | 1 if this is the active default model. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | ISO-8601 UTC timestamp. |
| `updated_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Last modified timestamp. |

---

### 3.7 `provider_configs` — Cloud Endpoint Configurations
Secure storage for external API endpoints.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `provider_id` | `TEXT` | `PRIMARY KEY` | Provider ID (`'openai'`, `'anthropic'`, `'gemini'`, `'openrouter'`). |
| `display_name` | `TEXT` | `NOT NULL` | Display label ("Anthropic Claude"). |
| `base_url` | `TEXT` | `NOT NULL` | REST endpoint base URL. |
| `api_key_encrypted`| `TEXT` | `NULL` | AES-GCM-256 encrypted or Secret Service URI. |
| `is_enabled` | `INTEGER` | `NOT NULL DEFAULT 1` | Check: `is_enabled IN (0, 1)`. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Creation timestamp. |
| `updated_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Modification timestamp. |

---

### 3.8 `audit_log` — Immutable Security & Action Audit Trail
An append-only log recording every tool invocation, security decision, and user signature.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Event sequence number. |
| `mission_id` | `INTEGER` | `NULL` | Associated mission ID if applicable. |
| `app_id` | `TEXT` | `NOT NULL` | Sub-app invoked. |
| `action_type` | `TEXT` | `NOT NULL` | `'tool_call'`, `'approval_granted'`, `'approval_denied'`, `'app_spawned'`. |
| `risk_tier` | `INTEGER` | `NOT NULL` | Safety tier (1, 2, 3). |
| `payload_json` | `TEXT` | `NOT NULL` | Full serialized parameters or diff summary. |
| `status` | `TEXT` | `NOT NULL` | `'success'`, `'denied'`, `'failed'`. |
| `signature_token`| `TEXT` | `NULL` | Cryptographic or ephemeral UI token proving human consent. |
| `created_at` | `TEXT` | `NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))` | Immutable UTC timestamp. |

---

### 3.9 `health_rollup_1m` — 1-Minute System Health Aggregates
High-resolution 1-minute rollup records for live Cockpit dashboards and sparklines. Retained for 7 days.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `ts` | `INTEGER` | `PRIMARY KEY` | Window start Unix epoch timestamp (seconds). |
| `cpu_avg` | `REAL` | `NOT NULL` | Average CPU utilization % across the minute. |
| `cpu_max` | `REAL` | `NOT NULL` | Peak CPU utilization % observed in the minute. |
| `mem_avail_pct_min` | `REAL` | `NOT NULL` | Lowest `MemAvailable` percentage observed. |
| `swap_in_max` | `REAL` | `NOT NULL` | Peak swap-in throughput (`pswpin` bytes/sec). |
| `disk_free_pct_min`| `REAL` | `NOT NULL` | Lowest primary partition free space %. |
| `temp_max` | `REAL` | `NOT NULL` | Maximum thermal sensor reading (Celsius). |
| `gpu_util_max` | `REAL` | `NULL` | Peak GPU compute utilization %. |
| `vram_pct_max` | `REAL` | `NULL` | Peak VRAM used percentage. |
| `psi_cpu_max` | `REAL` | `NULL` | Peak Linux PSI CPU stall 10s average. |
| `psi_mem_max` | `REAL` | `NULL` | Peak Linux PSI Memory stall 10s average. |
| `psi_io_max` | `REAL` | `NULL` | Peak Linux PSI I/O stall 10s average. |
| `state` | `TEXT` | `NOT NULL` | Worst governor state (`'Normal'`, `'Elevated'`, `'Constrained'`, `'Critical'`). |

---

### 3.10 `health_rollup_1h` — 1-Hour Long-Term System Health Aggregates
Long-term historical 1-hour rollup records. Retained for 90 days.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `ts` | `INTEGER` | `PRIMARY KEY` | Window start Unix epoch timestamp (seconds). |
| `cpu_avg` | `REAL` | `NOT NULL` | Average CPU utilization % across the hour. |
| `cpu_max` | `REAL` | `NOT NULL` | Peak CPU utilization % observed in the hour. |
| `mem_avail_pct_min` | `REAL` | `NOT NULL` | Lowest `MemAvailable` percentage observed. |
| `swap_in_max` | `REAL` | `NOT NULL` | Peak swap-in throughput (`pswpin` bytes/sec). |
| `disk_free_pct_min`| `REAL` | `NOT NULL` | Lowest primary partition free space %. |
| `temp_max` | `REAL` | `NOT NULL` | Maximum thermal sensor reading (Celsius). |
| `gpu_util_max` | `REAL` | `NULL` | Peak GPU compute utilization %. |
| `vram_pct_max` | `REAL` | `NULL` | Peak VRAM used percentage. |
| `psi_cpu_max` | `REAL` | `NULL` | Peak Linux PSI CPU stall 10s average. |
| `psi_mem_max` | `REAL` | `NULL` | Peak Linux PSI Memory stall 10s average. |
| `psi_io_max` | `REAL` | `NULL` | Peak Linux PSI I/O stall 10s average. |
| `state` | `TEXT` | `NOT NULL` | Worst governor state across the hour. |

---

### 3.11 `health_alerts` — Governor & System Overload Event Trail
Records every trip into Elevated, Constrained, or Critical health states and the resulting governor action.

| Column | SQLite Type | Constraints | Description |
|---|---|---|---|
| `id` | `INTEGER` | `PRIMARY KEY AUTOINCREMENT` | Alert event identifier. |
| `ts` | `INTEGER` | `NOT NULL` | Alert trigger Unix epoch timestamp (seconds). |
| `signal` | `TEXT` | `NOT NULL` | Tripped signal (`'cpu'`, `'memory'`, `'thermal'`, `'disk'`, `'psi'`). |
| `level` | `TEXT` | `NOT NULL` | Severity level (`'Elevated'`, `'Constrained'`, `'Critical'`). |
| `value` | `REAL` | `NOT NULL` | Measured sensor value that triggered the alert. |
| `action` | `TEXT` | `NOT NULL` | Governor intervention (`'warn'`, `'throttle'`, `'safe_pause'`). |
| `mission_id` | `INTEGER` | `NULL REFERENCES missions(id)` | Mission impacted by governor action if any. |

---

## 4. Performance Indexes

The following indexes guarantee sub-millisecond query response times across high-volume missions:

```sql
-- Mission message retrieval ordered chronologically
CREATE INDEX idx_mission_messages_order 
ON mission_messages(mission_id, created_at ASC);

-- Plan steps retrieval by step order
CREATE INDEX idx_plan_steps_order 
ON plan_steps(plan_id, step_index ASC);

-- Rapid lookup for active missions
CREATE INDEX idx_missions_status 
ON missions(status, updated_at DESC) WHERE archived_at IS NULL;

-- Fast audit log filtering by sub-app and timestamp
CREATE INDEX idx_audit_log_app_date 
ON audit_log(app_id, created_at DESC);

-- Model query by provider type
CREATE INDEX idx_models_provider 
ON models(provider_type, download_status);
```

---

## 5. Maintenance & Pruning Policy

1. **WAL Checkpointing:** Starship Control executes `PRAGMA wal_checkpoint(PASSIVE);` during idle periods, ensuring the `.wal` sidecar file does not grow indefinitely.
2. **Audit Log Retention:** By default, `audit_log` entries are preserved for **180 days**. An automated background vacuum prunes entries older than the user-configured retention policy.
3. **Cascading Cleanups:** Deleting a mission automatically cascades and removes all child records in `mission_messages`, `plans`, and `plan_steps`, leaving zero orphaned rows.
4. **Backup Integrity:** A one-click backup mechanism in Cockpit Settings triggers `VACUUM INTO 'backup-path.db'` to generate consistent point-in-time snapshots without locking the database.
