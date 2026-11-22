-- 0001_initial.sql: Starship Control initial database schema
-- Defined in DATABASE.md

-- 1. Models
CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL CHECK(provider_type IN ('local_gguf', 'cloud_openai', 'cloud_anthropic', 'cloud_gemini', 'cloud_openrouter')),
    hf_repo_id TEXT NULL,
    file_path TEXT NULL,
    file_size_bytes INTEGER NULL,
    quantization TEXT NULL,
    sha256_hash TEXT NULL,
    download_status TEXT NOT NULL DEFAULT 'ready' CHECK(download_status IN ('ready', 'downloading', 'failed', 'remote_only')),
    context_window INTEGER NOT NULL DEFAULT 8192,
    gpu_layers INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 2. Provider Configs
CREATE TABLE IF NOT EXISTS provider_configs (
    provider_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key_encrypted TEXT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK(is_enabled IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 3. Missions
CREATE TABLE IF NOT EXISTS missions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    user_goal TEXT NOT NULL,
    active_model_id TEXT NOT NULL REFERENCES models(id),
    provider_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'completed', 'failed', 'paused', 'aborted')),
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    archived_at TEXT NULL
);

-- 4. Mission Messages
CREATE TABLE IF NOT EXISTS mission_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id INTEGER NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    thought_process TEXT NULL,
    tool_calls_json TEXT NULL,
    tool_call_id TEXT NULL,
    tokens_used INTEGER NULL,
    latency_ms INTEGER NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 5. Plans
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id INTEGER NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    version INTEGER NOT NULL DEFAULT 1,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    raw_tasksheet_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft', 'pending_approval', 'executing', 'completed', 'rejected', 'failed')),
    approved_by TEXT NULL,
    approved_at TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 6. Plan Steps
CREATE TABLE IF NOT EXISTS plan_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    step_index INTEGER NOT NULL,
    app_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    risk_tier INTEGER NOT NULL CHECK(risk_tier IN (1, 2, 3)),
    approval_status TEXT NOT NULL DEFAULT 'pending' CHECK(approval_status IN ('auto', 'pending', 'approved', 'rejected')),
    execution_status TEXT NOT NULL DEFAULT 'pending' CHECK(execution_status IN ('pending', 'running', 'success', 'failed', 'skipped')),
    diff_summary TEXT NULL,
    result_json TEXT NULL,
    error_message TEXT NULL,
    started_at TEXT NULL,
    finished_at TEXT NULL
);

-- 7. App Registry
CREATE TABLE IF NOT EXISTS app_registry (
    app_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    version TEXT NOT NULL,
    binary_path TEXT NOT NULL,
    manifest_path TEXT NOT NULL,
    socket_path TEXT NULL,
    capabilities_json TEXT NOT NULL,
    is_active_gui INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- 8. Audit Log
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mission_id INTEGER NULL REFERENCES missions(id) ON DELETE SET NULL,
    app_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    risk_tier INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL,
    signature_token TEXT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Performance Indexes
CREATE INDEX IF NOT EXISTS idx_mission_messages_order 
ON mission_messages(mission_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_plan_steps_order 
ON plan_steps(plan_id, step_index ASC);

CREATE INDEX IF NOT EXISTS idx_missions_status 
ON missions(status, updated_at DESC) WHERE archived_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_audit_log_app_date 
ON audit_log(app_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_models_provider 
ON models(provider_type, download_status);

-- Seed Default Provider Configurations
INSERT OR IGNORE INTO provider_configs (provider_id, display_name, base_url, is_enabled)
VALUES 
    ('openai', 'OpenAI', 'https://api.openai.com/v1', 1),
    ('anthropic', 'Anthropic Claude', 'https://api.anthropic.com/v1', 1),
    ('gemini', 'Google Gemini', 'https://generativelanguage.googleapis.com/v1beta', 1),
    ('openrouter', 'OpenRouter', 'https://openrouter.ai/api/v1', 1);

-- Seed Default Model
INSERT OR IGNORE INTO models (id, name, provider_type, is_default, context_window, download_status)
VALUES 
    ('qwen2.5-coder-7b-q4', 'Qwen 2.5 Coder 7B (Q4_K_M)', 'local_gguf', 1, 8192, 'ready');
