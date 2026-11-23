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
import { GlassPanel } from "../common/GlassPanel";
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
      setStatusMessage(`Error: ${err instanceof Error ? err.message : String(err)}`);
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
          <h2 className="text-xl font-medium text-text-primary flex items-center gap-2">
            <Layers className="h-4 w-4 text-text-secondary" />
            Cross-app pipelines and workflows
          </h2>
          <p className="mt-1 text-xs text-text-secondary">
            End-to-end multi-app automation orchestrating ScreenStudio Pro, Audio Labs Pro, Easy Edits Pro, Net Watch Pro, and Agentic Tuneup Pro.
          </p>
        </div>

        {/* Pipeline Selector Tabs */}
        <div className="flex items-center gap-1 rounded-xl border border-border-subtle bg-white/[0.03] p-1 self-start">
          <button
            onClick={() => setActivePipelineTab("screencast")}
            className={`flex items-center gap-2 rounded-lg px-3 py-2 text-xs font-medium transition-colors ${
              activePipelineTab === "screencast"
                ? "bg-raised text-text-primary border border-border-subtle"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            <Film className="h-3.5 w-3.5" />
            Screencast and audio (SSP → ALP → EEP)
          </button>

          <button
            onClick={() => setActivePipelineTab("diagnostics")}
            className={`flex items-center gap-2 rounded-lg px-3 py-2 text-xs font-medium transition-colors ${
              activePipelineTab === "diagnostics"
                ? "bg-raised text-text-primary border border-border-subtle"
                : "text-text-secondary hover:text-text-primary"
            }`}
          >
            <Activity className="h-3.5 w-3.5" />
            Diagnostics and cleanup (NWP → ATP)
          </button>
        </div>
      </div>

      {statusMessage && (
        <div
          className={`flex items-center gap-2 rounded-xl p-3 text-xs border transition-colors ${
            statusMessage.startsWith("Error")
              ? "border-signal-gate/40 bg-signal-gate-bg text-signal-gate"
              : "border-signal-success/40 bg-signal-success-bg text-signal-success"
          }`}
        >
          {statusMessage.startsWith("Error") ? (
            <AlertCircle className="h-4 w-4 shrink-0" />
          ) : (
            <CheckCircle2 className="h-4 w-4 shrink-0" />
          )}
          <span>{statusMessage}</span>
        </div>
      )}

      {/* Main Studio View */}
      {activePipelineTab === "screencast" ? (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left Column: Pipeline Configuration */}
          <div className="lg:col-span-7 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-border-subtle space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-border-subtle">
                <div className="flex items-center gap-2">
                  <Film className="h-4 w-4 text-text-secondary" />
                  <h3 className="text-sm font-medium text-text-primary">
                    Screencast cut and voice mastering pipeline
                  </h3>
                </div>
                <span className="text-[11px] font-mono text-text-secondary bg-white/[0.05] px-2 py-1 rounded-lg border border-border-subtle">
                  tasksheet/1 · Kahn DAG
                </span>
              </div>

              {/* Source Video & CET Sidecar */}
              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-text-secondary mb-1">
                    ScreenStudio Pro recording file (.mkv)
                  </label>
                  <input
                    type="text"
                    value={recordingPath}
                    onChange={(e) => setRecordingPath(e.target.value)}
                    className="w-full rounded-xl border border-border-subtle bg-black/40 px-3 py-2 text-xs font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                  />
                </div>

                {/* CET Telemetry Source */}
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-text-secondary">
                      Common event telemetry (.cet.jsonl)
                    </label>
                    <button
                      onClick={() => setUseCustomCet(!useCustomCet)}
                      className="text-[11px] text-accent hover:text-accent/80 transition-colors"
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
                        className="w-full rounded-xl border border-border-subtle bg-black/40 p-2 text-[11px] font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                      />
                      <span className="text-[11px] text-text-disabled">
                        Silence intervals of 1200 ms or more trigger automated ripple cuts in EEP.
                      </span>
                    </div>
                  ) : (
                    <input
                      type="text"
                      value={cetPath}
                      onChange={(e) => setCetPath(e.target.value)}
                      className="w-full rounded-xl border border-border-subtle bg-black/40 px-3 py-2 text-xs font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                  )}
                </div>
              </div>

              {/* Audio Enhancement Options (ALP) */}
              <div className="pt-2 border-t border-border-subtle space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Mic className="h-4 w-4 text-text-secondary" />
                    <span className="text-xs font-medium text-text-primary">
                      Audio Labs Pro voice isolation and mastering
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={cleanAudio}
                    onChange={(e) => setCleanAudio(e.target.checked)}
                    className="rounded border-border-glass bg-black/40 accent-accent"
                  />
                </div>

                {cleanAudio && (
                  <div className="grid grid-cols-2 gap-3 pl-6">
                    <div>
                      <label className="block text-[11px] text-text-secondary mb-1">DSP cleanup preset</label>
                      <select
                        value={audioPreset}
                        onChange={(e) => setAudioPreset(e.target.value)}
                        className="w-full rounded-lg border border-border-subtle bg-black/40 px-2 py-2 text-xs text-text-primary focus:outline-none focus:border-accent transition-colors"
                      >
                        <option value="voice_isolate">Voice isolate and de-hum</option>
                        <option value="podcast_master">Podcast mastering (-14 LUFS)</option>
                        <option value="broadcast_clarity">Broadcast clarity</option>
                      </select>
                    </div>

                    <div>
                      <label className="block text-[11px] text-text-secondary mb-1">Target EEP track</label>
                      <select
                        value={timelineTrack}
                        onChange={(e) => setTimelineTrack(Number(e.target.value))}
                        className="w-full rounded-lg border border-border-subtle bg-black/40 px-2 py-2 text-xs text-text-primary focus:outline-none focus:border-accent transition-colors"
                      >
                        <option value={1}>Track 1 (primary vocal)</option>
                        <option value={2}>Track 2 (background music)</option>
                      </select>
                    </div>
                  </div>
                )}
              </div>

              {/* Intro Narration Synthesis (ALP TTS) */}
              <div className="pt-2 border-t border-border-subtle space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Waves className="h-4 w-4 text-text-secondary" />
                    <span className="text-xs font-medium text-text-primary">
                      Synthesize intro narration via ALP TTS
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={includeIntroScript}
                    onChange={(e) => setIncludeIntroScript(e.target.checked)}
                    className="rounded border-border-glass bg-black/40 accent-accent"
                  />
                </div>

                {includeIntroScript && (
                  <div className="pl-6 space-y-1">
                    <textarea
                      value={introScript}
                      onChange={(e) => setIntroScript(e.target.value)}
                      rows={2}
                      className="w-full rounded-lg border border-border-subtle bg-black/40 p-2 text-xs text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span className="text-[11px] text-text-disabled">
                      Intro aligns at timeline start (00:00:00).
                    </span>
                  </div>
                )}
              </div>

              {/* Pipeline Launch Button */}
              <div className="pt-3 border-t border-border-subtle">
                <button
                  onClick={handleRunScreencastPipeline}
                  disabled={isPlanning}
                  className="w-full flex items-center justify-center gap-2 rounded-xl bg-accent text-accent-text hover:bg-accent/90 px-4 py-3 text-xs font-medium disabled:opacity-50 transition-colors active:scale-[0.98] cursor-pointer"
                >
                  <Play className="h-3.5 w-3.5 fill-current" />
                  <span>{isPlanning ? "Resolving DAG pipeline" : "Generate screencast pipeline plan"}</span>
                </button>
              </div>
            </GlassPanel>
          </div>

          {/* Right Column: Execution Stages Preview */}
          <div className="lg:col-span-5 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-border-subtle space-y-4">
              <h3 className="text-xs font-medium text-text-secondary flex items-center gap-2">
                <Scissors className="h-3.5 w-3.5 text-text-secondary" />
                Pipeline execution stages
              </h3>

              <div className="space-y-3 font-sans text-xs">
                {/* Stage 0 */}
                <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3 space-y-1">
                  <div className="flex items-center justify-between text-text-primary font-medium">
                    <span>Stage 0 · Inspection</span>
                    <span className="text-[11px] font-mono text-signal-success bg-signal-success-bg px-2 py-1 rounded-lg border border-signal-success/40">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <p className="text-[11px] text-text-secondary">
                    <span className="font-mono text-text-primary">SSP</span>: Inspects video frames and audio channels in demo recording.
                  </p>
                </div>

                <div className="flex justify-center text-text-disabled">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 1 */}
                <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3 space-y-2">
                  <div className="flex items-center justify-between text-text-primary font-medium">
                    <span>Stage 1 · Audio processing</span>
                    <span className="text-[11px] font-mono text-signal-thinking bg-signal-thinking-bg px-2 py-1 rounded-lg border border-signal-thinking/40">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <div className="space-y-1 text-[11px] text-text-secondary">
                    <p>
                      <span className="font-mono text-text-primary">ALP</span>: Audio cleanup and vocal isolation preset.
                    </p>
                    {includeIntroScript && (
                      <p>
                        <span className="font-mono text-text-primary">ALP</span>: Synthesizes intro narration via TTS.
                      </p>
                    )}
                  </div>
                </div>

                <div className="flex justify-center text-text-disabled">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 2 */}
                <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3 space-y-1">
                  <div className="flex items-center justify-between text-text-primary font-medium">
                    <span>Stage 2 · Timeline cuts</span>
                    <span className="text-[11px] font-mono text-signal-thinking bg-signal-thinking-bg px-2 py-1 rounded-lg border border-signal-thinking/40">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <p className="text-[11px] text-text-secondary">
                    <span className="font-mono text-text-primary">EEP</span>: Applies automated ripple cuts to delete dead air silence intervals.
                  </p>
                </div>

                <div className="flex justify-center text-text-disabled">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 3 */}
                <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3 space-y-1">
                  <div className="flex items-center justify-between text-text-primary font-medium">
                    <span>Stage 3 · Audio alignment</span>
                    <span className="text-[11px] font-mono text-signal-thinking bg-signal-thinking-bg px-2 py-1 rounded-lg border border-signal-thinking/40">
                      Tier 2 · Signed gate
                    </span>
                  </div>
                  <p className="text-[11px] text-text-secondary">
                    <span className="font-mono text-text-primary">EEP</span>: Imports mastered audio and synchronizes intro narration to playhead.
                  </p>
                </div>
              </div>
            </GlassPanel>
          </div>
        </div>
      ) : (
        /* Diagnostics Pipeline View */
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-7 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-border-subtle space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-border-subtle">
                <div className="flex items-center gap-2">
                  <Activity className="h-4 w-4 text-text-secondary" />
                  <h3 className="text-sm font-medium text-text-primary">
                    System health and storage optimization
                  </h3>
                </div>
                <span className="text-[11px] font-mono text-signal-gate bg-signal-gate-bg px-2 py-1 rounded-lg border border-signal-gate/40">
                  Tier 3 · Destructive gate
                </span>
              </div>

              {/* Live Telemetry Overview Cards */}
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-xl border border-border-subtle bg-black/40 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[11px] font-mono text-text-secondary">
                    <span>Network sockets (NWP)</span>
                    <span className="text-accent">{liveConnections} active</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={liveConnections}
                      onChange={(e) => setLiveConnections(Number(e.target.value))}
                      className="w-20 rounded-lg border border-border-subtle bg-black/40 px-2 py-1 text-xs font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span className="text-[11px] text-text-secondary font-mono">sockets</span>
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-text-disabled font-mono pt-1 border-t border-border-subtle">
                    <span>Bandwidth:</span>
                    <input
                      type="number"
                      value={totalBandwidth}
                      step={10}
                      onChange={(e) => setTotalBandwidth(Number(e.target.value))}
                      className="w-24 rounded-lg border border-border-subtle bg-black/40 px-2 py-1 text-[11px] font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span>kbps</span>
                  </div>
                </div>

                <div className="rounded-xl border border-border-subtle bg-black/40 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[11px] font-mono text-text-secondary">
                    <span>Storage space (ATP)</span>
                    <span className="text-text-primary">{diskFreeGb} GB free</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={diskFreeGb}
                      step={1}
                      onChange={(e) => setDiskFreeGb(Number(e.target.value))}
                      className="w-20 rounded-lg border border-border-subtle bg-black/40 px-2 py-1 text-xs font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span className="text-[11px] text-text-secondary font-mono">GB free</span>
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-text-disabled font-mono pt-1 border-t border-border-subtle">
                    <span>Used /home:</span>
                    <input
                      type="number"
                      value={diskUsedGb}
                      step={1}
                      onChange={(e) => setDiskUsedGb(Number(e.target.value))}
                      className="w-20 rounded-lg border border-border-subtle bg-black/40 px-2 py-1 text-[11px] font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span>GB</span>
                  </div>
                </div>
              </div>

              {/* Detected Reclaimable Caches */}
              <div className="space-y-2 pt-2 border-t border-border-subtle">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-text-primary flex items-center gap-2">
                    <Trash2 className="h-3.5 w-3.5" />
                    Reclaimable build artifacts and package caches
                  </span>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={cacheBloatMb}
                      step={100}
                      onChange={(e) => setCacheBloatMb(Number(e.target.value))}
                      className="w-20 rounded-lg border border-border-subtle bg-black/40 px-2 py-1 text-xs font-medium font-mono text-text-primary focus:outline-none focus:border-accent transition-colors"
                    />
                    <span className="text-xs font-medium text-text-secondary font-mono">MB bloat</span>
                  </div>
                </div>

                <div className="rounded-xl border border-border-subtle bg-black/40 p-3 space-y-2">
                  {reclaimableItems.map((item, idx) => (
                    <div key={idx} className="flex items-center justify-between text-xs font-mono text-text-secondary">
                      <span>{item}</span>
                      <span className="text-[11px] text-signal-success">Purgeable</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Safety Arbiter Tier 3 Notice */}
              <div className="rounded-xl border border-signal-gate/30 bg-signal-gate-bg p-3 flex items-start gap-3">
                <ShieldAlert className="h-4 w-4 text-signal-gate shrink-0" />
                <div className="text-xs text-text-secondary">
                  <span className="font-medium text-signal-gate">Tier 3 safety policy:</span> Parallel scans run autonomously, but cache purge strictly halts execution until a signed confirmation token is authorized.
                </div>
              </div>

              {/* Action Button */}
              <button
                onClick={handleRunDiagnosticsPipeline}
                disabled={isPlanning}
                className="w-full flex items-center justify-center gap-2 rounded-xl bg-accent text-accent-text hover:bg-accent/90 px-4 py-3 text-xs font-medium disabled:opacity-50 transition-colors active:scale-[0.98] cursor-pointer"
              >
                <Play className="h-3.5 w-3.5 fill-current" />
                <span>{isPlanning ? "Planning diagnostic DAG" : "Execute health and cache diagnostic plan"}</span>
              </button>
            </GlassPanel>
          </div>

          {/* Right Column: Parallel DAG Topology */}
          <div className="lg:col-span-5 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-border-subtle space-y-4">
              <h3 className="text-xs font-medium text-text-secondary flex items-center gap-2">
                <Sliders className="h-3.5 w-3.5 text-text-secondary" />
                Parallel execution topology
              </h3>

              <div className="space-y-3 font-sans text-xs">
                {/* Stage 0 Parallel */}
                <div className="rounded-xl border border-border-subtle bg-white/[0.03] p-3 space-y-2">
                  <div className="flex items-center justify-between text-text-primary font-medium">
                    <span>Stage 0 · Parallel scans</span>
                    <span className="text-[11px] font-mono text-signal-success bg-signal-success-bg px-2 py-1 rounded-lg border border-signal-success/40">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-[11px] text-text-secondary">
                    <div className="rounded-lg bg-black/40 p-2 border border-border-subtle">
                      <span className="font-mono text-text-primary font-medium block">NWP</span>
                      Scan live sockets and resolve peer hosts
                    </div>
                    <div className="rounded-lg bg-black/40 p-2 border border-border-subtle">
                      <span className="font-mono text-text-primary font-medium block">ATP</span>
                      Scan disk bloat and directory trees
                    </div>
                  </div>
                </div>

                <div className="flex justify-center text-text-disabled">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 1 Destructive Gate */}
                <div className="rounded-xl border border-signal-gate/40 bg-signal-gate-bg p-3 space-y-2">
                  <div className="flex items-center justify-between text-signal-gate font-medium">
                    <span>Stage 1 · Cache purge gate</span>
                    <span className="text-[11px] font-mono text-signal-gate bg-signal-gate-bg px-2 py-1 rounded-lg border border-signal-gate/40">
                      Tier 3 · Signed token
                    </span>
                  </div>
                  <p className="text-[11px] text-text-secondary leading-relaxed">
                    <span className="font-mono text-text-primary">ATP</span>: Purges {(cacheBloatMb / 1024).toFixed(1)} GB of build caches after human signature verification.
                  </p>
                </div>
              </div>
            </GlassPanel>
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
