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

/**
 * Aggregated MCP Tool Representation (API.md §4).
 */
export interface AggregatedTool {
  appId: string;
  appName: string;
  name: string;
  description: string;
  riskTier: 1 | 2 | 3;
  parameters: Record<string, unknown>;
}

export interface CallToolPayload {
  appId: string;
  toolName: string;
  arguments: Record<string, unknown>;
  leaseTimeoutMs?: number;
}

/**
 * Plan & Safety Step Detail (API.md §2.2).
 */
export interface PlanStepDetail {
  stepId: number;
  stepIndex: number;
  appId: string;
  toolName: string;
  riskTier: 1 | 2 | 3;
  approvalStatus: "auto" | "pending" | "approved" | "rejected";
  executionStatus: "pending" | "running" | "success" | "failed" | "skipped";
  diffSummary: string | null;
  arguments: Record<string, unknown>;
  result?: Record<string, unknown> | null;
  errorMessage?: string | null;
  signatureToken?: string | null;
}

/**
 * Active Task Sheet Plan Response (API.md §2.2).
 */
export interface PlanActiveResponse {
  planId: number;
  missionId: number;
  version: number;
  title: string;
  summary: string;
  status:
    | "draft"
    | "pending_approval"
    | "executing"
    | "completed"
    | "rejected"
    | "failed";
  steps: PlanStepDetail[];
}

export interface PlanGetActivePayload {
  missionId: number;
}

export interface GeneratePlanPayload {
  missionId: number;
  userGoal: string;
}

export interface CreatePlanPayload {
  missionId: number;
  title: string;
  summary: string;
  rawTasksheetJson: string;
}

export interface ApproveStepPayload {
  missionId?: number;
  stepId: number;
  signatureToken: string;
}

export interface ApproveStepResponse {
  executed: boolean;
  status: string;
  result?: Record<string, unknown> | null;
}

export interface RejectStepPayload {
  missionId?: number;
  stepId: number;
  reason: string;
}

export interface RejectStepResponse {
  status: string;
}

/**
 * Real-time push event emitted when human approval is required (API.md §3.3).
 */
export interface ApprovalNeededEvent {
  missionId: number;
  planId: number;
  stepId: number;
  appId: string;
  riskTier: 1 | 2 | 3;
  actionTitle: string;
  diffSummary: string;
  checkpointName?: string | null;
  signatureToken: string;
}

/**
 * Immutable Audit Record (DATABASE.md §3.8).
 */
export interface AuditRecord {
  id: number;
  missionId: number | null;
  appId: string;
  actionType: string;
  riskTier: number;
  payloadJson: string;
  status: string;
  signatureToken: string | null;
  createdAt: string;
}

/**
 * Cross-App Pipeline Payloads (API.md §2.2 / PLAN.md §7).
 */
export interface PipelineScreencastPayload {
  missionId: number;
  recordingPath: string;
  cetPath?: string | null;
  cetContent?: string | null;
  cleanAudio: boolean;
  audioPreset?: string | null;
  introScript?: string | null;
  targetTimelineTrack: number;
}

export interface PipelineDiagnosticsPayload {
  missionId: number;
  liveConnectionsCount: number;
  totalBandwidthKbps: number;
  diskUsedGb: number;
  diskFreeGb: number;
  cacheBloatMb: number;
  reclaimableItems: string[];
}



