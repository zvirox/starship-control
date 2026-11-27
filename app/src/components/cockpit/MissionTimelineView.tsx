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
import { LinearCard } from "../linear";
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

  // Aggregated progress telemetry
  const [missionProgress, setMissionProgress] = useState<number>(0.65);
  const [elapsedSecs, setElapsedSecs] = useState<number>(48);
  const [etaText, setEtaText] = useState<string>("~15s – 35s");
  const [isStalled, setIsStalled] = useState<boolean>(false);

  // Polling / fetching timeline events from Rust backend
  const fetchTimeline = useCallback(async () => {
    if (!missionId) return;
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
  }, [missionId, cursor]);

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

  // Steps come only from the active plan loaded via IPC (no sample DAG).
  const steps: TimelineStepItem[] = useMemo(() => {
    if (!activePlan || activePlan.steps.length === 0) {
      return [];
    }

    return activePlan.steps.map((s, idx) => ({
      stepIndex: idx,
      appId: s.appId,
      toolName: s.toolName,
      description: s.diffSummary || s.toolName,
      riskTier: s.riskTier as 1 | 2 | 3,
      state: s.executionStatus as TimelineStepItem["state"],
      progress: s.executionStatus === "success" ? 1.0 : 0.0,
      message: s.executionStatus === "running" ? "Executing" : undefined,
    }));
  }, [activePlan]);

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
      <LinearCard level={1} className="p-5">
        <div className="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-[var(--border)]">
          <div className="flex items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded-[6px] bg-[var(--tint-blue)] text-[var(--accent-blue)]">
              <Layers className="size-4" />
            </div>
            <div>
              <h3 className="font-sans text-[14px] font-semibold text-[var(--text)]">
                {missionTitle}
              </h3>
              <p className="text-[12px] text-[var(--text-muted)] mt-0.5 line-clamp-1">{missionGoal}</p>
            </div>
          </div>

          <div className="flex items-center gap-2">

            {isStalled && (
              <span className="flex items-center gap-1.5 px-2 py-1 rounded-[4px] bg-[var(--tint-amber)] border border-[var(--accent-amber)] text-[var(--accent-amber)] text-[12px] font-medium">
                <AlertTriangle className="size-3.5" />
                <span>Stalled (no heartbeat)</span>
              </span>
            )}

            <span className="flex items-center gap-1.5 px-2 py-1 rounded-[4px] bg-[var(--bg-surface-2)] border border-[var(--border)] text-[12px] text-[var(--text-muted)] font-mono">
              <Clock className="size-3.5" />
              <span>{formatTime(elapsedSecs)} elapsed</span>
            </span>

            <span className="flex items-center gap-1.5 px-2 py-1 rounded-[4px] bg-[var(--tint-blue)] border border-[var(--border)] text-[12px] text-[var(--accent-blue)] font-mono">
              <Activity className="size-3.5" />
              <span>ETA: {etaText}</span>
            </span>
          </div>
        </div>

        <div className="mt-4">
          <div className="flex justify-between items-center text-[12px] mb-2">
            <span className="text-[var(--text-muted)] font-medium">Mission execution progress</span>
            <span className="font-mono text-[var(--accent-blue)] font-medium tabular-nums">
              {Math.round(missionProgress * 100)}%
            </span>
          </div>
          <div className="w-full h-1.5 rounded-[2px] bg-[var(--bg-bar-idle)] overflow-hidden">
            <div
              className="h-full rounded-[2px] bg-[var(--accent-blue)] transition-colors"
              style={{ width: `${Math.min(100, Math.max(0, missionProgress * 100))}%` }}
            />
          </div>
        </div>
      </LinearCard>

      <div className="space-y-2">
        <div className="flex items-center justify-between px-1">
          <h4 className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
            Execution stages and timeline
          </h4>
          <span className="text-[12px] text-[var(--text-muted)]">
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

            const stepSurface = isRunning
              ? "bg-[var(--tint-blue)] border-[var(--accent-blue)]"
              : isQueued || isApproval
              ? "bg-[var(--tint-amber)] border-[var(--accent-amber)]"
              : isFailed
              ? "bg-[var(--tint-red)] border-[var(--accent-red)]"
              : isSkipped
              ? "opacity-75"
              : "";

            const indexTone = isCompleted
              ? "bg-[var(--tint-green)] text-[var(--accent-green)]"
              : isRunning
              ? "bg-[var(--tint-blue)] text-[var(--accent-blue)]"
              : isQueued
              ? "bg-[var(--tint-amber)] text-[var(--accent-amber)]"
              : "bg-[var(--bg-surface-2)] text-[var(--text-muted)]";

            return (
              <LinearCard
                key={step.stepIndex}
                level={1}
                className={`p-4 ${stepSurface}`}
              >
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <div
                      className={`flex size-7 shrink-0 items-center justify-center rounded-[6px] font-mono text-[12px] font-medium ${indexTone}`}
                    >
                      {step.stepIndex + 1}
                    </div>

                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[12px] font-medium px-2 py-0.5 rounded-[4px] bg-[var(--bg-surface-2)] text-[var(--text-muted)] border border-[var(--border)]">
                          {step.appId}
                        </span>
                        <span className="font-mono text-[12px] font-medium text-[var(--text)]">
                          {step.toolName}
                        </span>
                      </div>
                      <p className="text-[12px] text-[var(--text-muted)] mt-1">{step.description}</p>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`flex items-center gap-1 text-[12px] font-medium px-2 py-0.5 rounded-[4px] border ${
                        step.riskTier === 1
                          ? "bg-[var(--bg-surface-2)] text-[var(--text-muted)] border-[var(--border)]"
                          : step.riskTier === 2
                          ? "bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]"
                          : "bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--accent-red)]"
                      }`}
                    >
                      <Shield className="size-3.5" />
                      <span>Tier {step.riskTier}</span>
                    </span>

                    {step.retryCount && step.retryCount > 0 ? (
                      <span className="flex items-center gap-1 text-[12px] font-mono px-2 py-0.5 rounded-[4px] bg-[var(--tint-amber)] text-[var(--accent-amber)] border border-[var(--accent-amber)]">
                        <RotateCcw className="size-3.5" />
                        <span>Retry #{step.retryCount}</span>
                      </span>
                    ) : null}

                    {step.fallbackRung && (
                      <div className="flex items-center gap-1.5">
                        <span className="flex items-center gap-1 text-[12px] font-mono px-2 py-0.5 rounded-[4px] bg-[var(--bg-surface-2)] text-[var(--text-muted)] border border-[var(--border)]">
                          <FastForward className="size-3.5" />
                          <span>{step.fallbackRung}</span>
                        </span>
                        <button
                          type="button"
                          onClick={() => handleWhyFallback(step.fallbackEventId)}
                          className="flex items-center gap-1 text-[12px] text-[var(--accent-blue)] hover:underline cursor-pointer"
                          title={step.fallbackReason}
                        >
                          <HelpCircle className="size-3.5" />
                          <span>Why did this fallback happen?</span>
                        </button>
                      </div>
                    )}

                    <div className="flex items-center gap-1.5 pl-2 border-l border-[var(--border)]">
                      {isCompleted ? (
                        <span className="flex items-center gap-1 text-[12px] text-[var(--accent-green)] font-medium">
                          <CheckCircle2 className="size-3.5" />
                          <span>Completed</span>
                        </span>
                      ) : isQueued ? (
                        <span className="flex items-center gap-1.5 text-[12px] text-[var(--accent-amber)] font-medium">
                          <Clock className="size-3.5" />
                          <span>Waiting for resources</span>
                        </span>
                      ) : isRunning ? (
                        <span className="flex items-center gap-1.5 text-[12px] text-[var(--accent-blue)] font-medium">
                          <RefreshCw className="size-3.5" />
                          <span>Running</span>
                        </span>
                      ) : isSkipped ? (
                        <span className="flex items-center gap-1 text-[12px] text-[var(--text-muted)] font-medium">
                          <FastForward className="size-3.5" />
                          <span>Skipped</span>
                        </span>
                      ) : isFailed ? (
                        <span className="flex items-center gap-1 text-[12px] text-[var(--accent-red)] font-semibold">
                          <XCircle className="size-3.5" />
                          <span>Failed</span>
                        </span>
                      ) : (
                        <span className="text-[12px] text-[var(--text-muted)]">Pending</span>
                      )}
                    </div>
                  </div>
                </div>

                {isQueued && (
                  <div className="mt-3 pt-3 border-t border-[var(--border)] flex items-center gap-2 text-[12px] text-[var(--accent-amber)] font-mono">
                    <Clock className="size-3.5 shrink-0" />
                    <span>{step.message || "Waiting for resources"}</span>
                  </div>
                )}

                {isRunning && (
                  <div className="mt-3 pt-3 border-t border-[var(--border)]">
                    <div className="flex justify-between items-center text-[12px] mb-1">
                      <span className="text-[var(--text-muted)] font-mono">
                        {step.message || "Working"}
                      </span>
                      <span className="font-mono text-[var(--accent-blue)] font-medium tabular-nums">
                        {Math.round(step.progress * 100)}%
                      </span>
                    </div>
                    <div className="w-full h-1 rounded-[2px] bg-[var(--bg-bar-idle)] overflow-hidden">
                      <div
                        className="h-full rounded-[2px] bg-[var(--accent-blue)] transition-colors"
                        style={{ width: `${Math.round(step.progress * 100)}%` }}
                      />
                    </div>
                  </div>
                )}
              </LinearCard>
            );
          })}
        </div>
      </div>

      <LinearCard level={1} className="overflow-hidden">
        <button
          type="button"
          onClick={() => setIsLogDrawerOpen((prev) => !prev)}
          className="w-full flex items-center justify-between p-4 text-[12px] font-medium text-[var(--text-muted)] hover:text-[var(--text)] transition-colors cursor-pointer bg-[var(--bg-surface)]"
        >
          <div className="flex items-center gap-2">
            <Terminal className="size-4" />
            <span className="font-semibold text-[var(--text)]">Mission timeline event log</span>
            <span className="text-[12px] font-mono px-2 py-0.5 rounded-[4px] bg-[var(--bg-surface-2)] text-[var(--text-muted)] border border-[var(--border)]">
              {events.length > 0 ? `${events.length} events` : "Live telemetry"}
            </span>
          </div>
          {isLogDrawerOpen ? <ChevronUp className="size-4" /> : <ChevronDown className="size-4" />}
        </button>

        {isLogDrawerOpen && (
          <div className="p-4 border-t border-[var(--border)] bg-[var(--bg-surface-2)] max-h-64 overflow-y-auto font-mono text-[12px] space-y-2">
            {events.length === 0 ? (
              <div className="text-[var(--text-muted)] py-2 text-center">
                Listening for real-time mission timeline events...
              </div>
            ) : (
              events.map((ev) => {
                const isSelected = selectedEventId === ev.id;
                return (
                  <div
                    key={ev.id}
                    className={`p-2.5 rounded-[6px] border transition-colors ${
                      isSelected
                        ? "border-[var(--accent-amber)] bg-[var(--tint-amber)] text-[var(--text)]"
                        : "border-[var(--border)] bg-[var(--bg-surface)] text-[var(--text-muted)]"
                    }`}
                  >
                    <div className="flex items-center justify-between text-[12px] text-[var(--text-muted)]">
                      <span className="text-[var(--accent-blue)] font-semibold">{ev.kind}</span>
                      <span>#{ev.id} · {new Date(ev.tsMs).toLocaleTimeString()}</span>
                    </div>
                    <div className="text-[var(--text)] mt-1 break-all">
                      {JSON.stringify(ev.payload)}
                    </div>
                  </div>
                );
              })
            )}

          </div>
        )}
      </LinearCard>
    </div>
  );
};
