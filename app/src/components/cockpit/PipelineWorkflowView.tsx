import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Film,
  Mic,
  Scissors,
  Activity,
  Trash2,
  Play,
  CheckCircle2,
  AlertCircle,
  Layers,
  Sliders,
  FileCode,
  ArrowRight,
  ShieldAlert,
  Waves,
} from "lucide-react";
import { LinearCard, LinearButton } from "../linear";
import { PlanWorkflowViewer } from "./PlanWorkflowViewer";
import { ActionApprovalCard } from "./ActionApprovalCard";
import type {
  PlanActiveResponse,
  ApprovalNeededEvent,
  PlanStepDetail,
  ApproveStepResponse,
  RejectStepResponse,
  PipelineScreencastPayload,
  PipelineDiagnosticsPayload,
} from "../../types/ipc";

export interface PipelineWorkflowViewProps {
  missionId: number;
  activePlan: PlanActiveResponse | null;
  approvalGate: ApprovalNeededEvent | null;
  onPlanUpdated: (plan: PlanActiveResponse | null) => void;
  onApprovalGateUpdated: (gate: ApprovalNeededEvent | null) => void;
}

const SAMPLE_CET_JSONL = `{"type":"header","version":"cet/1.0","recording_fps":60.0,"duration_ms":90000,"audio_sample_rate":48000}
 {"type":"marker","timestamp_ms":0,"label":"Recording Started"}
 {"type":"silence","start_ms":1000,"end_ms":4200,"duration_ms":3200}
 {"type":"click","timestamp_ms":5500,"button":"left","x":410.5,"y":220.0,"window_title":"Easy Edits Pro"}
 {"type":"silence","start_ms":15000,"end_ms":22500,"duration_ms":7500}
 {"type":"window_switch","timestamp_ms":23000,"app_name":"ssp","window_title":"ScreenStudio Pro"}
 {"type":"silence","start_ms":45000,"end_ms":45800,"duration_ms":800}
 `;

export const PipelineWorkflowView: React.FC<PipelineWorkflowViewProps> = ({
  missionId,
  activePlan,
  approvalGate: _approvalGate,
  onPlanUpdated,
  onApprovalGateUpdated,
}) => {
  const [activePipelineTab, setActivePipelineTab] = useState<"screencast" | "diagnostics">("screencast");
  const [isPlanning, setIsPlanning] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  // Screencast Pipeline Form State
  const [recordingPath, setRecordingPath] = useState("/home/user/Videos/starship-screencast-take01.mkv");
  const [cetPath, setCetPath] = useState("/home/user/Videos/starship-screencast-take01.cet.jsonl");
  const [cetContent, setCetContent] = useState(SAMPLE_CET_JSONL);
  const [useCustomCet, setUseCustomCet] = useState(true);
  const [cleanAudio, setCleanAudio] = useState(true);
  const [audioPreset, setAudioPreset] = useState("voice_isolate");
  const [includeIntroScript, setIncludeIntroScript] = useState(true);
  const [introScript, setIntroScript] = useState(
    "Welcome aboard Starship Control. In this screencast, we demonstrate native Linux multi-app orchestration."
  );
  const [timelineTrack, setTimelineTrack] = useState(1);

  // Diagnostics Pipeline Form State
  const [liveConnections, setLiveConnections] = useState(14);
  const [totalBandwidth, setTotalBandwidth] = useState(480.5);
  const [diskUsedGb, setDiskUsedGb] = useState(128.4);
  const [diskFreeGb, setDiskFreeGb] = useState(38.6);
  const [cacheBloatMb, setCacheBloatMb] = useState(4300);
  const [reclaimableItems] = useState([
    "~/.cache/cargo/registry (1.8 GB)",
    "~/.cache/npm (1.2 GB)",
    "~/.cache/thumbnails (850 MB)",
    "/var/tmp/build-artifacts (450 MB)",
  ]);

  // Execute Screencast Pipeline
  const handleRunScreencastPipeline = async () => {
    setIsPlanning(true);
    setStatusMessage(null);
    try {
      const payload: PipelineScreencastPayload = {
        missionId,
        recordingPath,
        cetPath: useCustomCet ? null : cetPath,
        cetContent: useCustomCet ? cetContent : null,
        cleanAudio,
        audioPreset,
        introScript: includeIntroScript ? introScript : null,
        targetTimelineTrack: timelineTrack,
      };

      const plan = await invoke<PlanActiveResponse>("cmd_pipeline_screencast", { payload });
      onPlanUpdated(plan);
      setStatusMessage("Screencast pipeline generated and active — review DAG stages below.");
    } catch (err: unknown) {
      console.error("Failed to plan screencast pipeline:", err);
      const core = err as { userMessage?: unknown } | null;
      const message =
        core && typeof core.userMessage === "string"
          ? core.userMessage
          : err instanceof Error
            ? err.message
            : String(err);
      setStatusMessage(message);
    } finally {
      setIsPlanning(false);
    }
  };

  // Execute Diagnostics Pipeline
  const handleRunDiagnosticsPipeline = async () => {
    setIsPlanning(true);
    setStatusMessage(null);
    try {
      const payload: PipelineDiagnosticsPayload = {
        missionId,
        liveConnectionsCount: liveConnections,
        totalBandwidthKbps: totalBandwidth,
        diskUsedGb,
        diskFreeGb,
        cacheBloatMb,
        reclaimableItems: [
          "~/.cache/cargo/registry",
          "~/.cache/npm",
          "~/.cache/thumbnails",
          "/var/tmp/build-artifacts",
        ],
      };

      const plan = await invoke<PlanActiveResponse>("cmd_pipeline_diagnostics", { payload });
      onPlanUpdated(plan);
      setStatusMessage("System diagnostics and cache cleanup plan generated");
    } catch (err: unknown) {
      console.error("Failed to plan diagnostics pipeline:", err);
      setStatusMessage(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsPlanning(false);
    }
  };

  // Step Approval handler
  const handleApprove = async (stepId: number, signatureToken: string) => {
    try {
      await invoke<ApproveStepResponse>("cmd_plan_approve_step", {
        payload: { missionId, stepId, signatureToken },
      });
      onApprovalGateUpdated(null);
      // Refresh active plan
      const updatedPlan = await invoke<PlanActiveResponse | null>("cmd_plan_get_active", {
        payload: { missionId },
      });
      onPlanUpdated(updatedPlan);
    } catch (err) {
      console.error("Step approval failed:", err);
    }
  };

  // Step Rejection handler
  const handleReject = async (stepId: number, reason: string) => {
    try {
      await invoke<RejectStepResponse>("cmd_plan_reject_step", {
        payload: { missionId, stepId, reason },
      });
      onApprovalGateUpdated(null);
      // Refresh active plan
      const updatedPlan = await invoke<PlanActiveResponse | null>("cmd_plan_get_active", {
        payload: { missionId },
      });
      onPlanUpdated(updatedPlan);
    } catch (err) {
      console.error("Step rejection failed:", err);
    }
  };

  // Find step pending approval for approval card display
  const pendingStep: PlanStepDetail | undefined = activePlan?.steps.find(
    (s) => s.riskTier >= 2 && s.approvalStatus === "pending"
  );

  return (
    <div className="space-y-6">
      {/* Header & Sub-Navigation */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-[16px] font-semibold text-[var(--text)] flex items-center gap-2">
            <Layers className="size-4 text-[var(--accent-blue)]" />
            Cross-app pipelines and workflows
          </h2>
          <p className="mt-1 text-[12px] text-[var(--text-muted)]">
            End-to-end multi-app automation orchestrating ScreenStudio Pro, Audio Labs Pro, Easy Edits Pro, Net Watch Pro, and Agentic Tuneup Pro.
          </p>
        </div>

        <div className="flex items-center gap-1 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] p-1 self-start">
          <button
            type="button"
            onClick={() => setActivePipelineTab("screencast")}
            className={`flex items-center gap-2 rounded-[4px] px-3 py-1.5 text-[12px] font-medium transition-colors cursor-pointer ${
              activePipelineTab === "screencast"
                ? "bg-[var(--bg-surface-2)] text-[var(--text)] border border-[var(--border)]"
                : "text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent"
            }`}
          >
            <Film className="size-3.5" />
            Screencast and audio (SSP → ALP → EEP)
          </button>

          <button
            type="button"
            onClick={() => setActivePipelineTab("diagnostics")}
            className={`flex items-center gap-2 rounded-[4px] px-3 py-1.5 text-[12px] font-medium transition-colors cursor-pointer ${
              activePipelineTab === "diagnostics"
                ? "bg-[var(--bg-surface-2)] text-[var(--text)] border border-[var(--border)]"
                : "text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent"
            }`}
          >
            <Activity className="size-3.5" />
            Diagnostics and cleanup (NWP → ATP)
          </button>
        </div>
      </div>

      {statusMessage && (
        <div
          className={`flex items-center gap-2 rounded-[8px] p-3 text-[12px] border transition-colors ${
            statusMessage.startsWith("Error")
              ? "border-[var(--accent-red)] bg-[var(--tint-red)] text-[var(--accent-red)]"
              : "border-[var(--accent-green)] bg-[var(--tint-green)] text-[var(--accent-green)]"
          }`}
        >
          {statusMessage.startsWith("Error") ? (
            <AlertCircle className="size-4 shrink-0" />
          ) : (
            <CheckCircle2 className="size-4 shrink-0" />
          )}
          <span>{statusMessage}</span>
        </div>
      )}

      {activePipelineTab === "screencast" ? (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
          <div className="lg:col-span-7 space-y-4">
            <LinearCard level={1} className="p-5 space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-[var(--border)]">
                <div className="flex items-center gap-2">
                  <Film className="size-4 text-[var(--text-muted)]" />
                  <h3 className="text-[14px] font-semibold text-[var(--text)]">
                    Screencast cut and voice mastering pipeline
                  </h3>
                </div>
                <span className="text-[12px] font-mono text-[var(--text-muted)] bg-[var(--bg-surface-2)] px-2 py-0.5 rounded-[4px] border border-[var(--border)]">
                  tasksheet/1 · Kahn DAG
                </span>
              </div>

              <div className="space-y-3">
                <div>
                  <label className="block text-[12px] font-medium text-[var(--text-muted)] mb-1">
                    ScreenStudio Pro recording file (.mkv)
                  </label>
                  <input
                    type="text"
                    value={recordingPath}
                    onChange={(e) => setRecordingPath(e.target.value)}
                    className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-3 py-2 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                  />
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-[12px] font-medium text-[var(--text-muted)]">
                      Common event telemetry (.cet.jsonl)
                    </label>
                    <button
                      type="button"
                      onClick={() => setUseCustomCet(!useCustomCet)}
                      className="text-[12px] text-[var(--accent-blue)] hover:underline cursor-pointer"
                    >
                      {useCustomCet ? "Use file path" : "Edit telemetry JSONL"}
                    </button>
                  </div>

                  {useCustomCet ? (
                    <div>
                      <textarea
                        value={cetContent}
                        onChange={(e) => setCetContent(e.target.value)}
                        rows={5}
                        className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2.5 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                      />
                      <span className="text-[12px] text-[var(--text-muted)]">
                        Silence intervals of 1200 ms or more trigger automated ripple cuts in EEP.
                      </span>
                    </div>
                  ) : (
                    <input
                      type="text"
                      value={cetPath}
                      onChange={(e) => setCetPath(e.target.value)}
                      className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-3 py-2 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                  )}
                </div>
              </div>

              <div className="pt-2 border-t border-[var(--border)] space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Mic className="size-4 text-[var(--text-muted)]" />
                    <span className="text-[12px] font-medium text-[var(--text)]">
                      Audio Labs Pro voice isolation and mastering
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={cleanAudio}
                    onChange={(e) => setCleanAudio(e.target.checked)}
                    className="rounded-[4px] border-[var(--border)] bg-[var(--bg-surface-2)] accent-[var(--accent-blue)]"
                  />
                </div>

                {cleanAudio && (
                  <div className="grid grid-cols-2 gap-3 pl-6">
                    <div>
                      <label className="block text-[12px] text-[var(--text-muted)] mb-1">DSP cleanup preset</label>
                      <select
                        value={audioPreset}
                        onChange={(e) => setAudioPreset(e.target.value)}
                        className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-2 text-[12px] text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                      >
                        <option value="voice_isolate">Voice isolate and de-hum</option>
                        <option value="podcast_master">Podcast mastering (-14 LUFS)</option>
                        <option value="broadcast_clarity">Broadcast clarity</option>
                      </select>
                    </div>

                    <div>
                      <label className="block text-[12px] text-[var(--text-muted)] mb-1">Target EEP track</label>
                      <select
                        value={timelineTrack}
                        onChange={(e) => setTimelineTrack(Number(e.target.value))}
                        className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-2 text-[12px] text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                      >
                        <option value={1}>Track 1 (primary vocal)</option>
                        <option value={2}>Track 2 (background music)</option>
                      </select>
                    </div>
                  </div>
                )}
              </div>

              <div className="pt-2 border-t border-[var(--border)] space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Waves className="size-4 text-[var(--text-muted)]" />
                    <span className="text-[12px] font-medium text-[var(--text)]">
                      Synthesize intro narration via ALP TTS
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={includeIntroScript}
                    onChange={(e) => setIncludeIntroScript(e.target.checked)}
                    className="rounded-[4px] border-[var(--border)] bg-[var(--bg-surface-2)] accent-[var(--accent-blue)]"
                  />
                </div>

                {includeIntroScript && (
                  <div className="pl-6 space-y-1">
                    <textarea
                      value={introScript}
                      onChange={(e) => setIntroScript(e.target.value)}
                      rows={2}
                      className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2 text-[12px] text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span className="text-[12px] text-[var(--text-muted)]">
                      Intro aligns at timeline start (00:00:00).
                    </span>
                  </div>
                )}
              </div>

              <div className="pt-3 border-t border-[var(--border)]">
                <LinearButton
                  variant="primary"
                  size="lg"
                  onClick={handleRunScreencastPipeline}
                  disabled={isPlanning}
                  className="w-full"
                >
                  <Play className="size-3.5 fill-current" />
                  <span>{isPlanning ? "Resolving DAG pipeline..." : "Generate screencast pipeline plan"}</span>
                </LinearButton>
              </div>
            </LinearCard>
          </div>

          <div className="lg:col-span-5 space-y-4">
            <LinearCard level={1} className="p-5 space-y-4">
              <h3 className="text-[13px] font-semibold text-[var(--text)] flex items-center gap-2">
                <Scissors className="size-3.5 text-[var(--accent-blue)]" />
                Pipeline execution stages
              </h3>

              <div className="space-y-3 font-sans text-[12px]">
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-1">
                  <div className="flex items-center justify-between text-[var(--text)] font-semibold">
                    <span>Stage 0 · Inspection</span>
                    <span className="text-[12px] font-mono font-semibold text-[var(--accent-green)] bg-[var(--tint-green)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-green)]">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <p className="text-[12px] text-[var(--text-muted)]">
                    <span className="font-mono text-[var(--text)] font-semibold">SSP</span>: Inspects video frames and audio channels in demo recording.
                  </p>
                </div>

                <div className="flex justify-center text-[var(--text-muted)]">
                  <ArrowRight className="size-3.5 rotate-90" />
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-2">
                  <div className="flex items-center justify-between text-[var(--text)] font-semibold">
                    <span>Stage 1 · Audio processing</span>
                    <span className="text-[12px] font-mono font-semibold text-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-amber)]">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <div className="space-y-1 text-[12px] text-[var(--text-muted)]">
                    <p>
                      <span className="font-mono text-[var(--text)] font-semibold">ALP</span>: Audio cleanup and vocal isolation preset.
                    </p>
                    {includeIntroScript && (
                      <p>
                        <span className="font-mono text-[var(--text)] font-semibold">ALP</span>: Synthesizes intro narration via TTS.
                      </p>
                    )}
                  </div>
                </div>

                <div className="flex justify-center text-[var(--text-muted)]">
                  <ArrowRight className="size-3.5 rotate-90" />
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-1">
                  <div className="flex items-center justify-between text-[var(--text)] font-semibold">
                    <span>Stage 2 · Timeline cuts</span>
                    <span className="text-[12px] font-mono font-semibold text-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-amber)]">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <p className="text-[12px] text-[var(--text-muted)]">
                    <span className="font-mono text-[var(--text)] font-semibold">EEP</span>: Applies automated ripple cuts to delete dead air silence intervals.
                  </p>
                </div>

                <div className="flex justify-center text-[var(--text-muted)]">
                  <ArrowRight className="size-3.5 rotate-90" />
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-1">
                  <div className="flex items-center justify-between text-[var(--text)] font-semibold">
                    <span>Stage 3 · Audio alignment</span>
                    <span className="text-[12px] font-mono font-semibold text-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-amber)]">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <p className="text-[12px] text-[var(--text-muted)]">
                    <span className="font-mono text-[var(--text)] font-semibold">EEP</span>: Imports mastered audio and synchronizes intro narration to playhead.
                  </p>
                </div>
              </div>
            </LinearCard>
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
          <div className="lg:col-span-7 space-y-4">
            <LinearCard level={1} className="p-5 space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-[var(--border)]">
                <div className="flex items-center gap-2">
                  <Activity className="size-4 text-[var(--text-muted)]" />
                  <h3 className="text-[14px] font-semibold text-[var(--text)]">
                    System health and storage optimization
                  </h3>
                </div>
                <span className="text-[12px] font-mono text-[var(--accent-red)] bg-[var(--tint-red)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-red)]">
                  Tier 3 · Destructive gate
                </span>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-2">
                  <div className="flex items-center justify-between text-[12px] font-mono text-[var(--text-muted)]">
                    <span>Network sockets (NWP)</span>
                    <span className="text-[var(--accent-blue)]">{liveConnections} active</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={liveConnections}
                      onChange={(e) => setLiveConnections(Number(e.target.value))}
                      className="w-20 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span className="text-[12px] text-[var(--text-muted)] font-mono">sockets</span>
                  </div>
                  <div className="flex items-center justify-between text-[12px] text-[var(--text-muted)] font-mono pt-1 border-t border-[var(--border)]">
                    <span>Bandwidth:</span>
                    <input
                      type="number"
                      value={totalBandwidth}
                      step={10}
                      onChange={(e) => setTotalBandwidth(Number(e.target.value))}
                      className="w-24 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span>kbps</span>
                  </div>
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-2">
                  <div className="flex items-center justify-between text-[12px] font-mono text-[var(--text-muted)]">
                    <span>Storage space (ATP)</span>
                    <span className="text-[var(--text)] font-semibold">{diskFreeGb} GB free</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={diskFreeGb}
                      step={1}
                      onChange={(e) => setDiskFreeGb(Number(e.target.value))}
                      className="w-20 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span className="text-[12px] text-[var(--text-muted)] font-mono">GB free</span>
                  </div>
                  <div className="flex items-center justify-between text-[12px] text-[var(--text-muted)] font-mono pt-1 border-t border-[var(--border)]">
                    <span>Used /home:</span>
                    <input
                      type="number"
                      value={diskUsedGb}
                      step={1}
                      onChange={(e) => setDiskUsedGb(Number(e.target.value))}
                      className="w-20 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] px-2 py-1 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span>GB</span>
                  </div>
                </div>
              </div>

              <div className="space-y-2 pt-2 border-t border-[var(--border)]">
                <div className="flex items-center justify-between">
                  <span className="text-[12px] font-medium text-[var(--text)] flex items-center gap-2">
                    <Trash2 className="size-3.5" />
                    Reclaimable build artifacts and package caches
                  </span>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={cacheBloatMb}
                      step={100}
                      onChange={(e) => setCacheBloatMb(Number(e.target.value))}
                      className="w-20 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2 py-1 text-[12px] font-mono text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
                    />
                    <span className="text-[12px] text-[var(--text-muted)] font-mono">MB bloat</span>
                  </div>
                </div>

                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-2">
                  {reclaimableItems.map((item, idx) => (
                    <div key={idx} className="flex items-center justify-between text-[12px] font-mono text-[var(--text-muted)]">
                      <span>{item}</span>
                      <span className="text-[var(--accent-green)]">Purgeable</span>
                    </div>
                  ))}
                </div>
              </div>

              <div className="rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] p-3 flex items-start gap-3">
                <ShieldAlert className="size-4 text-[var(--accent-red)] shrink-0" />
                <div className="text-[12px] text-[var(--text-muted)]">
                  <span className="font-medium text-[var(--accent-red)]">Tier 3 safety policy:</span> Parallel scans run autonomously, but cache purge strictly halts execution until a signed confirmation token is authorized.
                </div>
              </div>

              <LinearButton
                variant="primary"
                size="lg"
                onClick={handleRunDiagnosticsPipeline}
                disabled={isPlanning}
                className="w-full"
              >
                <Play className="size-3.5 fill-current" />
                <span>{isPlanning ? "Planning diagnostic DAG" : "Execute health and cache diagnostic plan"}</span>
              </LinearButton>
            </LinearCard>
          </div>

          <div className="lg:col-span-5 space-y-4">
            <LinearCard level={1} className="p-5 space-y-4">
              <h3 className="text-[13px] font-semibold text-[var(--text)] flex items-center gap-2">
                <Sliders className="size-3.5 text-[var(--text-muted)]" />
                Parallel execution topology
              </h3>

              <div className="space-y-3 font-sans text-[12px]">
                <div className="rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-3 space-y-2">
                  <div className="flex items-center justify-between text-[var(--text)] font-medium">
                    <span>Stage 0 · Parallel scans</span>
                    <span className="text-[12px] font-mono text-[var(--accent-green)] bg-[var(--tint-green)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-green)]">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-[12px] text-[var(--text-muted)]">
                    <div className="rounded-[6px] bg-[var(--bg-surface)] p-2 border border-[var(--border)]">
                      <span className="font-mono text-[var(--text)] font-medium block">NWP</span>
                      Scan live sockets and resolve peer hosts
                    </div>
                    <div className="rounded-[6px] bg-[var(--bg-surface)] p-2 border border-[var(--border)]">
                      <span className="font-mono text-[var(--text)] font-medium block">ATP</span>
                      Scan disk bloat and directory trees
                    </div>
                  </div>
                </div>

                <div className="flex justify-center text-[var(--text-muted)]">
                  <ArrowRight className="size-3.5 rotate-90" />
                </div>

                <div className="rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] p-3 space-y-2">
                  <div className="flex items-center justify-between text-[var(--accent-red)] font-medium">
                    <span>Stage 1 · Cache purge gate</span>
                    <span className="text-[12px] font-mono text-[var(--accent-red)] bg-[var(--tint-red)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-red)]">
                      Tier 3 · Signed token
                    </span>
                  </div>
                  <p className="text-[12px] text-[var(--text-muted)] leading-relaxed">
                    <span className="font-mono text-[var(--text)]">ATP</span>: Purges {(cacheBloatMb / 1024).toFixed(1)} GB of build caches after human signature verification.
                  </p>
                </div>
              </div>
            </LinearCard>
          </div>
        </div>
      )}

      {/* Active Plan and Approval Section */}
      {activePlan && (
        <div className="pt-4 border-t border-border-subtle space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium text-text-primary flex items-center gap-2">
              <FileCode className="h-4 w-4 text-text-secondary" />
              Active pipeline execution plan
            </h3>
            <span className="text-xs font-mono text-text-secondary">
              Mission ID: #{missionId} · Plan #{activePlan.planId}
            </span>
          </div>

          {/* Pending Approval Card */}
          {pendingStep && (
            <ActionApprovalCard
              missionId={missionId}
              step={pendingStep}
              planTitle={activePlan.title}
              onApprove={handleApprove}
              onReject={handleReject}
            />
          )}

          {/* Plan Workflow Steps Visualizer */}
          <PlanWorkflowViewer plan={activePlan} />
        </div>
      )}
    </div>
  );
};
