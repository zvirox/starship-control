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
 * Hardware compatibility gauge tier (DESIGN.md §6.2).
 */
export type HardwareFit = "fits_in_vram" | "partial_offload" | "cpu_only";

/**
 * Hugging Face GGUF model metadata.
 */
export interface HfModelMetadata {
  repo_id: string;
  filename: string;
  file_size_bytes: number;
  quantization: string;
  vram_required_mb: number;
  compatibility: HardwareFit;
  download_url: string;
}

/**
 * Search and download payloads.
 */
export interface SearchModelsPayload {
  query: string;
  limit?: number;
}

export interface DownloadModelPayload {
  repoId: string;
  filename: string;
  expectedSha256?: string;
}

export interface DownloadModelResponse {
  downloadJobId: string;
}

export interface DownloadProgress {
  downloadJobId: string;
  downloadedBytes: number;
  totalBytes: number;
  speedBytesPerSec: number;
  etaSeconds: number;
  progressFraction: number;
}

export interface DownloadCompleteEvent {
  downloadJobId: string;
  modelId: string;
  filePath: string;
  sha256: string;
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
 * Mission Message Record (DATABASE.md §3.2).
 */
export interface MissionMessageRecord {
  id: number;
  mission_id: number;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  thought_process: string | null;
  tool_calls_json: string | null;
  tool_call_id: string | null;
  tokens_used: number | null;
  latency_ms: number | null;
  created_at: string;
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
 * Mission creation payload and response.
 */
export interface CreateMissionPayload {
  title: string;
  userGoal: string;
  modelId: string;
}

export interface CreateMissionResponse {
  missionId: number;
  uuid: string;
  createdAt: string;
}

/**
 * Prompt submission payload and response.
 */
export interface SendPromptPayload {
  missionId: number;
  prompt: string;
}

export interface SendPromptResponse {
  messageId: number;
  status: string;
}

/**
 * Real-time push events (API.md §3).
 */
export interface TokenStreamEvent {
  missionId: number;
  chunk: string;
}

export interface ThoughtStreamEvent {
  missionId: number;
  thoughtChunk: string;
}

export interface StreamFinishedEvent {
  missionId: number;
  messageId: number;
  content: string;
  thought?: string | null;
}

/**
 * Provider Config Payload.
 */
export interface ProviderConfigPayload {
  providerId: string;
  displayName: string;
  baseUrl: string;
  apiKey?: string;
  isEnabled: boolean;
}

/**
 * App connection mode (API.md §2.4).
 */
export type AppConnectionMode = "live_gui" | "headless_cli" | "offline";

/**
 * App Topology Entry (API.md §2.4).
 */
export interface AppTopologyEntry {
  appId: string;
  displayName: string;
  version: string;
  mode: AppConnectionMode;
  socketPath: string | null;
  binaryPath: string;
  latencyMs: number | null;
  capabilities: Record<string, boolean>;
}

export interface ListAppsResponse {
  apps: AppTopologyEntry[];
}

export interface LaunchAppPayload {
  appId: string;
}

export interface LaunchAppResponse {
  spawned: boolean;
  pid: number | null;
}

export interface TopologyAppStateEvent {
  appId: string;
  mode: AppConnectionMode;
  socketPath?: string | null;
}

