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
    }

    setupListeners().catch(console.warn);

    return () => {
      if (unlistenApproval) unlistenApproval();
      if (unlistenTokens) unlistenTokens();
      if (unlistenProgress) unlistenProgress();
      if (unlistenTray) unlistenTray();
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
    { id: "idle", label: "Idle", dotColor: "bg-slate-400" },
    { id: "thinking", label: "Thinking", dotColor: "bg-sky-400" },
    { id: "executing", label: "Executing", dotColor: "bg-emerald-400" },
    { id: "waiting_approval", label: "Approval", dotColor: "bg-amber-400" },
    { id: "stalled", label: "Stalled", dotColor: "bg-yellow-400" },
    { id: "overloaded", label: "Overloaded", dotColor: "bg-orange-400" },
    { id: "failed", label: "Failed", dotColor: "bg-rose-400" },
  ];

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center bg-transparent p-2 font-sans select-none overflow-hidden">
      <GlassPanel
        recipe="float"
        className={`w-full max-w-[504px] transition-all duration-300 ${
          approvalGate ? "border-rose-500/50" : "border-white/[0.08]"
        } ${
          expanded
            ? "h-[224px] p-3.5 flex flex-col justify-between"
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
            <div
              className={`flex h-7 w-7 items-center justify-center rounded-xl bg-white/[0.05] border border-white/[0.08] overflow-hidden ${
                isStreaming ? "animate-pulse" : ""
              }`}
            >
              <img src="/logo.png" alt="Starship Control" className="h-5 w-5 object-contain" />
            </div>

            {/* Model Chip Button */}
            <button
              type="button"
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="flex items-center gap-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] px-2 py-1 border border-white/[0.06] text-xs text-slate-300 transition-colors cursor-pointer"
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  activeModel?.provider_type === "local_gguf" ? "bg-purple-400" : "bg-sky-400"
                }`}
              />
              <span className="font-medium text-slate-200">
                {activeModel ? activeModel.name.split(" ")[0] : "Qwen2.5"}
              </span>
              <ChevronDown className="h-2.5 w-2.5 text-slate-400" />
            </button>

            {/* Model Dropdown Menu */}
            {modelDropdownOpen && (
              <div className="absolute left-0 top-9 w-52 rounded-xl border border-white/[0.1] bg-[#14171E] p-1 shadow-2xl z-50 text-xs">
                {models.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      setActiveModel(m);
                      setModelDropdownOpen(false);
                    }}
                    className={`w-full text-left px-2 py-1.5 rounded-lg flex items-center justify-between cursor-pointer ${
                      activeModel?.id === m.id ? "bg-sky-500/15 text-sky-300 font-medium" : "text-slate-300 hover:bg-white/[0.05]"
                    }`}
                  >
                    <span className="truncate">{m.name}</span>
                    <span className="text-[9px] text-slate-500 capitalize">
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
              className="flex items-center gap-1.5 px-2 py-1 rounded-lg bg-white/[0.03] hover:bg-white/[0.06] border border-white/[0.06] cursor-pointer transition-colors max-w-[150px] shrink-0"
              title={`Mission Progress: ${Math.round(missionProgress * 100)}% — ${activeStepMessage}`}
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
                    className={`fill-none transition-all duration-300 ${
                      isStalled
                        ? "stroke-amber-400"
                        : trayState === "failed"
                        ? "stroke-rose-400"
                        : "stroke-sky-400"
                    }`}
                    strokeWidth="2.5"
                    strokeDasharray={2 * Math.PI * 8.5}
                    strokeDashoffset={2 * Math.PI * 8.5 * (1 - missionProgress)}
                    strokeLinecap="round"
                  />
                </svg>
                <span className="absolute text-[7px] font-mono font-medium text-slate-300">
                  {Math.round(missionProgress * 100)}
                </span>
              </div>
              <div className="flex flex-col overflow-hidden text-left min-w-0">
                <span className="text-[9px] font-medium text-slate-200 truncate">
                  Step {activeStepIndex ?? 1}
                </span>
                <span className="text-[8px] text-slate-400 truncate">
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
                placeholder={
                  isStreaming
                    ? "Processing..."
                    : "Ask or instruct..."
                }
                className="w-full bg-black/20 border border-white/[0.08] rounded-xl px-2.5 py-1 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-white/[0.2] transition-colors disabled:opacity-50"
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
              title={micActive ? "Mute Microphone" : "Voice Transcription"}
              onClick={() => setMicActive(!micActive)}
              className={`flex h-7 w-7 items-center justify-center rounded-lg transition-colors cursor-pointer ${
                micActive
                  ? "bg-rose-500/20 text-rose-300 border border-rose-500/40"
                  : "bg-white/[0.04] hover:bg-white/[0.08] text-slate-300"
              }`}
            >
              <Mic className="h-3.5 w-3.5" />
            </button>

            {/* Submit / Trigger Button */}
            {!expanded && (
              <button
                type="button"
                title="Send Prompt"
                disabled={isStreaming || !prompt.trim()}
                onClick={handleSendPrompt}
                className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.08] hover:bg-white/[0.15] text-slate-100 transition-colors disabled:opacity-40 cursor-pointer"
              >
                {isStreaming ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin text-sky-400" />
                ) : (
                  <Send className="h-3.5 w-3.5 text-sky-400" />
                )}
              </button>
            )}

            {/* Expand / Collapse Chevron */}
            <button
              type="button"
              title={expanded ? "Collapse HUD" : "Expand HUD"}
              onClick={toggleExpand}
              className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-slate-400 hover:text-slate-200 transition-colors cursor-pointer"
            >
              {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronUp className="h-3.5 w-3.5" />}
            </button>

            {/* Hide HUD */}
            <button
              type="button"
              title="Hide HUD"
              onClick={() => invoke("cmd_toggle_hud")}
              className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-slate-500 hover:text-slate-300 transition-colors cursor-pointer"
            >
              <Minus className="h-3 w-3" />
            </button>
          </div>
        </div>

        {/* Expanded View: Streamed Output or Action Approval Card */}
        {expanded && (
          <div className="mt-3 flex-1 flex flex-col justify-between border-t border-white/10 pt-2.5 overflow-hidden animate-in fade-in duration-200">
            {approvalGate ? (
              /* Live Approval Gate Card */
              <div className="rounded-xl border border-rose-500/40 bg-rose-500/10 p-3 space-y-2 shadow-[0_0_20px_rgba(244,63,94,0.2)]">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-rose-300 font-semibold text-xs">
                    <ShieldAlert className="h-4 w-4 text-rose-400 animate-pulse" />
                    <span>HUMAN APPROVAL REQUIRED — {appDisplayName}</span>
                  </div>
                  <span className="font-mono text-[10px] font-bold bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-500/40 uppercase">
                    Tier {approvalGate.riskTier}
                  </span>
                </div>

                <p className="text-xs text-slate-200 leading-relaxed font-sans">
                  {approvalGate.diffSummary || approvalGate.actionTitle}
                </p>

                <div className="text-[11px] font-mono text-slate-400 flex items-center justify-between pt-1 border-t border-rose-500/20">
                  <span className="flex items-center gap-1 text-emerald-300">
                    <KeyRound className="h-3 w-3" />
                    <span>Signature: {approvalGate.signatureToken.slice(0, 14)}...</span>
                  </span>
                  <span>Checkpoint: <strong className="text-slate-200">#bk-04 (Auto-save)</strong></span>
                </div>
              </div>
            ) : streamedText || isStreaming ? (
              /* Active Stream Output View */
              <div className="flex-1 overflow-y-auto rounded-xl bg-black/40 border border-white/5 p-2.5 font-sans text-xs text-slate-200 leading-relaxed">
                <div className="flex items-center gap-1.5 text-cyan-400 font-mono text-[10px] uppercase mb-1">
                  <Cpu className="h-3 w-3 animate-spin" />
                  <span>Agent Stream Output</span>
                </div>
                <div className="whitespace-pre-wrap">{streamedText || "Synthesizing plan..."}</div>
              </div>
            ) : (
              /* Idle Expanded Input View with Progress & Tray State Bar */
              <div className="space-y-2">
                {/* Mission Progress & Step Status Bar */}
                <div className="flex items-center justify-between rounded-xl bg-white/[0.03] border border-white/[0.06] p-2 text-xs">
                  <div className="flex items-center gap-2.5 min-w-0">
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
                          className={`fill-none transition-all duration-300 ${
                            isStalled
                              ? "stroke-amber-400"
                              : trayState === "failed"
                              ? "stroke-rose-400"
                              : "stroke-sky-400"
                          }`}
                          strokeWidth="2.5"
                          strokeDasharray={2 * Math.PI * 11}
                          strokeDashoffset={2 * Math.PI * 11 * (1 - missionProgress)}
                          strokeLinecap="round"
                        />
                      </svg>
                      <span className="absolute text-[8px] font-mono font-semibold text-slate-200">
                        {Math.round(missionProgress * 100)}%
                      </span>
                    </div>

                    <div className="flex flex-col min-w-0">
                      <div className="flex items-center gap-1.5">
                        <span className="text-[11px] font-medium text-slate-200 truncate">
                          Step {activeStepIndex ?? 1}: {activeStepMessage}
                        </span>
                        {isStalled && (
                          <span className="flex items-center gap-0.5 text-[9px] font-medium text-amber-300 bg-amber-500/15 px-1 py-0.5 rounded border border-amber-500/30 shrink-0">
                            <AlertTriangle className="h-2.5 w-2.5 text-amber-400" />
                            Stalled
                          </span>
                        )}
                      </div>
                      <span className="text-[10px] font-mono text-slate-400 truncate">
                        {activeStepTool}
                      </span>
                    </div>
                  </div>

                  <span className="text-[10px] font-mono text-slate-400 shrink-0 pl-2">
                    {Math.round(missionProgress * 100)}% Complete
                  </span>
                </div>

                {/* Tray State Selector (GAP-CLOSURE-PLAN.md §13.6) */}
                <div className="flex items-center gap-1 overflow-x-auto pb-0.5">
                  <span className="text-[9px] font-mono text-slate-400 shrink-0 mr-1">Tray:</span>
                  {trayStates.map((ts) => {
                    const isActive = trayState === ts.id;
                    return (
                      <button
                        key={ts.id}
                        type="button"
                        onClick={() => updateTrayState(ts.id)}
                        className={`flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] border transition-colors cursor-pointer shrink-0 ${
                          isActive
                            ? "bg-sky-500/20 text-sky-200 border-sky-400/40 font-medium"
                            : "bg-white/[0.02] hover:bg-white/[0.06] text-slate-400 border-white/[0.06]"
                        }`}
                      >
                        <span className={`h-1.5 w-1.5 rounded-full ${ts.dotColor}`} />
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
                  placeholder="Enter strategic command (e.g. 'Ripple cut silence in EEP')..."
                  className="w-full rounded-xl bg-black/40 border border-white/10 px-3 py-1.5 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400"
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
            <div className="flex items-center justify-between pt-2 border-t border-white/10">
              <div className="flex items-center gap-2 text-[10px] font-mono text-slate-400">
                {approvalGate ? (
                  <>
                    <kbd className="flex items-center gap-1 rounded bg-slate-800 px-1 py-0.5 text-slate-300 border border-slate-700">
                      Enter
                      <CornerDownLeft className="h-2 w-2" />
                    </kbd>
                    <span>Approve</span>
                    <kbd className="rounded bg-slate-800 px-1 py-0.5 text-slate-300 border border-slate-700">
                      Esc
                    </kbd>
                    <span>Reject</span>
                  </>
                ) : (
                  <span>Ready · Esc to collapse</span>
                )}
              </div>

              <div className="flex items-center gap-2">
                {approvalGate ? (
                  <>
                    <button
                      type="button"
                      onClick={handleReject}
                      disabled={isSubmitting}
                      className="flex items-center gap-1 px-3 py-1.5 rounded-lg border border-white/[0.08] bg-white/[0.04] hover:bg-rose-500/10 hover:text-rose-300 text-xs text-slate-300 transition-colors cursor-pointer"
                    >
                      <X className="h-3 w-3 text-rose-400" />
                      <span>Reject</span>
                    </button>
                    <button
                      type="button"
                      onClick={handleApprove}
                      disabled={isSubmitting}
                      className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-sky-500/20 hover:bg-sky-500/30 text-sky-200 border border-sky-400/30 text-xs font-medium transition-colors cursor-pointer"
                    >
                      {isSubmitting ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <Check className="h-3 w-3 stroke-[2.5]" />
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
                    className="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-white/[0.08] hover:bg-white/[0.15] text-slate-100 text-xs font-medium transition-colors cursor-pointer"
                  >
                    <Send className="h-3 w-3 text-sky-400" />
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
