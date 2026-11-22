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
  Sparkles,
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
      setStatusMessage("Screencast pipeline generated and active! Review DAG stages below.");
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
      setStatusMessage("System diagnostics and cache cleanup plan generated!");
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
          <h2 className="text-xl font-bold tracking-tight text-slate-100 flex items-center gap-2.5">
            <Sparkles className="h-5 w-5 text-cyan-400" />
            Cross-App Pipelines & Workflows
          </h2>
          <p className="mt-1 text-xs text-slate-400">
            End-to-end multi-app automation orchestrating ScreenStudio Pro, Audio Labs Pro, Easy Edits Pro, Net Watch Pro, and Agentic Tuneup Pro.
          </p>
        </div>

        {/* Pipeline Selector Tabs */}
        <div className="flex items-center gap-1 rounded-xl border border-white/10 bg-white/5 p-1 self-start">
          <button
            onClick={() => setActivePipelineTab("screencast")}
            className={`flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
              activePipelineTab === "screencast"
                ? "bg-cyan-500/20 text-cyan-300 border border-cyan-500/30 shadow-[0_0_12px_rgba(56,189,248,0.2)]"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            <Film className="h-3.5 w-3.5 text-cyan-400" />
            Screencast & Audio (SSP $\to$ ALP $\to$ EEP)
          </button>

          <button
            onClick={() => setActivePipelineTab("diagnostics")}
            className={`flex items-center gap-2 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
              activePipelineTab === "diagnostics"
                ? "bg-purple-500/20 text-purple-300 border border-purple-500/30 shadow-[0_0_12px_rgba(192,132,252,0.2)]"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            <Activity className="h-3.5 w-3.5 text-purple-400" />
            Diagnostics & Cleanup (NWP $\to$ ATP)
          </button>
        </div>
      </div>

      {statusMessage && (
        <div
          className={`flex items-center gap-2 rounded-xl p-3 text-xs border ${
            statusMessage.startsWith("Error")
              ? "border-rose-500/40 bg-rose-500/10 text-rose-300"
              : "border-cyan-500/40 bg-cyan-500/10 text-cyan-300"
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
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/10 space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-white/5">
                <div className="flex items-center gap-2">
                  <Film className="h-4 w-4 text-cyan-400" />
                  <h3 className="text-sm font-semibold text-slate-100">
                    Screencast Cut & Voice Mastering Pipeline
                  </h3>
                </div>
                <span className="text-[10px] font-mono uppercase tracking-wider text-cyan-400 bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-500/30">
                  tasksheet/1 · Kahn DAG
                </span>
              </div>

              {/* Source Video & CET Sidecar */}
              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-slate-300 mb-1">
                    ScreenStudio Pro Recording File (.mkv)
                  </label>
                  <input
                    type="text"
                    value={recordingPath}
                    onChange={(e) => setRecordingPath(e.target.value)}
                    className="w-full rounded-xl border border-white/10 bg-black/40 px-3 py-2 text-xs font-mono text-slate-200 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                  />
                </div>

                {/* CET Telemetry Source */}
                <div>
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs font-medium text-slate-300">
                      Common Event Telemetry (.cet.jsonl)
                    </label>
                    <button
                      onClick={() => setUseCustomCet(!useCustomCet)}
                      className="text-[11px] text-cyan-400 hover:underline"
                    >
                      {useCustomCet ? "Use File Path Instead" : "Edit Telemetry JSONL Directly"}
                    </button>
                  </div>

                  {useCustomCet ? (
                    <div>
                      <textarea
                        value={cetContent}
                        onChange={(e) => setCetContent(e.target.value)}
                        rows={5}
                        className="w-full rounded-xl border border-white/10 bg-black/40 p-2.5 text-[11px] font-mono text-cyan-200/90 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                      />
                      <span className="text-[10px] text-slate-500">
                        Includes silence intervals (≥1200ms trigger automated ripple cuts in EEP).
                      </span>
                    </div>
                  ) : (
                    <input
                      type="text"
                      value={cetPath}
                      onChange={(e) => setCetPath(e.target.value)}
                      className="w-full rounded-xl border border-white/10 bg-black/40 px-3 py-2 text-xs font-mono text-slate-200 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                    />
                  )}
                </div>
              </div>

              {/* Audio Enhancement Options (ALP) */}
              <div className="pt-2 border-t border-white/5 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Mic className="h-4 w-4 text-emerald-400" />
                    <span className="text-xs font-semibold text-slate-200">
                      Audio Labs Pro Voice Isolation & Mastering
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={cleanAudio}
                    onChange={(e) => setCleanAudio(e.target.checked)}
                    className="rounded border-white/20 bg-black/40 text-cyan-500 focus:ring-0"
                  />
                </div>

                {cleanAudio && (
                  <div className="grid grid-cols-2 gap-3 pl-6">
                    <div>
                      <label className="block text-[11px] text-slate-400 mb-1">DSP Cleanup Preset</label>
                      <select
                        value={audioPreset}
                        onChange={(e) => setAudioPreset(e.target.value)}
                        className="w-full rounded-lg border border-white/10 bg-black/40 px-2.5 py-1.5 text-xs text-slate-200 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                      >
                        <option value="voice_isolate">Voice Isolate & De-hum</option>
                        <option value="podcast_master">Podcast Mastering (-14 LUFS)</option>
                        <option value="broadcast_clarity">Broadcast Clarity</option>
                      </select>
                    </div>

                    <div>
                      <label className="block text-[11px] text-slate-400 mb-1">Target EEP Track</label>
                      <select
                        value={timelineTrack}
                        onChange={(e) => setTimelineTrack(Number(e.target.value))}
                        className="w-full rounded-lg border border-white/10 bg-black/40 px-2.5 py-1.5 text-xs text-slate-200 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                      >
                        <option value={1}>Track 1 (Primary Vocal)</option>
                        <option value={2}>Track 2 (Background Music)</option>
                      </select>
                    </div>
                  </div>
                )}
              </div>

              {/* Intro Narration Synthesis (ALP TTS) */}
              <div className="pt-2 border-t border-white/5 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Waves className="h-4 w-4 text-purple-400" />
                    <span className="text-xs font-semibold text-slate-200">
                      Synthesize Intro Narration via ALP TTS
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    checked={includeIntroScript}
                    onChange={(e) => setIncludeIntroScript(e.target.checked)}
                    className="rounded border-white/20 bg-black/40 text-purple-500 focus:ring-0"
                  />
                </div>

                {includeIntroScript && (
                  <div className="pl-6 space-y-1">
                    <textarea
                      value={introScript}
                      onChange={(e) => setIntroScript(e.target.value)}
                      rows={2}
                      className="w-full rounded-lg border border-white/10 bg-black/40 p-2 text-xs text-slate-200 focus:outline-none focus:ring-1 focus:ring-purple-400"
                    />
                    <span className="text-[10px] text-slate-500">
                      Synthesized intro will be aligned at timeline start (00:00:00).
                    </span>
                  </div>
                )}
              </div>

              {/* Pipeline Launch Button */}
              <div className="pt-3 border-t border-white/5">
                <button
                  onClick={handleRunScreencastPipeline}
                  disabled={isPlanning}
                  className="w-full flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-cyan-500 to-blue-600 px-4 py-2.5 text-xs font-bold text-white shadow-lg hover:from-cyan-400 hover:to-blue-500 disabled:opacity-50 transition-all cursor-pointer"
                >
                  <Play className="h-3.5 w-3.5 fill-current" />
                  <span>{isPlanning ? "Resolving DAG Pipeline..." : "Generate Screencast Pipeline Plan"}</span>
                </button>
              </div>
            </GlassPanel>
          </div>

          {/* Right Column: Execution Stages Preview */}
          <div className="lg:col-span-5 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/10 space-y-4">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-2">
                <Scissors className="h-3.5 w-3.5 text-cyan-400" />
                Pipeline Execution Stages
              </h3>

              <div className="space-y-3 font-sans text-xs">
                {/* Stage 0 */}
                <div className="rounded-xl border border-white/10 bg-white/5 p-3 space-y-1">
                  <div className="flex items-center justify-between text-slate-300 font-semibold">
                    <span>Stage 0 · Inspection</span>
                    <span className="text-[10px] font-mono text-emerald-400 bg-emerald-950/60 px-1.5 py-0.5 rounded border border-emerald-500/30">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <p className="text-[11px] text-slate-400">
                    <span className="font-mono text-cyan-300">SSP</span>: Inspects video frames and audio channels in demo recording.
                  </p>
                </div>

                <div className="flex justify-center text-slate-600">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 1 */}
                <div className="rounded-xl border border-white/10 bg-white/5 p-3 space-y-2">
                  <div className="flex items-center justify-between text-slate-300 font-semibold">
                    <span>Stage 1 · Audio Processing</span>
                    <span className="text-[10px] font-mono text-amber-400 bg-amber-950/60 px-1.5 py-0.5 rounded border border-amber-500/30">
                      Tier 2 · Signed Gate
                    </span>
                  </div>
                  <div className="space-y-1 text-[11px] text-slate-400">
                    <p>
                      <span className="font-mono text-emerald-300">ALP</span>: Audio cleanup and vocal isolation preset.
                    </p>
                    {includeIntroScript && (
                      <p>
                        <span className="font-mono text-purple-300">ALP</span>: Synthesizes intro narration via TTS.
                      </p>
                    )}
                  </div>
                </div>

                <div className="flex justify-center text-slate-600">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 2 */}
                <div className="rounded-xl border border-white/10 bg-white/5 p-3 space-y-1">
                  <div className="flex items-center justify-between text-slate-300 font-semibold">
                    <span>Stage 2 · Timeline Cuts</span>
                    <span className="text-[10px] font-mono text-amber-400 bg-amber-950/60 px-1.5 py-0.5 rounded border border-amber-500/30">
                      Tier 2 · Signed Gate
                    </span>
                  </div>
                  <p className="text-[11px] text-slate-400">
                    <span className="font-mono text-blue-300">EEP</span>: Applies automated ripple cuts to delete dead air silence intervals.
                  </p>
                </div>

                <div className="flex justify-center text-slate-600">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 3 */}
                <div className="rounded-xl border border-white/10 bg-white/5 p-3 space-y-1">
                  <div className="flex items-center justify-between text-slate-300 font-semibold">
                    <span>Stage 3 · Audio Alignment</span>
                    <span className="text-[10px] font-mono text-amber-400 bg-amber-950/60 px-1.5 py-0.5 rounded border border-amber-500/30">
                      Tier 2 · Signed Gate
                    </span>
                  </div>
                  <p className="text-[11px] text-slate-400">
                    <span className="font-mono text-blue-300">EEP</span>: Imports mastered audio and synchronizes intro narration to playhead.
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
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/10 space-y-4">
              <div className="flex items-center justify-between pb-3 border-b border-white/5">
                <div className="flex items-center gap-2">
                  <Activity className="h-4 w-4 text-purple-400" />
                  <h3 className="text-sm font-semibold text-slate-100">
                    System Health & Storage Optimization
                  </h3>
                </div>
                <span className="text-[10px] font-mono uppercase tracking-wider text-rose-400 bg-rose-950/60 px-2 py-0.5 rounded border border-rose-500/30">
                  Tier 3 Destructive Gate
                </span>
              </div>

              {/* Live Telemetry Overview Cards */}
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-xl border border-white/10 bg-black/40 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[10px] uppercase font-mono text-slate-400">
                    <span>Network Sockets (NWP)</span>
                    <span className="text-cyan-400">{liveConnections} Active</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={liveConnections}
                      onChange={(e) => setLiveConnections(Number(e.target.value))}
                      className="w-20 rounded border border-white/10 bg-black/60 px-2 py-1 text-xs font-mono text-cyan-300 focus:outline-none focus:ring-1 focus:ring-cyan-400"
                    />
                    <span className="text-[11px] text-slate-400 font-mono">sockets</span>
                  </div>
                  <div className="flex items-center justify-between text-[10px] text-slate-500 font-mono pt-1 border-t border-white/5">
                    <span>Bandwidth:</span>
                    <input
                      type="number"
                      value={totalBandwidth}
                      step={10}
                      onChange={(e) => setTotalBandwidth(Number(e.target.value))}
                      className="w-24 rounded border border-white/10 bg-black/60 px-1.5 py-0.5 text-[10px] font-mono text-slate-300 focus:outline-none"
                    />
                    <span>kbps</span>
                  </div>
                </div>

                <div className="rounded-xl border border-white/10 bg-black/40 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[10px] uppercase font-mono text-slate-400">
                    <span>Storage Space (ATP)</span>
                    <span className="text-purple-400">{diskFreeGb} GB Free</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      value={diskFreeGb}
                      step={1}
                      onChange={(e) => setDiskFreeGb(Number(e.target.value))}
                      className="w-20 rounded border border-white/10 bg-black/60 px-2 py-1 text-xs font-mono text-purple-300 focus:outline-none focus:ring-1 focus:ring-purple-400"
                    />
                    <span className="text-[11px] text-slate-400 font-mono">GB free</span>
                  </div>
                  <div className="flex items-center justify-between text-[10px] text-slate-500 font-mono pt-1 border-t border-white/5">
                    <span>Used /home:</span>
                    <input
                      type="number"
                      value={diskUsedGb}
                      step={1}
                      onChange={(e) => setDiskUsedGb(Number(e.target.value))}
                      className="w-20 rounded border border-white/10 bg-black/60 px-1.5 py-0.5 text-[10px] font-mono text-slate-300 focus:outline-none"
                    />
                    <span>GB</span>
                  </div>
                </div>
              </div>

              {/* Detected Reclaimable Caches */}
              <div className="space-y-2 pt-2 border-t border-white/5">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
                    <Trash2 className="h-3.5 w-3.5 text-rose-400" />
                    Reclaimable Build Artifacts & Package Caches
                  </span>
                  <div className="flex items-center gap-1.5">
                    <input
                      type="number"
                      value={cacheBloatMb}
                      step={100}
                      onChange={(e) => setCacheBloatMb(Number(e.target.value))}
                      className="w-20 rounded border border-white/10 bg-black/60 px-1.5 py-0.5 text-xs font-bold font-mono text-rose-400 focus:outline-none"
                    />
                    <span className="text-xs font-bold text-rose-400 font-mono">MB Bloat</span>
                  </div>
                </div>

                <div className="rounded-xl border border-white/10 bg-black/40 p-3 space-y-1.5">
                  {reclaimableItems.map((item, idx) => (
                    <div key={idx} className="flex items-center justify-between text-xs font-mono text-slate-300">
                      <span>{item}</span>
                      <span className="text-[10px] text-emerald-400">Purgeable</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Safety Arbiter Tier 3 Notice */}
              <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 flex items-start gap-3">
                <ShieldAlert className="h-5 w-5 text-rose-400 shrink-0 mt-0.5" />
                <div className="text-xs text-rose-200">
                  <span className="font-bold">Tier 3 Safety Policy Enforced:</span> Parallel scans run autonomously, but cache purge strictly halts execution until a signed confirmation token is authorized.
                </div>
              </div>

              {/* Action Button */}
              <button
                onClick={handleRunDiagnosticsPipeline}
                disabled={isPlanning}
                className="w-full flex items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-purple-600 to-rose-600 px-4 py-2.5 text-xs font-bold text-white shadow-lg hover:from-purple-500 hover:to-rose-500 disabled:opacity-50 transition-all cursor-pointer"
              >
                <Play className="h-3.5 w-3.5 fill-current" />
                <span>{isPlanning ? "Planning Diagnostic DAG..." : "Execute Health & Cache Diagnostic Plan"}</span>
              </button>
            </GlassPanel>
          </div>

          {/* Right Column: Parallel DAG Topology */}
          <div className="lg:col-span-5 space-y-4">
            <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/10 space-y-4">
              <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 flex items-center gap-2">
                <Sliders className="h-3.5 w-3.5 text-purple-400" />
                Parallel Execution Topology
              </h3>

              <div className="space-y-3 font-sans text-xs">
                {/* Stage 0 Parallel */}
                <div className="rounded-xl border border-cyan-500/30 bg-cyan-500/5 p-3 space-y-2">
                  <div className="flex items-center justify-between text-cyan-300 font-semibold">
                    <span>Stage 0 · Parallel Scans</span>
                    <span className="text-[10px] font-mono text-emerald-400 bg-emerald-950/60 px-1.5 py-0.5 rounded border border-emerald-500/30">
                      Tier 1 · Autonomous
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-[11px] text-slate-300">
                    <div className="rounded-lg bg-black/40 p-2 border border-white/5">
                      <span className="font-mono text-cyan-400 font-bold block">NWP</span>
                      Scan live sockets and resolve peer hosts
                    </div>
                    <div className="rounded-lg bg-black/40 p-2 border border-white/5">
                      <span className="font-mono text-purple-400 font-bold block">ATP</span>
                      Scan disk bloat and directory trees
                    </div>
                  </div>
                </div>

                <div className="flex justify-center text-slate-600">
                  <ArrowRight className="h-3.5 w-3.5 rotate-90" />
                </div>

                {/* Stage 1 Destructive Gate */}
                <div className="rounded-xl border border-rose-500/40 bg-rose-500/10 p-3 space-y-2">
                  <div className="flex items-center justify-between text-rose-300 font-semibold">
                    <span>Stage 1 · Cache Purge Gate</span>
                    <span className="text-[10px] font-mono text-rose-400 bg-rose-950/60 px-1.5 py-0.5 rounded border border-rose-500/30">
                      Tier 3 · Signed Token
                    </span>
                  </div>
                  <p className="text-[11px] text-rose-200/90 leading-relaxed">
                    <span className="font-mono text-amber-300">ATP</span>: Purges {(cacheBloatMb / 1024).toFixed(1)} GB of build caches after human signature verification.
                  </p>
                </div>
              </div>
            </GlassPanel>
          </div>
        </div>
      )}

      {/* Active Plan and Approval Section */}
      {activePlan && (
        <div className="pt-4 border-t border-white/10 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-bold text-slate-100 flex items-center gap-2">
              <FileCode className="h-4 w-4 text-cyan-400" />
              Active Pipeline Execution Plan
            </h3>
            <span className="text-xs font-mono text-slate-400">
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
