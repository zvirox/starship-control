import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Sparkles,
  Layers,
  Cpu,
  FileText,
  Activity,
  Plus,
  Send,
  CheckCircle2,
  AlertTriangle,
  Play,
  RotateCcw,
  Check,
  X,
  ExternalLink,
  Search,
  Database as DbIcon,
  ShieldCheck,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import { DesignVerification } from "../common/DesignVerification";
import type { HelloResponse, ModelRecord } from "../../types/ipc";

export const CockpitView: React.FC = () => {
  const [navTab, setNavTab] = useState<"missions" | "topology" | "models" | "audit" | "design">("missions");
  const [prompt, setPrompt] = useState("");
  const [hudActive, setHudActive] = useState(false);
  const [systemInfo, setSystemInfo] = useState<HelloResponse | null>(null);
  const [models, setModels] = useState<ModelRecord[]>([]);

  useEffect(() => {
    async function initHandshake() {
      try {
        const hello = await invoke<HelloResponse>("ipc_hello");
        setSystemInfo(hello);
        setModels(hello.available_models);
      } catch (err) {
        console.warn("ipc_hello call in browser mode:", err);
      }
    }
    initHandshake();
  }, []);

  const handleToggleHud = async () => {
    try {
      const active = await invoke<boolean>("cmd_toggle_hud");
      setHudActive(active);
    } catch (err) {
      console.warn("cmd_toggle_hud call failed:", err);
      setHudActive(!hudActive);
    }
  };

  return (
    <div className="flex h-screen w-screen flex-col bg-[#080B10] text-[#F1F5F9] font-sans overflow-hidden">
      {/* 1. Custom Titlebar (52px) */}
      <header
        data-tauri-drag-region
        className="flex h-[52px] items-center justify-between border-b border-white/10 px-4 bg-[#0E131B]/80 backdrop-blur-md select-none"
      >
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-cyan-500/20 text-cyan-400 border border-cyan-400/40 shadow-[0_0_16px_rgba(56,189,248,0.35)]">
            <Sparkles className="h-4 w-4" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-bold text-sm tracking-wide text-slate-100">STARSHIP CONTROL</span>
              <span className="rounded-full bg-cyan-400/10 px-2 py-0.5 text-[10px] font-mono font-medium text-cyan-300 border border-cyan-400/20">
                v0.1.0-alpha
              </span>
            </div>
            <p className="text-[10px] text-slate-400">Desktop AI Orchestration Matrix</p>
          </div>
        </div>

        {/* Center Model Indicator */}
        <div className="flex items-center gap-2 rounded-full border border-white/10 bg-black/30 px-3 py-1">
          <span className="h-2 w-2 rounded-full bg-purple-400 animate-pulse" />
          <span className="text-xs font-mono text-slate-300">Active Engine:</span>
          <span className="text-xs font-semibold text-purple-300 font-mono">
            {models[0]?.name ?? "Qwen 2.5 Coder 7B (Local GGUF)"}
          </span>
          <span className="rounded bg-purple-500/20 px-1 text-[9px] font-mono text-purple-300 border border-purple-500/30">
            Vulkan 12GB
          </span>
        </div>

        {/* Actions & HUD Launcher */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleToggleHud}
            className={`flex items-center gap-1.5 rounded-lg border px-3 py-1 text-xs transition-all ${
              hudActive
                ? "border-cyan-400 bg-cyan-400/20 text-cyan-200 shadow-[0_0_12px_rgba(56,189,248,0.3)]"
                : "border-white/10 bg-white/5 text-slate-300 hover:bg-white/10"
            }`}
          >
            <ExternalLink className="h-3.5 w-3.5 text-cyan-400" />
            <span>{hudActive ? "HUD Active" : "Launch Floating HUD"}</span>
          </button>
        </div>
      </header>

      {/* 2. Navigation Strip (44px) */}
      <nav className="flex h-[44px] items-center justify-between border-b border-white/10 px-4 bg-[#080B10]">
        <div className="flex items-center gap-1 text-xs">
          {[
            { id: "missions", label: "Missions & Canvas", icon: Layers },
            { id: "topology", label: "App Ecosystem", icon: Activity },
            { id: "models", label: "Model Control", icon: Cpu },
            { id: "audit", label: "Audit Log", icon: FileText },
            { id: "design", label: "Design Law Verification", icon: ShieldCheck },
          ].map((tab) => {
            const Icon = tab.icon;
            const isActive = navTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setNavTab(tab.id as typeof navTab)}
                className={`flex items-center gap-2 rounded-lg px-3 py-1.5 transition-colors ${
                  isActive
                    ? "bg-cyan-500/15 text-cyan-300 font-medium border border-cyan-500/30 shadow-[0_0_12px_rgba(56,189,248,0.2)]"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/5"
                }`}
              >
                <Icon className="h-3.5 w-3.5" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-3 text-xs font-mono text-slate-400">
          <div className="flex items-center gap-1.5">
            <DbIcon className="h-3.5 w-3.5 text-emerald-400" />
            <span>SQLite WAL (0600):</span>
            <span className="text-emerald-300 font-semibold">
              {systemInfo?.database_ready ? "Operational" : "Ready"}
            </span>
          </div>
        </div>
      </nav>

      {/* 3. Main Multi-Drawer Layout */}
      {navTab === "design" ? (
        <div className="flex-1 overflow-y-auto p-6">
          <DesignVerification />
        </div>
      ) : (
        <div className="flex flex-1 overflow-hidden">
          {/* Left Mission Drawer (280px) */}
          <aside className="w-[280px] border-r border-white/10 bg-[#0E131B]/50 flex flex-col justify-between p-3 select-none">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Missions
                </span>
                <button className="flex items-center gap-1 rounded-md bg-white/5 px-2 py-1 text-[11px] text-cyan-300 hover:bg-white/10 border border-white/10">
                  <Plus className="h-3 w-3" />
                  <span>New</span>
                </button>
              </div>

              {/* Search */}
              <div className="relative">
                <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-slate-500" />
                <input
                  type="text"
                  placeholder="Search missions..."
                  className="w-full rounded-lg border border-white/10 bg-black/20 pl-8 pr-2 py-1 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400"
                />
              </div>

              {/* Mission Sessions */}
              <div className="space-y-1.5">
                <div className="rounded-xl border border-cyan-400/40 bg-cyan-500/10 p-2.5 cursor-pointer shadow-[0_0_16px_rgba(56,189,248,0.15)]">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-slate-100">Tutorial Ep4 Screencast</span>
                    <span className="rounded bg-cyan-400/20 px-1.5 py-0.5 text-[9px] font-mono text-cyan-300 border border-cyan-400/30">
                      ACTIVE
                    </span>
                  </div>
                  <p className="mt-1 text-[11px] text-slate-400 line-clamp-1">
                    Trim dead air in EEP, clean audio in ALP
                  </p>
                  <div className="mt-2 flex items-center justify-between text-[10px] font-mono text-slate-500">
                    <span>3 steps</span>
                    <span>1,420 tokens</span>
                  </div>
                </div>

                <div className="rounded-xl border border-white/5 bg-white/5 p-2.5 opacity-60 hover:opacity-100 transition-opacity cursor-pointer">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-medium text-slate-200">System Package Tuneup</span>
                    <span className="rounded bg-emerald-500/20 px-1.5 py-0.5 text-[9px] font-mono text-emerald-300">
                      DONE
                    </span>
                  </div>
                  <p className="mt-1 text-[11px] text-slate-400 line-clamp-1">
                    Purged 1.8GB orphaned packages via ATP
                  </p>
                </div>
              </div>
            </div>

            <div className="border-t border-white/10 pt-2 text-[10px] font-mono text-slate-500 flex justify-between">
              <span>Security Tier: Enforced</span>
              <span>SO_PEERCRED</span>
            </div>
          </aside>

          {/* Center Canvas (Flexible Workspace) */}
          <main className="flex-1 flex flex-col justify-between overflow-y-auto p-4 bg-[#080B10]">
            <div className="space-y-4 max-w-4xl mx-auto w-full">
              {/* Mission Header Card */}
              <GlassPanel recipe="pane" className="p-4 space-y-2">
                <div className="flex items-center justify-between">
                  <div>
                    <h2 className="text-base font-bold text-slate-100">
                      Tutorial Ep4 Screencast Assembly
                    </h2>
                    <p className="text-xs text-slate-400">
                      Goal: Automate multi-app workflow spanning Easy Edits Pro, ScreenStudio Pro, and Audio Labs Pro.
                    </p>
                  </div>
                  <span className="font-mono text-xs rounded-full bg-amber-500/10 text-amber-300 border border-amber-500/30 px-3 py-1 flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full bg-amber-400 animate-ping" />
                    <span>Planning (1.2s)</span>
                  </span>
                </div>
              </GlassPanel>

              {/* Task Sheet Step Cards */}
              <div className="space-y-2.5">
                <h3 className="text-xs font-semibold tracking-wider text-slate-400 uppercase">
                  Execution DAG (Task Sheet v1)
                </h3>

                {/* Step 1: Safe Inspection (Tier 1) */}
                <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-3 flex items-start justify-between">
                  <div className="flex items-start gap-3">
                    <CheckCircle2 className="h-4 w-4 text-emerald-400 mt-0.5" />
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-xs text-slate-100">
                          1. Ingest CET telemetry from ScreenStudio Pro
                        </span>
                        <span className="rounded bg-emerald-500/20 text-emerald-300 font-mono text-[9px] px-1.5 py-0.5 border border-emerald-500/30">
                          TIER 1 (READ-ONLY)
                        </span>
                      </div>
                      <p className="text-xs text-slate-400 mt-0.5">
                        Extracted 14 dead-air silence intervals and mouse clicks from CET sidecar.
                      </p>
                    </div>
                  </div>
                  <span className="text-[10px] font-mono text-emerald-400">Complete (18ms)</span>
                </div>

                {/* Step 2: Audio Labs Pro (Tier 1) */}
                <div className="rounded-xl border border-cyan-400/30 bg-cyan-400/5 p-3 flex items-start justify-between">
                  <div className="flex items-start gap-3">
                    <Play className="h-4 w-4 text-cyan-400 mt-0.5 animate-pulse" />
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="font-semibold text-xs text-slate-100">
                          2. Clean voiceover audio in Audio Labs Pro
                        </span>
                        <span className="rounded bg-cyan-400/20 text-cyan-300 font-mono text-[9px] px-1.5 py-0.5 border border-cyan-400/30">
                          TIER 1 (NON-MUTATING)
                        </span>
                      </div>
                      <p className="text-xs text-slate-400 mt-0.5">
                        Applying de-reverb and RNNoise spectral gate over track-01.wav.
                      </p>
                    </div>
                  </div>
                  <span className="text-[10px] font-mono text-cyan-400">Running...</span>
                </div>

                {/* Step 3: Mutating Operation Gate (Tier 2) */}
                <GlassPanel
                  recipe="float"
                  glow="rose"
                  className="p-4 space-y-3 border-rose-500/40 bg-rose-500/10"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <AlertTriangle className="h-4 w-4 text-rose-400 animate-bounce" />
                      <span className="font-bold text-xs text-rose-300">
                        HUMAN APPROVAL REQUIRED — Easy Edits Pro
                      </span>
                    </div>
                    <span className="rounded bg-rose-500/20 text-rose-300 font-mono text-[10px] px-2 py-0.5 border border-rose-500/40 font-semibold">
                      TIER 2 (MUTATING)
                    </span>
                  </div>

                  <p className="text-xs text-slate-200">
                    Proposed Action: Apply 18 ripple cuts and remove 01:24 dead time on main timeline.
                  </p>

                  <div className="rounded-lg bg-black/40 border border-white/10 p-2.5 font-mono text-[11px] text-slate-300 space-y-1">
                    <div>Target File: ~/Videos/Tutorial-Ep4.eeproj</div>
                    <div>Safety Checkpoint: #bk-04 (Auto-created, 100% reversible)</div>
                    <div>Impact: 18 timeline cuts · 0 audio clips deleted</div>
                  </div>

                  <div className="flex items-center justify-end gap-2 pt-1">
                    <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 text-xs text-slate-300">
                      <X className="h-3.5 w-3.5" />
                      <span>Reject Plan</span>
                    </button>
                    <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-cyan-400 hover:bg-cyan-300 text-slate-950 font-semibold text-xs shadow-[0_0_16px_rgba(56,189,248,0.4)]">
                      <Check className="h-3.5 w-3.5" />
                      <span>Approve & Execute</span>
                    </button>
                  </div>
                </GlassPanel>
              </div>
            </div>

            {/* Bottom Instruction Bar */}
            <div className="mt-4 max-w-4xl mx-auto w-full">
              <div className="relative flex items-center rounded-2xl border border-white/15 bg-[#0E131B]/90 backdrop-blur-xl p-2 shadow-[0_8px_32px_rgba(0,0,0,0.5)]">
                <input
                  type="text"
                  value={prompt}
                  onChange={(e) => setPrompt(e.target.value)}
                  placeholder="Enter instruction or request task sheet decomposition..."
                  className="flex-1 bg-transparent px-3 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none"
                />
                <button className="flex h-8 items-center gap-1.5 rounded-xl bg-cyan-400 hover:bg-cyan-300 px-3 text-xs font-semibold text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.35)] transition-transform active:scale-95">
                  <Send className="h-3.5 w-3.5" />
                  <span>Send</span>
                </button>
              </div>
            </div>
          </main>

          {/* Right Topology Drawer (300px) */}
          <aside className="w-[300px] border-l border-white/10 bg-[#0E131B]/50 flex flex-col justify-between p-3 select-none">
            <div className="space-y-4">
              <div>
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Sub-App Topology
                </span>
                <p className="text-[10px] text-slate-500">Unix Domain Sockets & MCP Stdio</p>
              </div>

              {/* Sub-Apps Matrix */}
              <div className="space-y-2">
                {[
                  { id: "eep", name: "Easy Edits Pro", status: "Live GUI", mode: "socket", ping: "0.4ms" },
                  { id: "ssp", name: "ScreenStudio Pro", status: "Live GUI", mode: "socket", ping: "0.6ms" },
                  { id: "alp", name: "Audio Labs Pro", status: "Ready", mode: "socket", ping: "1.1ms" },
                  { id: "atp", name: "Agentic Tuneup Pro", status: "Headless CLI", mode: "stdio", ping: "—" },
                  { id: "nwp", name: "Net Watch Pro", status: "eBPF Sensor", mode: "stdio", ping: "—" },
                ].map((app) => (
                  <div
                    key={app.id}
                    className="flex items-center justify-between rounded-xl border border-white/10 bg-white/5 p-2 text-xs"
                  >
                    <div>
                      <div className="font-semibold text-slate-200">{app.name}</div>
                      <div className="text-[10px] font-mono text-slate-400 flex items-center gap-1.5">
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                        <span>{app.status}</span>
                      </div>
                    </div>
                    <div className="text-right font-mono text-[10px] text-slate-400">
                      <div>{app.mode}</div>
                      <div className="text-cyan-400">{app.ping}</div>
                    </div>
                  </div>
                ))}
              </div>

              {/* Resource Gauges */}
              <div className="border-t border-white/10 pt-3 space-y-2">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Hardware & Budget
                </span>
                <div className="grid grid-cols-2 gap-2 text-xs font-mono">
                  <div className="rounded-lg border border-white/10 bg-black/30 p-2">
                    <span className="text-[10px] text-slate-500 block">VRAM</span>
                    <span className="text-purple-300 font-semibold">5.4 / 12 GB</span>
                  </div>
                  <div className="rounded-lg border border-white/10 bg-black/30 p-2">
                    <span className="text-[10px] text-slate-500 block">CPU LOAD</span>
                    <span className="text-cyan-300 font-semibold">12%</span>
                  </div>
                  <div className="rounded-lg border border-white/10 bg-black/30 p-2">
                    <span className="text-[10px] text-slate-500 block">TOKENS</span>
                    <span className="text-slate-300 font-semibold">1,420</span>
                  </div>
                  <div className="rounded-lg border border-white/10 bg-black/30 p-2">
                    <span className="text-[10px] text-slate-500 block">EST. COST</span>
                    <span className="text-emerald-300 font-semibold">$0.00</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="border-t border-white/10 pt-2 text-[10px] font-mono text-slate-500 flex items-center justify-between">
              <span>Security Token: Valid</span>
              <RotateCcw className="h-3 w-3 hover:text-cyan-400 cursor-pointer" />
            </div>
          </aside>
        </div>
      )}
    </div>
  );
};
