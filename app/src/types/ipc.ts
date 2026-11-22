/**
 * Standard error families defined in API.md §1.1.
 */
export type ErrorKind =
  | "infer"
  | "mcp"
  | "safety"
  | "db"
  | "app"
  | "settings"
  | "ipc";

/**
 * Standard error envelope strictly adhering to API.md §1.
 */
export interface CoreError {
  kind: ErrorKind;
  code: string;
  retryable: boolean;
  userMessage: string;
  details?: Record<string, unknown> | null;
}

/**
 * Model Record from SQLite models table (DATABASE.md §3.6).
 */
export interface ModelRecord {
  id: string;
  name: string;
  provider_type:
    | "local_gguf"
    | "cloud_openai"
    | "cloud_anthropic"
    | "cloud_gemini"
    | "cloud_openrouter";
  hf_repo_id: string | null;
  file_path: string | null;
  file_size_bytes: number | null;
  quantization: string | null;
  sha256_hash: string | null;
  download_status: "ready" | "downloading" | "failed" | "remote_only";
  context_window: number;
  gpu_layers: number;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * Mission Record from SQLite missions table (DATABASE.md §3.1).
 */
export interface MissionRecord {
  id: number;
  uuid: string;
  title: string;
  user_goal: string;
  active_model_id: string;
  provider_type: string;
  status: "active" | "completed" | "failed" | "paused" | "aborted";
  total_input_tokens: number;
  total_output_tokens: number;
  cost_usd: number;
  created_at: string;
  updated_at: string;
  archived_at: string | null;
}

/**
 * Initial handshake response from ipc_hello.
 */
export interface HelloResponse {
  name: string;
  version: string;
  protocol: string;
  status: string;
  database_ready: boolean;
  available_models: ModelRecord[];
}

/**
 * HUD resize payload.
 */
export interface HudResizePayload {
  expanded: boolean;
}
