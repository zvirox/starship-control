import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Mic,
  Send,
  ChevronUp,
  ChevronDown,
  ShieldAlert,
  Check,
  X,
  Loader2,
  Cpu,
  KeyRound,
  Minus,
  CornerDownLeft,
  AlertTriangle,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type {
  ApprovalNeededEvent,
  ApproveStepResponse,
  RejectStepResponse,
  ModelRecord,
  HelloResponse,
  TokenStreamEvent,
  TrayState,
  MissionProgressPayload,
  HealthState,
  HealthStateChangedEvent,
  HealthSnapshotResponse,
} from "../../types/ipc";

const APP_NAMES: Record<string, string> = {
  eep: "Easy Edits Pro",
  ssp: "ScreenStudio Pro",
  alp: "Audio Labs Pro",
  nwp: "Net Watch Pro",
  atp: "Agentic Tuneup Pro",
};

export const HudView: React.FC = () => {
  const [expanded, setExpanded] = useState<boolean>(false);
  const [prompt, setPrompt] = useState<string>("");
  const [isStreaming, setIsStreaming] = useState<boolean>(false);
  const [streamedText, setStreamedText] = useState<string>("");
  const [models, setModels] = useState<ModelRecord[]>([]);
  const [activeModel, setActiveModel] = useState<ModelRecord | null>(null);
  const [modelDropdownOpen, setModelDropdownOpen] = useState<boolean>(false);
  const [approvalGate, setApprovalGate] = useState<ApprovalNeededEvent | null>(null);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [micActive, setMicActive] = useState<boolean>(false);

  // Real-time mission progress & tray state (GAP-CLOSURE-PLAN.md §13.6)
  const [missionProgress, setMissionProgress] = useState<number>(0.65);
  const [activeStepIndex, setActiveStepIndex] = useState<number | null>(3);
  const [activeStepTool] = useState<string>("alp.denoise.spectral");
  const [activeStepMessage, setActiveStepMessage] = useState<string>("Applying spectral noise subtraction");
  const [trayState, setTrayState] = useState<TrayState>("executing");
  const [isStalled, setIsStalled] = useState<boolean>(false);

  // Health & governor state (GAP-CLOSURE-PLAN.md §14.10)
  const [healthState, setHealthState] = useState<HealthState>("normal");
  const [governorPaused, setGovernorPaused] = useState<boolean>(false);
  const [governorMessage, setGovernorMessage] = useState<string>("");

  // Initialize models and system state
  useEffect(() => {
    async function initHud() {
      try {
        const hello = await invoke<HelloResponse>("ipc_hello");
        setModels(hello.available_models);
        if (hello.available_models.length > 0) {
          const def =
            hello.available_models.find((m) => m.is_default) || hello.available_models[0];
          setActiveModel(def);
        }

        const snap = await invoke<HealthSnapshotResponse>("cmd_health_get_snapshot");
        setHealthState(snap.state);
        if (snap.governorDirective) {
          setGovernorMessage(snap.governorDirective.actionMessage);
          if (snap.governorDirective.shouldPause || snap.state === "critical") {
            setGovernorPaused(true);
            setTrayState("overloaded");
          }
        }
      } catch (err) {
        console.warn("HUD init handshake failed:", err);
      }
    }
    initHud();
  }, []);

  // Window resize wrapper
  const setHudExpandedState = useCallback(async (nextState: boolean) => {
    setExpanded(nextState);
    try {
      await invoke("cmd_resize_hud", { expanded: nextState });
    } catch (e) {
      console.warn("Resize IPC failed (browser fallback):", e);
    }
  }, []);

  const toggleExpand = () => {
    setHudExpandedState(!expanded);
  };

  // Helper to change tray state and broadcast to Tauri backend
  const updateTrayState = useCallback(async (newState: TrayState) => {
    setTrayState(newState);
    try {
      await invoke("cmd_tray_set_state", { payload: { state: newState } });
    } catch (err) {
      console.warn("Failed to set tray state:", err);
    }
  }, []);

  // Listen to real-time events: evt://plan/approval_needed, evt://token/stream, evt://mission/progress
  useEffect(() => {
    let unlistenApproval: (() => void) | undefined;
    let unlistenTokens: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    let unlistenTray: (() => void) | undefined;
    let unlistenHealth: (() => void) | undefined;

    async function setupListeners() {
      unlistenApproval = await listen<ApprovalNeededEvent>(
        "evt://plan/approval_needed",
        async (event) => {
          setApprovalGate(event.payload);
          setTrayState("waiting_approval");
          // Automatically trigger spring-physics expansion from 76px to 240px
          await setHudExpandedState(true);
        }
      );

      unlistenTokens = await listen<TokenStreamEvent>("evt://token/stream", (event) => {
        setIsStreaming(true);
        setTrayState("thinking");
        setStreamedText((prev) => prev + event.payload.chunk);
      });

      unlistenProgress = await listen<MissionProgressPayload>(
        "evt://mission/progress",
        (event) => {
          setMissionProgress(event.payload.missionProgress);
          setIsStalled(event.payload.isStalled);
          if (event.payload.activeStepIndex !== undefined) {
            setActiveStepIndex(event.payload.activeStepIndex);
          }
          if (event.payload.stepMessage) {
            setActiveStepMessage(event.payload.stepMessage);
          }
          if (event.payload.isStalled) {
            setTrayState("stalled");
          } else if (event.payload.missionProgress >= 1.0) {
            setTrayState("idle");
          } else {
            setTrayState("executing");
          }
        }
      );

      unlistenTray = await listen<{ state: TrayState }>(
        "evt://tray/state_changed",
        (event) => {
          setTrayState(event.payload.state);
        }
      );

      unlistenHealth = await listen<HealthStateChangedEvent>(
        "evt://health/state_changed",
        async (event) => {
          const currentState = event.payload.currentState;
          setHealthState(currentState);
          if (event.payload.directive) {
            setGovernorMessage(event.payload.directive.actionMessage);
            if (event.payload.directive.shouldPause || currentState === "critical") {
              setGovernorPaused(true);
              setTrayState("overloaded");
              // Expand HUD with high-visibility banner within 2s of Critical injection
              await setHudExpandedState(true);
            } else if (
              event.payload.directive.shouldResume ||
              currentState === "normal" ||
              currentState === "elevated"
            ) {
              setGovernorPaused(false);
            }
          }
        }
      );
    }

    setupListeners().catch(console.warn);

    return () => {
      if (unlistenApproval) unlistenApproval();
      if (unlistenTokens) unlistenTokens();
      if (unlistenProgress) unlistenProgress();
      if (unlistenTray) unlistenTray();
      if (unlistenHealth) unlistenHealth();
    };
  }, [setHudExpandedState]);

  // Handle Approve Action directly from HUD
  const handleApprove = async (e?: React.MouseEvent | React.KeyboardEvent | KeyboardEvent) => {
    // GAP-CLOSURE-PLAN.md §10.4: Frontend approval handler requires event.isTrusted
    if (e && !e.isTrusted) {
      console.warn("Security warning: synthetic untrusted event rejected for HUD action approval");
      return;
    }
    if (!approvalGate || isSubmitting) return;
    try {
      setIsSubmitting(true);
      await invoke<ApproveStepResponse>("cmd_plan_approve_step", {
        payload: {
          missionId: approvalGate.missionId,
          stepId: approvalGate.stepId,
          signatureToken: approvalGate.signatureToken,
        },
      });
      setApprovalGate(null);
      // Collapse HUD smoothly back to resting state
      await setHudExpandedState(false);
    } catch (err) {
      console.error("HUD approval execution failed:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle Reject Action directly from HUD
  const handleReject = async (e?: React.MouseEvent | React.KeyboardEvent | KeyboardEvent) => {
    if (e && !e.isTrusted) {
      console.warn("Security warning: synthetic untrusted event rejected for HUD action rejection");
      return;
    }
    if (!approvalGate || isSubmitting) return;
    try {
      setIsSubmitting(true);
      await invoke<RejectStepResponse>("cmd_plan_reject_step", {
        payload: {
          missionId: approvalGate.missionId,
          stepId: approvalGate.stepId,
          reason: "User declined modification in Floating HUD",
        },
      });
      setApprovalGate(null);
      // Collapse HUD smoothly back to resting state
      await setHudExpandedState(false);
    } catch (err) {
      console.error("HUD rejection failed:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  // Handle Prompt submission from HUD
  const handleSendPrompt = async () => {
    if (!prompt.trim() || isStreaming) return;
    const userPrompt = prompt.trim();
    setPrompt("");
    setIsStreaming(true);
    setStreamedText("");

    try {
      // Dispatch goal decomposition
      await invoke("cmd_plan_generate", {
        payload: {
          missionId: 1,
          userGoal: userPrompt,
        },
      });

      // Stream agent response
      await invoke("cmd_mission_send_prompt", {
        payload: {
          missionId: 1,
          prompt: userPrompt,
        },
      });
    } catch (err) {
      console.error("HUD prompt dispatch error:", err);
    } finally {
      setIsStreaming(false);
    }
  };

  // Keyboard shortcut listeners (Enter to approve, Esc to reject or collapse)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (approvalGate) {
        if (e.key === "Enter") {
          e.preventDefault();
          handleApprove();
        } else if (e.key === "Escape") {
          e.preventDefault();
          handleReject();
        }
      } else if (e.key === "Escape" && expanded) {
        e.preventDefault();
        setHudExpandedState(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  const appDisplayName = approvalGate
    ? APP_NAMES[approvalGate.appId.toLowerCase()] || approvalGate.appId.toUpperCase()
    : "Easy Edits Pro";

  const trayStates: { id: TrayState; label: string; dotColor: string }[] = [
    { id: "idle", label: "Idle", dotColor: "bg-text-disabled" },
    { id: "thinking", label: "Thinking", dotColor: "bg-signal-thinking" },
    { id: "executing", label: "Executing", dotColor: "bg-signal-success" },
    { id: "waiting_approval", label: "Approval", dotColor: "bg-signal-gate" },
    { id: "stalled", label: "Stalled", dotColor: "bg-signal-thinking" },
    { id: "overloaded", label: "Overloaded", dotColor: "bg-signal-thinking" },
    { id: "failed", label: "Failed", dotColor: "bg-signal-gate" },
  ];

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center bg-transparent p-2 font-sans select-none overflow-hidden">
      <GlassPanel
        recipe="float"
        className={`w-full max-w-[504px] transition-colors duration-150 ${
          approvalGate ? "border-signal-gate/40" : "border-border-subtle"
        } ${
          expanded
            ? "h-[240px] p-3 flex flex-col justify-between"
            : "h-[64px] px-3 flex items-center justify-between"
        }`}
      >
        {/* Top Capsule Bar / Native Wayland Drag Region */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-between gap-2 w-full cursor-grab active:cursor-grabbing"
        >
          {/* Brand Emblem & Model Selection Dropdown */}
          <div className="relative flex items-center gap-2 shrink-0">
            <div className="flex h-7 w-7 items-center justify-center rounded-xl bg-white/[0.05] border border-border-subtle overflow-hidden">
              <img src="/logo.png" alt="Starship Control" className="h-5 w-5 object-contain" />
            </div>

            {/* Health Indicator Dot (GAP-CLOSURE-PLAN.md §14.10) */}
            <div
              className="flex items-center justify-center p-0.5 cursor-pointer"
              title={`System health: ${healthState}${governorMessage ? ` — ${governorMessage}` : ""}`}
            >
              <span
                className={`h-2 w-2 rounded-full transition-colors duration-150 ${
                  healthState === "normal"
                    ? "bg-signal-success"
                    : healthState === "elevated"
                    ? "bg-signal-thinking"
                    : healthState === "constrained"
                    ? "bg-signal-thinking"
                    : "bg-signal-gate animate-pulse"
                }`}
              />
            </div>

            {/* Model Chip Button */}
            <button
              type="button"
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="flex items-center gap-1 rounded-lg bg-white/[0.03] hover:bg-white/[0.05] px-2 py-1 border border-border-subtle text-xs text-text-secondary transition-colors cursor-pointer"
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  activeModel?.provider_type === "local_gguf" ? "bg-signal-local-model" : "bg-accent"
                }`}
              />
              <span className="font-medium text-text-primary">
                {activeModel ? activeModel.name.split(" ")[0] : "Qwen2.5"}
              </span>
              <ChevronDown className="h-3.5 w-3.5 text-text-secondary" />
            </button>

            {/* Model Dropdown Menu */}
            {modelDropdownOpen && (
              <div className="absolute left-0 top-9 w-52 rounded-xl border border-border-subtle bg-raised p-1 z-50 text-xs">
                {models.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      setActiveModel(m);
                      setModelDropdownOpen(false);
                    }}
                    className={`w-full text-left px-2 py-1 rounded-lg flex items-center justify-between cursor-pointer transition-colors ${
                      activeModel?.id === m.id ? "bg-accent-muted text-accent font-medium" : "text-text-secondary hover:bg-white/[0.05]"
                    }`}
                  >
                    <span className="truncate">{m.name}</span>
                    <span className="text-[11px] text-text-secondary font-medium">
                      {m.provider_type === "local_gguf" ? "Local" : "Cloud"}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Compact Progress Ring & Active Step (Resting State) */}
          {!expanded && (
            <div
              onClick={toggleExpand}
              className="flex items-center gap-2 px-2 py-1 rounded-lg bg-white/[0.03] hover:bg-white/[0.05] border border-border-subtle cursor-pointer transition-colors max-w-[150px] shrink-0"
              title={`Mission progress: ${Math.round(missionProgress * 100)}% — ${activeStepMessage}`}
            >
              <div className="relative flex items-center justify-center w-5 h-5 shrink-0">
                <svg className="w-5 h-5 -rotate-90" viewBox="0 0 24 24">
                  <circle
                    cx="12"
                    cy="12"
                    r="8.5"
                    className="stroke-white/10 fill-none"
                    strokeWidth="2.5"
                  />
                  <circle
                    cx="12"
                    cy="12"
                    r="8.5"
                    className={`fill-none transition-colors duration-150 ${
                      isStalled
                        ? "stroke-signal-thinking"
                        : trayState === "failed"
                        ? "stroke-signal-gate"
                        : "stroke-accent"
                    }`}
                    strokeWidth="2.5"
                    strokeDasharray={2 * Math.PI * 8.5}
                    strokeDashoffset={2 * Math.PI * 8.5 * (1 - missionProgress)}
                    strokeLinecap="round"
                  />
                </svg>
                <span className="absolute text-[11px] font-mono font-medium text-text-primary tabular">
                  {Math.round(missionProgress * 100)}
                </span>
              </div>
              <div className="flex flex-col overflow-hidden text-left min-w-0">
                <span className="text-[11px] font-medium text-text-primary truncate">
                  Step {activeStepIndex ?? 1}
                </span>
                <span className="text-[11px] text-text-secondary truncate">
                  {isStalled ? "Stalled" : activeStepMessage}
                </span>
              </div>
            </div>
          )}

          {/* Quick Input Pill (Resting State) */}
          {!expanded && (
            <div className="flex-1 min-w-0 px-1">
              <input
                type="text"
                value={prompt}
                disabled={isStreaming}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder={isStreaming ? "Working..." : "Ask or instruct..."}
                className="w-full bg-surface-solid border border-border-subtle rounded-xl px-2.5 py-1 text-xs font-medium text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent transition-colors disabled:opacity-50"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && prompt.trim()) {
                    handleSendPrompt();
                  }
                }}
              />
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex items-center gap-1 shrink-0">
            {/* Microphone Toggle (ALP Audio Labs Voice Input) */}
            <button
              type="button"
              title={micActive ? "Mute microphone" : "Voice transcription"}
              onClick={() => setMicActive(!micActive)}
              className={`flex h-7 w-7 items-center justify-center rounded-lg transition-colors cursor-pointer ${
                micActive
                  ? "bg-accent-muted text-accent border border-accent/40"
                  : "bg-white/[0.03] hover:bg-white/[0.05] text-text-secondary"
              }`}
            >
              <Mic className="h-3.5 w-3.5" />
            </button>

            {/* Submit / Trigger Button */}
            {!expanded && (
              <button
                type="button"
                title="Send prompt"
                disabled={isStreaming || !prompt.trim()}
                onClick={handleSendPrompt}
                className="flex h-7 w-7 items-center justify-center rounded-lg bg-accent text-accent-text hover:bg-accent/90 text-xs transition-colors active:scale-[0.98] disabled:opacity-40 cursor-pointer"
              >
                {isStreaming ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Send className="h-3.5 w-3.5" />
                )}
              </button>
            )}

            {/* Expand / Collapse Chevron */}
            <button
              type="button"
              title={expanded ? "Collapse HUD" : "Expand HUD"}
              onClick={toggleExpand}
              className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.03] hover:bg-white/[0.05] text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
            >
              {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronUp className="h-3.5 w-3.5" />}
            </button>

            {/* Hide HUD */}
            <button
              type="button"
              title="Hide HUD"
              onClick={() => invoke("cmd_toggle_hud")}
              className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.03] hover:bg-white/[0.05] text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
            >
              <Minus className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        {/* Expanded View: Streamed Output or Action Approval Card */}
        {expanded && (
          <div className="mt-3 flex-1 flex flex-col justify-between border-t border-border-subtle pt-2 overflow-hidden transition-[opacity,transform] duration-200">
            {/* Governor Safe-Pause Banner (GAP-CLOSURE-PLAN.md §14.10) */}
            {governorPaused && (
              <div className="rounded-xl border border-signal-thinking/40 bg-signal-thinking-bg p-2 mb-2 flex items-center justify-between text-signal-thinking text-xs transition-[opacity,transform] duration-200">
                <div className="flex items-center gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0" />
                  <div>
                    <span className="font-medium text-text-primary text-[11px]">
                      Mission safe-paused by resource governor
                    </span>
                    <p className="text-[11px] text-text-secondary mt-1">
                      {governorMessage || "Host machine resources constrained or critical. Execution will auto-resume safely."}
                    </p>
                  </div>
                </div>
                <span className="font-mono text-[11px] text-text-secondary px-2 py-1 rounded-lg bg-white/[0.05] border border-border-subtle shrink-0">
                  Auto-resume
                </span>
              </div>
            )}

            {approvalGate ? (
              /* Live Approval Gate Card */
              <div className="rounded-xl border border-signal-gate/40 bg-signal-gate-bg p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-signal-gate font-medium text-xs">
                    <ShieldAlert className="h-4 w-4" />
                    <span>Approval required — {appDisplayName}</span>
                  </div>
                  <span className="font-mono text-[11px] text-signal-gate px-2 py-1 rounded-lg bg-white/[0.05] border border-signal-gate/40">
                    Tier {approvalGate.riskTier}
                  </span>
                </div>

                <p className="text-xs text-text-primary leading-relaxed font-sans">
                  {approvalGate.diffSummary || approvalGate.actionTitle}
                </p>

                <div className="text-[11px] font-mono text-text-secondary flex items-center justify-between pt-2 border-t border-signal-gate/30">
                  <span className="flex items-center gap-1 text-signal-success">
                    <KeyRound className="h-3.5 w-3.5" />
                    <span>Signed token: {approvalGate.signatureToken.slice(0, 14)}...</span>
                  </span>
                  <span>Checkpoint: <strong className="font-medium text-text-primary">#bk-04 (Auto-save)</strong></span>
                </div>
              </div>
            ) : streamedText || isStreaming ? (
              /* Active Stream Output View */
              <div className="flex-1 overflow-y-auto rounded-xl bg-black/40 border border-border-subtle p-2 font-sans text-xs text-text-primary leading-relaxed">
                <div className="flex items-center gap-2 text-text-secondary font-mono text-[11px] mb-1">
                  <Cpu className="h-3.5 w-3.5" />
                  <span>Agent stream output</span>
                  <span className="ml-1 inline-block h-3 w-0.5 animate-pulse bg-accent" aria-hidden="true" />
                </div>
                <div className="whitespace-pre-wrap">{streamedText || "Synthesizing plan"}</div>
              </div>
            ) : (
              /* Idle Expanded Input View with Progress & Tray State Bar */
              <div className="space-y-2">
                {/* Mission Progress & Step Status Bar */}
                <div className="flex items-center justify-between rounded-xl bg-white/[0.03] border border-border-subtle p-2 text-xs">
                  <div className="flex items-center gap-2 min-w-0">
                    <div className="relative flex items-center justify-center w-7 h-7 shrink-0">
                      <svg className="w-7 h-7 -rotate-90" viewBox="0 0 28 28">
                        <circle
                          cx="14"
                          cy="14"
                          r="11"
                          className="stroke-white/10 fill-none"
                          strokeWidth="2.5"
                        />
                        <circle
                          cx="14"
                          cy="14"
                          r="11"
                          className={`fill-none transition-colors duration-150 ${
                            isStalled
                              ? "stroke-signal-thinking"
                              : trayState === "failed"
                              ? "stroke-signal-gate"
                              : "stroke-accent"
                          }`}
                          strokeWidth="2.5"
                          strokeDasharray={2 * Math.PI * 11}
                          strokeDashoffset={2 * Math.PI * 11 * (1 - missionProgress)}
                          strokeLinecap="round"
                        />
                      </svg>
                      <span className="absolute text-[11px] font-mono font-medium text-text-primary tabular">
                        {Math.round(missionProgress * 100)}%
                      </span>
                    </div>

                    <div className="flex flex-col min-w-0">
                      <div className="flex items-center gap-1">
                        <span className="text-[11px] font-medium text-text-primary truncate">
                          Step {activeStepIndex ?? 1}: {activeStepMessage}
                        </span>
                        {isStalled && (
                          <span className="flex items-center gap-1 text-[11px] font-medium text-signal-thinking bg-signal-thinking-bg px-2 py-1 rounded-lg border border-signal-thinking/40 shrink-0">
                            <AlertTriangle className="h-3.5 w-3.5" />
                            Stalled
                          </span>
                        )}
                      </div>
                      <span className="text-[11px] font-mono text-text-secondary truncate">
                        {activeStepTool}
                      </span>
                    </div>
                  </div>

                  <span className="text-[11px] font-mono text-text-secondary shrink-0 pl-2 tabular">
                    {Math.round(missionProgress * 100)}% complete
                  </span>
                </div>

                {/* Tray State Selector (GAP-CLOSURE-PLAN.md §13.6) */}
                <div className="flex items-center gap-1 overflow-x-auto pb-1">
                  <span className="text-[11px] font-mono text-text-secondary shrink-0 mr-1">Tray:</span>
                  {trayStates.map((ts) => {
                    const isActive = trayState === ts.id;
                    return (
                      <button
                        key={ts.id}
                        type="button"
                        onClick={() => updateTrayState(ts.id)}
                        className={`flex items-center gap-1 px-2 py-1 rounded-lg text-[11px] border transition-colors cursor-pointer shrink-0 ${
                          isActive
                            ? "bg-accent-muted text-accent border-accent/40 font-medium"
                            : "bg-white/[0.03] hover:bg-white/[0.05] text-text-secondary border-border-subtle"
                        }`}
                      >
                        <span className={`h-2 w-2 rounded-full ${ts.dotColor}`} />
                        <span>{ts.label}</span>
                      </button>
                    );
                  })}
                </div>

                {/* Strategic Command Input */}
                <input
                  type="text"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="Enter a strategic command..."
                  className="w-full rounded-xl bg-surface-solid border border-border-subtle px-3 py-2 text-xs font-medium text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent transition-colors"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && prompt.trim()) {
                      handleSendPrompt();
                    }
                  }}
                />
              </div>
            )}

            {/* Bottom Approval / Action Controls */}
            <div className="flex items-center justify-between pt-2 border-t border-border-subtle">
              <div className="flex items-center gap-2 text-[11px] font-mono text-text-secondary">
                {approvalGate ? (
                  <>
                    <kbd className="keycap gap-1">
                      Enter
                      <CornerDownLeft className="h-3.5 w-3.5" />
                    </kbd>
                    <span className="text-text-secondary font-medium">Approve</span>
                    <kbd className="keycap">Esc</kbd>
                    <span className="text-text-secondary font-medium">Reject</span>
                  </>
                ) : (
                  <span className="font-medium">Ready · Esc to collapse</span>
                )}
              </div>

              <div className="flex items-center gap-2">
                {approvalGate ? (
                  <>
                    <button
                      type="button"
                      onClick={handleReject}
                      disabled={isSubmitting}
                      className="flex items-center gap-1 px-3 py-2 rounded-lg border border-border-subtle bg-raised hover:bg-white/[0.05] text-xs text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
                    >
                      <X className="h-3.5 w-3.5" />
                      <span>Reject</span>
                    </button>
                    <button
                      type="button"
                      onClick={handleApprove}
                      disabled={isSubmitting}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent text-accent-text hover:bg-accent/90 border border-accent text-xs font-medium transition-colors active:scale-[0.98] cursor-pointer"
                    >
                      {isSubmitting ? (
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <Check className="h-3.5 w-3.5" />
                      )}
                      <span>Approve</span>
                    </button>
                  </>
                ) : (
                  <button
                    type="button"
                    onClick={() => {
                      if (prompt.trim()) {
                        handleSendPrompt();
                      } else {
                        setHudExpandedState(false);
                      }
                    }}
                    className="flex items-center gap-1 px-3 py-2 rounded-lg bg-accent text-accent-text hover:bg-accent/90 text-xs font-medium transition-colors active:scale-[0.98] cursor-pointer"
                  >
                    <Send className="h-3.5 w-3.5" />
                    <span>Send</span>
                  </button>
                )}
              </div>
            </div>
          </div>
        )}
      </GlassPanel>
    </div>
  );
};
