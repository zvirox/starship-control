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
  Sparkles,
  Loader2,
  Cpu,
  KeyRound,
  Minus,
  CornerDownLeft,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type {
  ApprovalNeededEvent,
  ApproveStepResponse,
  RejectStepResponse,
  ModelRecord,
  HelloResponse,
  TokenStreamEvent,
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

  // Listen to real-time events: evt://plan/approval_needed and evt://token/stream
  useEffect(() => {
    let unlistenApproval: (() => void) | undefined;
    let unlistenTokens: (() => void) | undefined;

    async function setupListeners() {
      unlistenApproval = await listen<ApprovalNeededEvent>(
        "evt://plan/approval_needed",
        async (event) => {
          setApprovalGate(event.payload);
          // Automatically trigger spring-physics expansion from 76px to 240px
          await setHudExpandedState(true);
        }
      );

      unlistenTokens = await listen<TokenStreamEvent>("evt://token/stream", (event) => {
        setIsStreaming(true);
        setStreamedText((prev) => prev + event.payload.chunk);
      });
    }

    setupListeners().catch(console.warn);

    return () => {
      if (unlistenApproval) unlistenApproval();
      if (unlistenTokens) unlistenTokens();
    };
  }, [setHudExpandedState]);

  // Handle Approve Action directly from HUD
  const handleApprove = async () => {
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
  const handleReject = async () => {
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

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center bg-transparent p-2 font-sans select-none overflow-hidden">
      <GlassPanel
        recipe="float"
        glow={approvalGate ? "rose" : "cyan"}
        className={`w-full max-w-[504px] transition-all duration-300 ${
          approvalGate ? "border-rose-500/80 shadow-[0_0_32px_rgba(244,63,94,0.35)]" : ""
        } ${
          expanded
            ? "h-[224px] p-4 flex flex-col justify-between"
            : "h-[64px] px-3 flex items-center justify-between"
        }`}
      >
        {/* Top Capsule Bar / Native Wayland Drag Region */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-between gap-2 w-full cursor-grab active:cursor-grabbing"
        >
          {/* Brand Emblem & Model Selection Dropdown */}
          <div className="relative flex items-center gap-2">
            <div
              className={`flex h-7 w-7 items-center justify-center rounded-full bg-orange-500/10 border border-orange-400/30 overflow-hidden shadow-[0_0_12px_rgba(249,115,22,0.25)] ${
                isStreaming ? "animate-pulse" : ""
              }`}
            >
              <img src="/logo.png" alt="Starship Control" className="h-5 w-5 object-contain" />
            </div>

            {/* Model Chip Button */}
            <button
              type="button"
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="flex items-center gap-1.5 rounded-full bg-white/5 hover:bg-white/10 px-2.5 py-0.5 border border-white/10 text-[11px] font-mono text-slate-300 transition-colors"
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  activeModel?.provider_type === "local_gguf" ? "bg-purple-400" : "bg-cyan-400"
                } animate-pulse`}
              />
              <span className="font-semibold text-slate-200">
                {activeModel ? activeModel.name.split(" ")[0] : "Qwen2.5"}
              </span>
              <ChevronDown className="h-2.5 w-2.5 text-slate-400" />
            </button>

            {/* Model Dropdown Menu */}
            {modelDropdownOpen && (
              <div className="absolute left-0 top-8 w-52 rounded-xl border border-white/10 bg-[#0E131B]/95 backdrop-blur-2xl p-1 shadow-[0_8px_32px_rgba(0,0,0,0.6)] z-50 text-xs">
                {models.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => {
                      setActiveModel(m);
                      setModelDropdownOpen(false);
                    }}
                    className={`w-full text-left px-2 py-1.5 rounded-lg flex items-center justify-between ${
                      activeModel?.id === m.id ? "bg-cyan-500/20 text-cyan-300" : "text-slate-300 hover:bg-white/5"
                    }`}
                  >
                    <span className="truncate">{m.name}</span>
                    <span className="text-[9px] font-mono text-slate-400">
                      {m.provider_type === "local_gguf" ? "Local" : "Cloud"}
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Quick Input Pill (Resting State) */}
          {!expanded && (
            <div className="flex-1 px-2">
              <input
                type="text"
                value={prompt}
                disabled={isStreaming}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder={
                  isStreaming
                    ? "Executing instruction..."
                    : "Ask Starship or give instruction..."
                }
                className="w-full bg-black/30 border border-white/10 rounded-full px-3 py-1 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400 transition-colors disabled:opacity-50"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && prompt.trim()) {
                    handleSendPrompt();
                  }
                }}
              />
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex items-center gap-1.5">
            {/* Microphone Toggle (ALP Audio Labs Voice Input) */}
            <button
              type="button"
              title={micActive ? "Mute Microphone" : "Voice Transcription (ALP)"}
              onClick={() => setMicActive(!micActive)}
              className={`flex h-7 w-7 items-center justify-center rounded-full transition-colors ${
                micActive
                  ? "bg-rose-500/20 text-rose-300 border border-rose-500/40 animate-pulse"
                  : "bg-white/5 hover:bg-white/10 text-slate-300"
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
                className="flex h-7 w-7 items-center justify-center rounded-full bg-cyan-400 hover:bg-cyan-300 text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.4)] transition-transform active:scale-95 disabled:opacity-40"
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
              title={expanded ? "Collapse HUD (520×76)" : "Expand HUD (520×240)"}
              onClick={toggleExpand}
              className="flex h-7 w-7 items-center justify-center rounded-full bg-white/5 hover:bg-white/10 text-slate-400 hover:text-slate-200 transition-colors"
            >
              {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronUp className="h-3.5 w-3.5" />}
            </button>

            {/* Hide HUD */}
            <button
              type="button"
              title="Hide HUD (Super+Shift+Space to restore)"
              onClick={() => invoke("cmd_toggle_hud")}
              className="flex h-7 w-7 items-center justify-center rounded-full bg-white/5 hover:bg-rose-500/20 hover:text-rose-300 text-slate-500 transition-colors"
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
              /* Idle Expanded Input View */
              <div className="space-y-2">
                <input
                  type="text"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="Enter strategic command (e.g. 'Ripple cut silence in EEP')..."
                  className="w-full rounded-xl bg-black/40 border border-white/10 px-3 py-2 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && prompt.trim()) {
                      handleSendPrompt();
                    }
                  }}
                />
                <div className="flex items-center justify-between text-[11px] font-mono text-slate-500">
                  <span className="flex items-center gap-1">
                    <Sparkles className="h-3 w-3 text-cyan-400" />
                    <span>Always-on-Top Linux Companion Capsule</span>
                  </span>
                  <span>Super+Shift+Space</span>
                </div>
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
                      className="flex items-center gap-1 px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 hover:bg-rose-500/20 hover:text-rose-300 text-xs text-slate-300 transition-colors"
                    >
                      <X className="h-3 w-3 text-rose-400" />
                      <span>Reject</span>
                    </button>
                    <button
                      type="button"
                      onClick={handleApprove}
                      disabled={isSubmitting}
                      className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-cyan-400 hover:bg-cyan-300 text-slate-950 font-bold text-xs shadow-[0_0_16px_rgba(56,189,248,0.4)] transition-transform active:scale-95"
                    >
                      {isSubmitting ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <Check className="h-3 w-3 stroke-[3]" />
                      )}
                      <span>Approve & Apply</span>
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
                    className="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-cyan-400 hover:bg-cyan-300 text-slate-950 font-bold text-xs shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-transform active:scale-95"
                  >
                    <Send className="h-3 w-3" />
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
