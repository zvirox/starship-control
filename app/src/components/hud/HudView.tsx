import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sparkles, Mic, Send, ChevronUp, ChevronDown, ShieldAlert, Check, X } from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";

export const HudView: React.FC = () => {
  const [expanded, setExpanded] = useState(false);
  const [prompt, setPrompt] = useState("");
  const [hasGate, setHasGate] = useState(false);

  const toggleExpand = async () => {
    const next = !expanded;
    setExpanded(next);
    try {
      await invoke("cmd_resize_hud", { expanded: next });
    } catch (e) {
      console.warn("Resize IPC failed (browser preview fallback)", e);
    }
  };

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center bg-transparent p-2 font-sans select-none">
      <GlassPanel
        recipe="float"
        glow={hasGate ? "rose" : "cyan"}
        className={`w-full max-w-[504px] transition-all duration-300 ${
          expanded ? "h-[224px] p-4 flex flex-col justify-between" : "h-[64px] px-3 flex items-center justify-between"
        }`}
      >
        {/* Top Bar / Drag Region */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-between gap-2 w-full cursor-grab active:cursor-grabbing"
        >
          {/* Brand Glyph & Model Chip */}
          <div className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-full bg-cyan-500/20 text-cyan-400 border border-cyan-400/40 shadow-[0_0_12px_rgba(56,189,248,0.3)]">
              <Sparkles className="h-3.5 w-3.5 animate-pulse" />
            </div>
            <div className="flex items-center gap-1.5 rounded-full bg-white/5 px-2.5 py-0.5 border border-white/10 text-[11px] font-mono text-slate-300">
              <span className="h-1.5 w-1.5 rounded-full bg-purple-400" />
              <span>Qwen2.5-7B</span>
            </div>
          </div>

          {/* Quick Input (Resting state) */}
          {!expanded && (
            <div className="flex-1 px-2">
              <input
                type="text"
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                placeholder="Ask Starship or give instruction..."
                className="w-full bg-black/20 border border-white/10 rounded-full px-3 py-1 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400 transition-colors"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && prompt) {
                    toggleExpand();
                  }
                }}
              />
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex items-center gap-1.5">
            <button
              title="Voice Input (ALP)"
              className="flex h-7 w-7 items-center justify-center rounded-full bg-white/5 hover:bg-white/10 text-slate-300 transition-colors"
            >
              <Mic className="h-3.5 w-3.5" />
            </button>
            <button
              title="Execute"
              onClick={toggleExpand}
              className="flex h-7 w-7 items-center justify-center rounded-full bg-cyan-400 hover:bg-cyan-300 text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.4)] transition-transform active:scale-95"
            >
              <Send className="h-3.5 w-3.5" />
            </button>
            <button
              title={expanded ? "Collapse HUD" : "Expand HUD"}
              onClick={toggleExpand}
              className="flex h-7 w-7 items-center justify-center rounded-full bg-white/5 hover:bg-white/10 text-slate-400 hover:text-slate-200 transition-colors"
            >
              {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronUp className="h-3.5 w-3.5" />}
            </button>
          </div>
        </div>

        {/* Expanded Content View */}
        {expanded && (
          <div className="mt-3 flex-1 flex flex-col justify-between border-t border-white/10 pt-2.5">
            {/* Approval Gate or Active Step */}
            <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 space-y-1.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-rose-300 font-semibold text-xs">
                  <ShieldAlert className="h-4 w-4 text-rose-400" />
                  <span>HUMAN APPROVAL REQUIRED — Easy Edits Pro</span>
                </div>
                <span className="font-mono text-[10px] bg-rose-500/20 text-rose-300 px-1.5 py-0.5 rounded border border-rose-500/40">
                  TIER 2
                </span>
              </div>
              <p className="text-[11px] text-slate-300 leading-relaxed">
                Apply 18 ripple cuts and remove 01:24 dead time via CET telemetry.
              </p>
              <div className="text-[10px] font-mono text-slate-400 flex items-center gap-3">
                <span>Checkpoint: #bk-04</span>
                <span>•</span>
                <span>Impact: 18 edits, 0 audio lost</span>
              </div>
            </div>

            {/* Actions Bar */}
            <div className="flex items-center justify-between pt-2">
              <button
                onClick={() => setHasGate(!hasGate)}
                className="text-[10px] text-slate-400 hover:text-cyan-400 transition-colors"
              >
                Toggle Gate State Preview
              </button>
              <div className="flex items-center gap-2">
                <button
                  onClick={toggleExpand}
                  className="flex items-center gap-1.5 px-3 py-1 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 text-xs text-slate-300 hover:text-white transition-colors"
                >
                  <X className="h-3 w-3" />
                  <span>Reject</span>
                </button>
                <button
                  onClick={toggleExpand}
                  className="flex items-center gap-1.5 px-3 py-1 rounded-lg bg-cyan-400 hover:bg-cyan-300 text-slate-950 font-semibold text-xs shadow-[0_0_12px_rgba(56,189,248,0.3)] transition-transform active:scale-95"
                >
                  <Check className="h-3 w-3" />
                  <span>Approve & Apply</span>
                </button>
              </div>
            </div>
          </div>
        )}
      </GlassPanel>
    </div>
  );
};
