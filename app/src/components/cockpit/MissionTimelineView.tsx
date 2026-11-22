import React, { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Clock,
  CheckCircle2,
  AlertTriangle,
  RotateCcw,
  FastForward,
  XCircle,
  Shield,
  Layers,
  ChevronDown,
  ChevronUp,
  HelpCircle,
  Terminal,
  Activity,
  RefreshCw,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type {
  MissionGetTimelineResponse,
  MissionTimelineEvent,
  MissionProgressPayload,
  StepStatePayload,
  PlanActiveResponse,
} from "../../types/ipc";

export interface TimelineStepItem {
  stepIndex: number;
  appId: string;
  toolName: string;
  description: string;
  riskTier: 1 | 2 | 3;
  state:
    | "pending"
    | "queued"
    | "running"
    | "paused_approval"
    | "success"
    | "failed"
    | "rolled_back"
    | "skipped";
  progress: number; // 0.0 - 1.0
  message?: string;
  retryCount?: number;
  fallbackRung?: string;
  fallbackReason?: string;
  fallbackEventId?: number;
}

export interface MissionTimelineViewProps {
  missionId: number;
  missionTitle?: string;
  missionGoal?: string;
  activePlan?: PlanActiveResponse | null;
}

export const MissionTimelineView: React.FC<MissionTimelineViewProps> = ({
  missionId,
  missionTitle = "Screen Recording & Audio Denoise Pipeline",
  missionGoal = "Record screen in ScreenStudio Pro, clean hum in Audio Labs, and trim silences in Easy Edits",
  activePlan,
}) => {
  // Timeline events and cursor
  const [events, setEvents] = useState<MissionTimelineEvent[]>([]);
  const [cursor, setCursor] = useState<number>(0);
  const [selectedEventId, setSelectedEventId] = useState<number | null>(null);
  const [isLogDrawerOpen, setIsLogDrawerOpen] = useState<boolean>(false);
  const [isMockMode, setIsMockMode] = useState<boolean>(false);

  // Aggregated progress telemetry
  const [missionProgress, setMissionProgress] = useState<number>(0.65);
  const [elapsedSecs, setElapsedSecs] = useState<number>(48);
  const [etaText, setEtaText] = useState<string>("~15s – 35s");
  const [isStalled, setIsStalled] = useState<boolean>(false);

  // Polling / fetching timeline events from Rust backend
  const fetchTimeline = useCallback(async () => {
    if (!missionId || isMockMode) return;
    try {
      const resp = await invoke<MissionGetTimelineResponse>(
        "cmd_mission_get_timeline",
        {
          payload: {
            missionId,
            sinceId: cursor > 0 ? cursor : null,
          },
        }
      );
      if (resp.events.length > 0) {
        setEvents((prev) => [...prev, ...resp.events]);
        setCursor(resp.cursor);
      }
    } catch (err) {
      console.warn("Failed to fetch mission timeline:", err);
    }
  }, [missionId, cursor, isMockMode]);

  useEffect(() => {
    fetchTimeline();
    const interval = setInterval(fetchTimeline, 1000);
    return () => clearInterval(interval);
  }, [fetchTimeline]);

  // Real-time event listeners for push notifications
  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenStep: (() => void) | undefined;

    async function subscribe() {
      unlistenProgress = await listen<MissionProgressPayload>(
        "evt://mission/progress",
        (event) => {
          if (event.payload.missionId === missionId) {
            setMissionProgress(event.payload.missionProgress);
            setIsStalled(event.payload.isStalled);
            if (event.payload.etaRange) {
              const { p50Ms, p90Ms, isApproximate } = event.payload.etaRange;
              const p50 = Math.round(p50Ms / 1000);
              const p90 = Math.round(p90Ms / 1000);
              setEtaText(`${isApproximate ? "~" : ""}${p50}s – ${p90}s`);
            }
          }
        }
      );

      unlistenStep = await listen<StepStatePayload>(
        "evt://step/state",
        (event) => {
          if (event.payload.missionId === missionId) {
            // Trigger incremental fetch
            fetchTimeline();
          }
        }
      );
    }

    subscribe();
    return () => {
      unlistenProgress?.();
      unlistenStep?.();
    };
  }, [missionId, fetchTimeline]);

  // Elapsed time ticker
  useEffect(() => {
    const timer = setInterval(() => {
      setElapsedSecs((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Canonical mock timeline steps (satisfies Verification for 13.5:
  // "mock mission containing a retry, a fallback and a skipped step")
  const mockSteps: TimelineStepItem[] = useMemo(
    () => [
      {
        stepIndex: 0,
        appId: "ssp",
        toolName: "ssp.recording.start",
        description: "Captures 60fps high resolution screen display",
        riskTier: 1,
        state: "success",
        progress: 1.0,
      },
      {
        stepIndex: 1,
        appId: "ssp",
        toolName: "ssp.recording.stop",
        description: "Finalizes video container and writes CET metadata sidecar",
        riskTier: 1,
        state: "success",
        progress: 1.0,
      },
      {
        stepIndex: 2,
        appId: "alp",
        toolName: "alp.denoise.spectral",
        description: "Spectral subtraction noise reduction on vocal track",
        riskTier: 2,
        state: "success",
        progress: 1.0,
        retryCount: 1, // Step with RETRY
        message: "Applied voice isolation preset after transient socket retry",
      },
      {
        stepIndex: 3,
        appId: "eep",
        toolName: "eep.timeline.cut_interval",
        description: "Precision cuts dead air interval on timeline",
        riskTier: 2,
        state: "success",
        progress: 1.0,
        fallbackRung: "F2 Alt Tool", // Step with FALLBACK
        fallbackReason: "Primary cut lock was contested; engaged fallback tool eep.track.slice_clips",
        fallbackEventId: 104,
      },
      {
        stepIndex: 4,
        appId: "eep",
        toolName: "eep.filter.heavy_vfx",
        description: "Optional cinematic color grade and motion blur",
        riskTier: 1,
        state: "skipped", // SKIPPED step
        progress: 0.0,
        fallbackRung: "F4 Degrade",
        fallbackReason: "CPU memory pressure high; skipped non-critical visual filter",
        fallbackEventId: 105,
      },
      {
        stepIndex: 5,
        appId: "eep",
        toolName: "eep.timeline.export_mp4",
        description: "Exports assembled master timeline to H.264 MP4 container",
        riskTier: 2,
        state: "running",
        progress: 0.45,
        message: "Encoding video frames: chunk 45/100 (1080p60)",
      },
    ],
    []
  );

  // Compute active steps list
  const steps: TimelineStepItem[] = useMemo(() => {
    if (isMockMode || !activePlan || activePlan.steps.length === 0) {
      return mockSteps;
    }

    return activePlan.steps.map((s, idx) => ({
      stepIndex: idx,
      appId: s.appId,
      toolName: s.toolName,
      description: s.diffSummary || s.toolName,
      riskTier: s.riskTier as 1 | 2 | 3,
      state: s.executionStatus as TimelineStepItem["state"],
      progress: s.executionStatus === "success" ? 1.0 : 0.0,
      message: s.executionStatus === "running" ? "Executing..." : undefined,
    }));
  }, [isMockMode, activePlan, mockSteps]);

  // Format mm:ss
  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  };

  // Open log drawer and highlight a specific fallback event
  const handleWhyFallback = (eventId?: number) => {
    setIsLogDrawerOpen(true);
    if (eventId) {
      setSelectedEventId(eventId);
    }
  };

  return (
    <div className="flex flex-col gap-4 w-full">
      {/* 1. Header & Overall Mission Progress Bar */}
      <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/10">
        <div className="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-white/[0.06]">
          <div className="flex items-center gap-2.5">
            <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-sky-500/10 text-sky-400 border border-sky-500/20">
              <Layers className="h-4 w-4" />
            </div>
            <div>
              <h3 className="font-sans text-sm font-semibold text-slate-100 tracking-tight">
                {missionTitle}
              </h3>
              <p className="text-xs text-slate-400 mt-0.5 line-clamp-1">{missionGoal}</p>
            </div>
          </div>

          {/* Right Status Controls */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setIsMockMode((prev) => !prev)}
              className={`text-xs px-2.5 py-1 rounded-lg border transition-colors cursor-pointer ${
                isMockMode
                  ? "bg-amber-500/15 text-amber-300 border-amber-500/30"
                  : "bg-white/[0.04] text-slate-400 border-white/[0.08] hover:text-slate-200"
              }`}
              title="Toggle mock mission demonstrating retry, fallback, and skipped steps"
            >
              {isMockMode ? "Mock Mode Active" : "Load Sample DAG"}
            </button>

            {isStalled && (
              <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-500/15 border border-amber-500/30 text-amber-300 text-xs font-medium animate-pulse">
                <AlertTriangle className="h-3.5 w-3.5" />
                <span>Stalled (no heartbeat)</span>
              </span>
            )}

            <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-white/[0.05] border border-white/10 text-xs text-slate-300 font-mono">
              <Clock className="h-3.5 w-3.5 text-slate-400" />
              <span>{formatTime(elapsedSecs)} elapsed</span>
            </span>

            <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-sky-500/10 border border-sky-500/20 text-xs text-sky-300 font-mono">
              <Activity className="h-3.5 w-3.5 text-sky-400" />
              <span>ETA: {etaText}</span>
            </span>
          </div>
        </div>

        {/* Overall Mission Progress Gauge */}
        <div className="mt-4">
          <div className="flex justify-between items-center text-xs mb-1.5">
            <span className="text-slate-300 font-medium">Mission Execution Progress</span>
            <span className="font-mono text-sky-400 font-semibold">
              {Math.round(missionProgress * 100)}%
            </span>
          </div>
          <div className="w-full h-2 rounded-full bg-slate-800/80 overflow-hidden p-0.5 border border-white/[0.05]">
            <div
              className="h-full rounded-full bg-gradient-to-r from-sky-500 to-teal-400 transition-all duration-300"
              style={{ width: `${Math.min(100, Math.max(0, missionProgress * 100))}%` }}
            />
          </div>
        </div>
      </GlassPanel>

      {/* 2. DAG Timeline Steps View */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between px-1">
          <h4 className="text-xs font-semibold text-slate-300 uppercase tracking-wider">
            Execution Stages & Timeline
          </h4>
          <span className="text-[11px] text-slate-400">
            {steps.filter((s) => s.state === "success").length} of {steps.length} stages completed
          </span>
        </div>

        <div className="grid gap-2">
          {steps.map((step) => {
            const isCompleted = step.state === "success";
            const isRunning = step.state === "running";
            const isSkipped = step.state === "skipped";
            const isFailed = step.state === "failed";
            const isApproval = step.state === "paused_approval";
            const isQueued = step.state === "queued";

            return (
              <GlassPanel
                key={step.stepIndex}
                recipe="pane"
                className={`p-3.5 rounded-xl border transition-all ${
                  isRunning
                    ? "border-sky-500/40 bg-sky-950/20 shadow-[0_0_16px_rgba(56,189,248,0.12)]"
                    : isQueued
                    ? "border-amber-500/40 bg-amber-950/20 shadow-[0_0_16px_rgba(245,158,11,0.12)]"
                    : isApproval
                    ? "border-amber-500/40 bg-amber-950/20"
                    : isCompleted
                    ? "border-white/[0.08] bg-white/[0.02]"
                    : isSkipped
                    ? "border-white/[0.04] bg-white/[0.01] opacity-70"
                    : "border-white/[0.05] bg-white/[0.02]"
                }`}
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  {/* Left: Step Identification & Title */}
                  <div className="flex items-center gap-3">
                    {/* Index Chip */}
                    <div
                      className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg font-mono text-xs font-bold ${
                        isCompleted
                          ? "bg-emerald-500/15 text-emerald-400 border border-emerald-500/30"
                          : isRunning
                          ? "bg-sky-500/20 text-sky-300 border border-sky-400/50 animate-pulse"
                          : isQueued
                          ? "bg-amber-500/20 text-amber-300 border border-amber-400/50"
                          : isSkipped
                          ? "bg-slate-800/60 text-slate-500 border border-slate-700/50"
                          : "bg-slate-800 text-slate-400 border border-slate-700"
                      }`}
                    >
                      {step.stepIndex + 1}
                    </div>

                    <div>
                      <div className="flex items-center gap-2">
                        {/* App Badge */}
                        <span className="font-mono text-[10px] uppercase font-bold tracking-wider px-1.5 py-0.5 rounded bg-white/[0.06] text-slate-300 border border-white/10">
                          {step.appId}
                        </span>
                        <span className="font-mono text-xs font-semibold text-slate-100">
                          {step.toolName}
                        </span>
                      </div>
                      <p className="text-xs text-slate-400 mt-0.5">{step.description}</p>
                    </div>
                  </div>

                  {/* Right: Badges, Markers, State */}
                  <div className="flex flex-wrap items-center gap-2">
                    {/* Risk Tier Badge */}
                    <span
                      className={`flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded-md border ${
                        step.riskTier === 1
                          ? "bg-slate-800/80 text-slate-300 border-slate-700"
                          : step.riskTier === 2
                          ? "bg-amber-500/10 text-amber-300 border-amber-500/30"
                          : "bg-rose-500/15 text-rose-300 border-rose-500/30"
                      }`}
                    >
                      <Shield className="h-3 w-3" />
                      <span>Tier {step.riskTier}</span>
                    </span>

                    {/* Retry Marker */}
                    {step.retryCount && step.retryCount > 0 ? (
                      <span className="flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded-md bg-amber-500/15 text-amber-300 border border-amber-500/30">
                        <RotateCcw className="h-3 w-3" />
                        <span>Retry #{step.retryCount}</span>
                      </span>
                    ) : null}

                    {/* Fallback Marker & "Why" link */}
                    {step.fallbackRung && (
                      <div className="flex items-center gap-1.5">
                        <span className="flex items-center gap-1 text-[10px] font-semibold px-2 py-0.5 rounded-md bg-purple-500/15 text-purple-300 border border-purple-500/30">
                          <FastForward className="h-3 w-3" />
                          <span>{step.fallbackRung}</span>
                        </span>
                        <button
                          onClick={() => handleWhyFallback(step.fallbackEventId)}
                          className="flex items-center gap-1 text-[11px] text-sky-400 hover:text-sky-300 underline cursor-pointer"
                          title={step.fallbackReason}
                        >
                          <HelpCircle className="h-3 w-3" />
                          <span>Why did this fallback happen?</span>
                        </button>
                      </div>
                    )}

                    {/* Step State Chip */}
                    <div className="flex items-center gap-1.5 pl-2 border-l border-white/[0.06]">
                      {isCompleted ? (
                        <span className="flex items-center gap-1 text-xs text-emerald-400 font-medium">
                          <CheckCircle2 className="h-3.5 w-3.5" />
                          <span>Completed</span>
                        </span>
                      ) : isQueued ? (
                        <span className="flex items-center gap-1.5 text-xs text-amber-400 font-medium">
                          <Clock className="h-3.5 w-3.5" />
                          <span>Waiting for resources</span>
                        </span>
                      ) : isRunning ? (
                        <span className="flex items-center gap-1.5 text-xs text-sky-400 font-medium">
                          <RefreshCw className="h-3.5 w-3.5 animate-spin" />
                          <span>Running</span>
                        </span>
                      ) : isSkipped ? (
                        <span className="flex items-center gap-1 text-xs text-slate-500 font-medium">
                          <FastForward className="h-3.5 w-3.5" />
                          <span>Skipped</span>
                        </span>
                      ) : isFailed ? (
                        <span className="flex items-center gap-1 text-xs text-rose-400 font-medium">
                          <XCircle className="h-3.5 w-3.5" />
                          <span>Failed</span>
                        </span>
                      ) : (
                        <span className="text-xs text-slate-500">Pending</span>
                      )}
                    </div>
                  </div>
                </div>

                {/* Queued / Waiting for Resources Banner */}
                {isQueued && (
                  <div className="mt-3 pt-2.5 border-t border-white/[0.05] flex items-center gap-2 text-xs text-amber-300 font-mono">
                    <Clock className="h-3.5 w-3.5 text-amber-400 shrink-0" />
                    <span>{step.message || "Waiting for resources..."}</span>
                  </div>
                )}

                {/* Running Stage Progress Bar */}
                {isRunning && (
                  <div className="mt-3 pt-2.5 border-t border-white/[0.05]">
                    <div className="flex justify-between items-center text-xs mb-1">
                      <span className="text-slate-300 font-mono text-[11px]">
                        {step.message || "Working..."}
                      </span>
                      <span className="font-mono text-sky-400 font-semibold text-[11px]">
                        {Math.round(step.progress * 100)}%
                      </span>
                    </div>
                    <div className="w-full h-1.5 rounded-full bg-slate-800 overflow-hidden">
                      <div
                        className="h-full rounded-full bg-sky-400 transition-all duration-300"
                        style={{ width: `${Math.round(step.progress * 100)}%` }}
                      />
                    </div>
                  </div>
                )}
              </GlassPanel>
            );
          })}
        </div>
      </div>

      {/* 3. Collapsible Live Event Log Drawer */}
      <GlassPanel recipe="pane" className="rounded-2xl border border-white/10 overflow-hidden">
        <button
          onClick={() => setIsLogDrawerOpen((prev) => !prev)}
          className="w-full flex items-center justify-between p-4 text-xs font-semibold text-slate-300 hover:text-slate-100 transition-colors cursor-pointer bg-white/[0.02]"
        >
          <div className="flex items-center gap-2">
            <Terminal className="h-4 w-4 text-sky-400" />
            <span>Mission Timeline Event Log</span>
            <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-slate-800 text-slate-400">
              {events.length > 0 ? `${events.length} events` : "Live telemetry"}
            </span>
          </div>
          {isLogDrawerOpen ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
        </button>

        {isLogDrawerOpen && (
          <div className="p-4 border-t border-white/[0.06] bg-[#07090C]/90 max-h-64 overflow-y-auto font-mono text-xs space-y-1.5">
            {events.length === 0 ? (
              <div className="text-slate-500 py-2 text-center">
                Listening for real-time mission timeline events...
              </div>
            ) : (
              events.map((ev) => {
                const isSelected = selectedEventId === ev.id;
                return (
                  <div
                    key={ev.id}
                    className={`p-2 rounded-lg border transition-all ${
                      isSelected
                        ? "border-amber-500/50 bg-amber-500/10 text-amber-200"
                        : "border-white/[0.03] bg-white/[0.01] text-slate-300"
                    }`}
                  >
                    <div className="flex items-center justify-between text-[11px] text-slate-400">
                      <span className="text-sky-400 font-semibold">{ev.kind}</span>
                      <span>#{ev.id} • {new Date(ev.tsMs).toLocaleTimeString()}</span>
                    </div>
                    <div className="text-slate-200 mt-1 break-all">
                      {JSON.stringify(ev.payload)}
                    </div>
                  </div>
                );
              })
            )}

            {/* Mock events if in mock mode */}
            {isMockMode && events.length === 0 && (
              <div className="space-y-1.5">
                <div
                  className={`p-2 rounded-lg border transition-all ${
                    selectedEventId === 104
                      ? "border-purple-500/50 bg-purple-500/10 text-purple-200"
                      : "border-white/[0.03] bg-white/[0.01] text-slate-300"
                  }`}
                >
                  <div className="flex items-center justify-between text-[11px] text-slate-400">
                    <span className="text-purple-400 font-semibold">fallback_engaged</span>
                    <span>#104 • Event ID 104</span>
                  </div>
                  <div className="text-slate-200 mt-1">
                    Ladder F2 engaged: Resource lock contested on EEP timeline cut interval. Switched to alternative tool eep.track.slice_clips.
                  </div>
                </div>

                <div
                  className={`p-2 rounded-lg border transition-all ${
                    selectedEventId === 105
                      ? "border-amber-500/50 bg-amber-500/10 text-amber-200"
                      : "border-white/[0.03] bg-white/[0.01] text-slate-300"
                  }`}
                >
                  <div className="flex items-center justify-between text-[11px] text-slate-400">
                    <span className="text-amber-400 font-semibold">step_skipped</span>
                    <span>#105 • Event ID 105</span>
                  </div>
                  <div className="text-slate-200 mt-1">
                    Ladder F4 degrade applied: Non-critical visual filter skipped due to memory pressure threshold.
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </GlassPanel>
    </div>
  );
};
