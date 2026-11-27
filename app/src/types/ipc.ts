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
  | "ipc"
  | "plan";

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
  provider_id: string | null;
  supports_tools: boolean;
  supports_streaming: boolean;
  input_price: number;
  output_price: number;
  is_favorite: boolean;
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
  peak_cpu_pct: number;
  peak_ram_bytes: number;
  peak_temp_celsius: number;
  governor_interventions: number;
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

export interface InferMetricRecord {
  id: number;
  tsMs: number;
  missionId: number | null;
  provider: string;
  modelId: string;
  firstTokenMs: number;
  totalLatencyMs: number;
  tokensIn: number;
  tokensOut: number;
  tokensPerSec: number;
  errorCode: string | null;
}

export interface InferMetricsEvent {
  metric: InferMetricRecord;
}

export interface InferMetricsHistoryPayload {
  range: string;
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

export interface StreamErrorEvent {
  missionId: number;
  message: string;
  code: string;
}

export interface DeleteMissionPayload {
  missionId: number;
}

/**
 * Provider Config Payload.
 */
export interface ProviderInfo {
  providerId: string;
  displayName: string;
  baseUrl: string;
  isEnabled: boolean;
  hasKey: boolean;
}

export interface ProviderIdPayload {
  providerId: string;
}

export interface ProviderTestResponse {
  ok: boolean;
  latencyMs: number;
  modelCount: number;
  errorCode: string | null;
  message: string;
}

export interface DiscoveredModel {
  id: string;
  displayName: string;
  contextWindow: number;
  supportsTools: boolean;
  supportsStreaming: boolean;
  inputPrice: number;
  outputPrice: number;
  raw: unknown;
}

export interface ProviderModelsResponse {
  providerId: string;
  models: DiscoveredModel[];
}

export interface ModelIdTestPayload {
  providerId: string;
  modelId: string;
}

export interface ModelIdTestResponse {
  ok: boolean;
  latencyMs: number;
  errorCode: string | null;
  message: string;
}

export interface ModelDeletePayload {
  modelId: string;
}

export interface ModelFavoritePayload {
  modelId: string;
  favorite: boolean;
}

export interface MissionModelPayload {
  missionId: number;
  modelId: string;
}

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
  bridgeStatus: string;
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

/**
 * Plan Preview & Dry-Run Payload and Responses (GAP-CLOSURE-PLAN.md §12.5).
 */
export interface ResourceProfileEstimate {
  peakVramMb: number;
  peakCpuPct: number;
  diskSpaceRequiredBytes: number;
}

export interface PlanPreviewResponse {
  planGoal: string;
  totalSteps: number;
  tierCounts: Record<number, number>;
  estimatedDurationMs: number;
  resourceProfile: ResourceProfileEstimate;
  requiredCheckpoints: string[];
  requiresHumanApproval: boolean;
}

export interface DryRunStepResult {
  stepIndex: number;
  toolName: string;
  riskTier: number;
  simulated: boolean;
  success: boolean;
  message: string;
}

export interface DryRunReportResponse {
  totalSteps: number;
  tier1Executed: number;
  tier2Simulated: number;
  tier3Simulated: number;
  stepResults: DryRunStepResult[];
  allPassed: boolean;
}

export interface PlanDryRunPayload {
  tasksheetJson: string;
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

/**
 * Event-sourced mission timeline types (GAP-CLOSURE-PLAN.md §13.4).
 */
export interface MissionTimelineEvent {
  id: number;
  missionId: number;
  stepId: number | null;
  tsMs: number;
  kind: string;
  payload: Record<string, unknown>;
}

export interface MissionGetTimelinePayload {
  missionId: number;
  sinceId?: number | null;
}

export interface MissionGetTimelineResponse {
  missionId: number;
  cursor: number;
  events: MissionTimelineEvent[];
}

/**
 * Real-time push event for mission progress (`evt://mission/progress`).
 */
export interface MissionProgressPayload {
  missionId: number;
  missionProgress: number;
  activeStepIndex?: number | null;
  stepProgress: number;
  stepMessage?: string | null;
  etaRange?: {
    p50Ms: number;
    p90Ms: number;
    isApproximate: boolean;
  } | null;
  isStalled: boolean;
  elapsedMs: number;
}

/**
 * Real-time push event for step state transition (`evt://step/state`).
 */
export interface StepStatePayload {
  missionId: number;
  stepId: number;
  stepIndex: number;
  state:
    | "pending"
    | "queued"
    | "running"
    | "paused_approval"
    | "success"
    | "failed"
    | "rolled_back"
    | "skipped";
  failureClass?: string | null;
  error?: string | null;
  attempt: number;
}

/**
 * System tray icon states (GAP-CLOSURE-PLAN.md §13.6).
 */
export type TrayState =
  | "idle"
  | "thinking"
  | "executing"
  | "waiting_approval"
  | "stalled"
  | "overloaded"
  | "failed";

export interface TraySetStatePayload {
  state: TrayState;
}

/**
 * Desktop notification triggers & payloads (GAP-CLOSURE-PLAN.md §13.7).
 */
export type NotificationTrigger =
  | "approval_needed"
  | "mission_finished"
  | "mission_failed"
  | "mission_stalled";

export interface DispatchNotificationPayload {
  trigger: NotificationTrigger;
  missionId: number;
  stepId?: number | null;
  stallSeconds?: number | null;
  title: string;
  body: string;
}

/**
 * Mission History & Replay (GAP-CLOSURE-PLAN.md §13.8).
 */
export interface MissionHistorySummary {
  id: number;
  uuid: string;
  title: string;
  userGoal: string;
  status: string;
  outcome: string;
  durationMs: number;
  fallbackCount: number;
  totalSteps: number;
  completedSteps: number;
  activeModelId: string;
  createdAt: string;
  updatedAt: string;
  peakCpuPct: number;
  peakRamBytes: number;
  peakTempCelsius: number;
  governorInterventions: number;
}

export interface ReplayMissionPayload {
  missionId: number;
}

export interface ReplayMissionResponse {
  missionId: number;
  missionState: string;
  stepStates: Record<number, string>;
  events: MissionTimelineEvent[];
}

/**
 * System Health & Overload Monitor Types (GAP-CLOSURE-PLAN.md §14.8).
 */
export type HealthState = "normal" | "elevated" | "constrained" | "critical";

export interface CpuMetrics {
  globalUtilPct: number;
  coreUtilPct: number[];
  loadAvg1m: number;
  loadAvg5m: number;
  loadAvg15m: number;
  avgFreqMhz: number;
}

export interface PsiMetrics {
  cpuSomeAvg10: number;
  cpuSomeAvg60: number;
  memSomeAvg10: number;
  memSomeAvg60: number;
  memFullAvg10: number;
  memFullAvg60: number;
  ioSomeAvg10: number;
  ioSomeAvg60: number;
  ioFullAvg10: number;
  ioFullAvg60: number;
}

export interface MemoryMetrics {
  totalBytes: number;
  usedBytes: number;
  availableBytes: number;
  availablePct: number;
  swapTotalBytes: number;
  swapUsedBytes: number;
  swapInBytesPerSec: number;
}

export interface DiskMetrics {
  totalBytes: number;
  freeBytes: number;
  freePct: number;
  readBytesPerSec: number;
  writeBytesPerSec: number;
}

export interface NetworkMetrics {
  rxBytesPerSec: number;
  txBytesPerSec: number;
  connectionCount: number;
}

export interface ThermalMetrics {
  maxTempCelsius: number;
  critTempCelsius?: number | null;
  deltaToMax: number;
  fanRpm?: number | null;
}

export interface GpuMetrics {
  deviceName: string;
  utilPct: number;
  vramUsedBytes: number;
  vramTotalBytes: number;
  vramUsedPct: number;
  tempCelsius?: number | null;
}

export interface PowerMetrics {
  onAcPower: boolean;
  batteryPct?: number | null;
}

export interface ProcessMetrics {
  pid: number;
  name: string;
  cpuPct: number;
  memBytes: number;
  isStarshipOwned: boolean;
}

export interface FootprintMetrics {
  totalCpuPct: number;
  totalRssBytes: number;
  openSockets: number;
  processCount: number;
}

export interface HealthSample {
  timestampMs: number;
  cpu?: CpuMetrics | null;
  psi?: PsiMetrics | null;
  memory?: MemoryMetrics | null;
  disk?: DiskMetrics | null;
  network?: NetworkMetrics | null;
  thermal?: ThermalMetrics | null;
  gpu?: GpuMetrics | null;
  power?: PowerMetrics | null;
  topProcesses: ProcessMetrics[];
  ownFootprint?: FootprintMetrics | null;
}

export interface GovernorDirective {
  currentHealthState: HealthState;
  maxParallelism: number;
  allowHeavySteps: boolean;
  allowLocalInference: boolean;
  allowBackgroundPrefetch: boolean;
  shouldPause: boolean;
  shouldResume: boolean;
  shouldUnloadIdleModel: boolean;
  actionMessage: string;
}

export interface HealthSnapshotResponse {
  sample: HealthSample;
  state: HealthState;
  governorDirective: GovernorDirective;
}

export interface HealthHistoryPayload {
  range: "15m" | "1h" | "24h" | "7d" | "30d" | "90d";
  resolution?: "1s" | "1m" | "1h";
}

export interface HealthRollupRecord {
  ts: number;
  cpuAvg: number;
  cpuMax: number;
  memAvailPctMin: number;
  swapInMax: number;
  diskFreePctMin: number;
  tempMax: number;
  gpuUtilMax?: number | null;
  vramPctMax?: number | null;
  psiCpuMax?: number | null;
  psiMemMax?: number | null;
  psiIoMax?: number | null;
  state: string;
}

export interface HealthHistoryResponse {
  range: string;
  samples: HealthSample[];
  rollups: HealthRollupRecord[];
}

export interface OverloadThresholds {
  cpuElevatedPct: number;
  cpuConstrainedPct: number;
  cpuCriticalPct: number;
  cpuExitPct: number;
  psiElevated: number;
  psiConstrained: number;
  psiCritical: number;
  psiExit: number;
  memAvailElevatedPct: number;
  memAvailConstrainedPct: number;
  memAvailCriticalPct: number;
  memAvailExitPct: number;
  swapInCriticalBytesPerSec: number;
  diskFreeElevatedPct: number;
  diskFreeConstrainedPct: number;
  diskFreeCriticalPct: number;
  diskFreeExitPct: number;
  thermalDeltaElevated: number;
  thermalDeltaConstrained: number;
  thermalDeltaCritical: number;
  thermalDeltaExit: number;
  vramElevatedPct: number;
  vramConstrainedPct: number;
  vramCriticalPct: number;
  vramExitPct: number;
  enterConsecutiveSamples: number;
  exitConsecutiveSamples: number;
}

export interface HealthAlert {
  signal: string;
  level: HealthState;
  value: number;
  action: string;
}

export interface HealthStateChangedEvent {
  previousState: HealthState;
  currentState: HealthState;
  directive: GovernorDirective;
}



