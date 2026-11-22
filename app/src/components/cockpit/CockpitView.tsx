import React, { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Layers,
  Cpu,
  FileText,
  Activity,
  Plus,
  Send,
  Check,
  ExternalLink,
  Search,
  Database as DbIcon,
  ShieldCheck,
  Settings,
  ChevronDown,
  Wrench,
  Eye,
  EyeOff,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import { DesignVerification } from "../common/DesignVerification";
import { ChatCanvas } from "./ChatCanvas";
import { ModelSettingsModal } from "./ModelSettingsModal";
import { ModelControlView } from "./ModelControlView";
import { AppTopologyView } from "./AppTopologyView";
import { AppTopologyDrawer } from "./AppTopologyDrawer";
import { McpToolInspector } from "./McpToolInspector";
import { ActionApprovalCard } from "./ActionApprovalCard";
import { PlanWorkflowViewer } from "./PlanWorkflowViewer";
import { AuditLogView } from "./AuditLogView";
import type {
  HelloResponse,
  ModelRecord,
  CreateMissionResponse,
  SendPromptResponse,
  PlanActiveResponse,
  ApprovalNeededEvent,
  ApproveStepResponse,
  RejectStepResponse,
} from "../../types/ipc";

export const CockpitView: React.FC = () => {
  const [navTab, setNavTab] = useState<"missions" | "topology" | "tools" | "models" | "audit" | "design">("missions");
  const [prompt, setPrompt] = useState("");
  const [hudActive, setHudActive] = useState(false);
  const [systemInfo, setSystemInfo] = useState<HelloResponse | null>(null);
  const [models, setModels] = useState<ModelRecord[]>([]);
  const [activeModel, setActiveModel] = useState<ModelRecord | null>(null);
  const [activeMissionId, setActiveMissionId] = useState<number>(1);
  const [isStreaming, setIsStreaming] = useState<boolean>(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [modelDropdownOpen, setModelDropdownOpen] = useState<boolean>(false);
  const [activePlan, setActivePlan] = useState<PlanActiveResponse | null>(null);
  const [approvalGate, setApprovalGate] = useState<ApprovalNeededEvent | null>(null);
  const [showPlanViewer, setShowPlanViewer] = useState<boolean>(true);

  useEffect(() => {
    async function initHandshake() {
      try {
        const hello = await invoke<HelloResponse>("ipc_hello");
        setSystemInfo(hello);
        setModels(hello.available_models);
        if (hello.available_models.length > 0) {
          const defaultModel = hello.available_models.find((m) => m.is_default) || hello.available_models[0];
          setActiveModel(defaultModel);
        }
      } catch (err) {
        console.warn("ipc_hello call in browser mode:", err);
      }
    }
    initHandshake();
  }, []);

  const fetchActivePlan = useCallback(async (missionId: number) => {
    try {
      const plan = await invoke<PlanActiveResponse | null>("cmd_plan_get_active", {
        payload: { missionId },
      });
      setActivePlan(plan);
    } catch (err) {
      console.warn("Failed to fetch active plan:", err);
    }
  }, []);

  useEffect(() => {
    fetchActivePlan(activeMissionId);
  }, [activeMissionId, fetchActivePlan]);

  // Subscribe to real-time approval events
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    async function setupApprovalListener() {
      unlisten = await listen<ApprovalNeededEvent>("evt://plan/approval_needed", (event) => {
        if (event.payload.missionId === activeMissionId) {
          setApprovalGate(event.payload);
          fetchActivePlan(activeMissionId);
        }
      });
    }
    setupApprovalListener().catch(console.warn);
    return () => {
      if (unlisten) unlisten();
    };
  }, [activeMissionId, fetchActivePlan]);

  const handleApproveStep = async (stepId: number, signatureToken: string) => {
    try {
      await invoke<ApproveStepResponse>("cmd_plan_approve_step", {
        payload: {
          missionId: activeMissionId,
          stepId,
          signatureToken,
        },
      });
      setApprovalGate(null);
      await fetchActivePlan(activeMissionId);
    } catch (err) {
      console.error("Approve step failed:", err);
      throw err;
    }
  };

  const handleRejectStep = async (stepId: number, reason: string) => {
    try {
      await invoke<RejectStepResponse>("cmd_plan_reject_step", {
        payload: {
          missionId: activeMissionId,
          stepId,
          reason,
        },
      });
      setApprovalGate(null);
      await fetchActivePlan(activeMissionId);
    } catch (err) {
      console.error("Reject step failed:", err);
      throw err;
    }
  };

  const handleToggleHud = async () => {
    try {
      const active = await invoke<boolean>("cmd_toggle_hud");
      setHudActive(active);
    } catch (err) {
      console.warn("cmd_toggle_hud call failed:", err);
      setHudActive(!hudActive);
    }
  };

  const handleSendPrompt = async () => {
    if (!prompt.trim() || isStreaming) return;

    const userPrompt = prompt.trim();
    setPrompt("");
    setIsStreaming(true);

    try {
      // If no mission yet, initialize one
      let mId = activeMissionId;
      if (!mId) {
        const createRes = await invoke<CreateMissionResponse>("cmd_mission_create", {
          payload: {
            title: "Autonomous Session",
            userGoal: userPrompt,
            modelId: activeModel?.id ?? "qwen2.5-coder-7b-q4",
          },
        });
        mId = createRes.missionId;
        setActiveMissionId(mId);
      }

      // Automatically decompose into tasksheet/1 plan if goal is executable
      try {
        const plan = await invoke<PlanActiveResponse>("cmd_plan_generate", {
          payload: {
            missionId: mId,
            userGoal: userPrompt,
          },
        });
        setActivePlan(plan);
      } catch (planErr) {
        console.warn("cmd_plan_generate notice:", planErr);
      }

      await invoke<SendPromptResponse>("cmd_mission_send_prompt", {
        payload: {
          missionId: mId,
          prompt: userPrompt,
        },
      });
    } catch (err) {
      console.error("Failed to dispatch prompt:", err);
      setIsStreaming(false);
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
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-orange-500/10 text-orange-400 border border-orange-400/30 overflow-hidden shadow-[0_0_16px_rgba(249,115,22,0.25)]">
            <img src="/logo.png" alt="Starship Control" className="h-7 w-7 object-contain" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-bold text-sm tracking-wide text-slate-100">STARSHIP CONTROL</span>
              <span className="rounded-full bg-cyan-400/10 px-2 py-0.5 text-[10px] font-mono font-medium text-cyan-300 border border-cyan-400/20">
                v0.1.0 · Autonomous
              </span>
            </div>
            <p className="text-[10px] text-slate-400">Multi-App Agent Orchestrator</p>
          </div>
        </div>

        {/* Center Model Selector Dropdown */}
        <div className="relative">
          <button
            onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
            className="flex items-center gap-2 rounded-full border border-white/10 bg-black/40 px-3.5 py-1 text-xs hover:border-cyan-400/50 transition-colors"
          >
            <span
              className={`h-2 w-2 rounded-full ${
                activeModel?.provider_type === "local_gguf" ? "bg-purple-400" : "bg-cyan-400"
              } animate-pulse`}
            />
            <span className="font-mono text-slate-400">Engine:</span>
            <span className="font-semibold text-slate-200 font-mono">
              {activeModel?.name ?? "Qwen 2.5 Coder 7B (Local)"}
            </span>
            <span
              className={`rounded px-1 text-[9px] font-mono border ${
                activeModel?.provider_type === "local_gguf"
                  ? "bg-purple-500/20 text-purple-300 border-purple-500/30"
                  : "bg-cyan-500/20 text-cyan-300 border-cyan-500/30"
              }`}
            >
              {activeModel?.provider_type === "local_gguf" ? "Vulkan GGUF" : "Cloud SSE"}
            </span>
            <ChevronDown className="h-3 w-3 text-slate-400" />
          </button>

          {modelDropdownOpen && (
            <div className="absolute left-0 mt-1.5 w-64 rounded-xl border border-white/10 bg-[#0E131B]/95 backdrop-blur-2xl p-1.5 shadow-[0_8px_32px_rgba(0,0,0,0.6)] z-50 text-xs">
              <div className="px-2 py-1 text-[10px] uppercase tracking-wider text-slate-500 font-semibold flex justify-between">
                <span>Available Engines</span>
                <span>{models.length} loaded</span>
              </div>
              <button
                onClick={() => {
                  setActiveModel({
                    id: "qwen2.5-coder-7b-q4",
                    name: "Qwen 2.5 Coder 7B (Q4_K_M)",
                    provider_type: "local_gguf",
                    hf_repo_id: null,
                    file_path: null,
                    file_size_bytes: null,
                    quantization: "Q4_K_M",
                    sha256_hash: null,
                    download_status: "ready",
                    context_window: 8192,
                    gpu_layers: 33,
                    is_default: true,
                    created_at: "",
                    updated_at: "",
                  });
                  setModelDropdownOpen(false);
                }}
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-white/5 flex items-center justify-between"
              >
                <div>
                  <div className="font-semibold text-slate-200">Qwen 2.5 Coder 7B</div>
                  <div className="text-[10px] text-purple-400 font-mono">Local GGUF · Vulkan Offload</div>
                </div>
                {activeModel?.id === "qwen2.5-coder-7b-q4" && <Check className="h-3.5 w-3.5 text-cyan-400" />}
              </button>

              <button
                onClick={() => {
                  setActiveModel({
                    id: "claude-3-7-sonnet",
                    name: "Claude 3.7 Sonnet",
                    provider_type: "cloud_anthropic",
                    hf_repo_id: null,
                    file_path: null,
                    file_size_bytes: null,
                    quantization: null,
                    sha256_hash: null,
                    download_status: "remote_only",
                    context_window: 200000,
                    gpu_layers: 0,
                    is_default: false,
                    created_at: "",
                    updated_at: "",
                  });
                  setModelDropdownOpen(false);
                }}
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-white/5 flex items-center justify-between"
              >
                <div>
                  <div className="font-semibold text-slate-200">Claude 3.7 Sonnet</div>
                  <div className="text-[10px] text-cyan-400 font-mono">Cloud Anthropic SSE</div>
                </div>
                {activeModel?.id === "claude-3-7-sonnet" && <Check className="h-3.5 w-3.5 text-cyan-400" />}
              </button>

              <button
                onClick={() => {
                  setActiveModel({
                    id: "gemini-2.0-flash",
                    name: "Gemini 2.0 Flash",
                    provider_type: "cloud_gemini",
                    hf_repo_id: null,
                    file_path: null,
                    file_size_bytes: null,
                    quantization: null,
                    sha256_hash: null,
                    download_status: "remote_only",
                    context_window: 1000000,
                    gpu_layers: 0,
                    is_default: false,
                    created_at: "",
                    updated_at: "",
                  });
                  setModelDropdownOpen(false);
                }}
                className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-white/5 flex items-center justify-between"
              >
                <div>
                  <div className="font-semibold text-slate-200">Gemini 2.0 Flash</div>
                  <div className="text-[10px] text-cyan-400 font-mono">Cloud Google SSE</div>
                </div>
                {activeModel?.id === "gemini-2.0-flash" && <Check className="h-3.5 w-3.5 text-cyan-400" />}
              </button>
            </div>
          )}
        </div>

        {/* Actions & HUD Launcher */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsSettingsOpen(true)}
            title="Configure Cloud API Keys"
            className="flex h-7 w-7 items-center justify-center rounded-lg border border-white/10 bg-white/5 hover:bg-white/10 text-slate-300 transition-colors"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>

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
            { id: "missions", label: "Missions & Chat Stream", icon: Layers },
            { id: "topology", label: "App Ecosystem", icon: Activity },
            { id: "tools", label: "MCP Tools", icon: Wrench },
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
      ) : navTab === "models" ? (
        <ModelControlView
          activeModelId={activeModel?.id}
          onSelectModel={(selected) => {
            setActiveModel(selected);
            setNavTab("missions");
          }}
        />
      ) : navTab === "topology" ? (
        <AppTopologyView />
      ) : navTab === "tools" ? (
        <McpToolInspector />
      ) : navTab === "audit" ? (
        <AuditLogView />
      ) : (
        <div className="flex flex-1 overflow-hidden">
          {/* Left Mission Drawer (280px) */}
          <aside className="w-[280px] border-r border-white/10 bg-[#0E131B]/50 flex flex-col justify-between p-3 select-none">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold uppercase tracking-wider text-slate-400">
                  Missions
                </span>
                <button
                  onClick={async () => {
                    try {
                      const res = await invoke<CreateMissionResponse>("cmd_mission_create", {
                        payload: {
                          title: "New Orchestration Session",
                          userGoal: "Pending instruction",
                          modelId: activeModel?.id ?? "qwen2.5-coder-7b-q4",
                        },
                      });
                      setActiveMissionId(res.missionId);
                    } catch (e) {
                      console.warn(e);
                    }
                  }}
                  className="flex items-center gap-1 rounded-md bg-white/5 px-2 py-1 text-[11px] text-cyan-300 hover:bg-white/10 border border-white/10"
                >
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
                <div
                  onClick={() => setActiveMissionId(1)}
                  className={`rounded-xl border p-2.5 cursor-pointer transition-all ${
                    activeMissionId === 1
                      ? "border-cyan-400/40 bg-cyan-500/10 shadow-[0_0_16px_rgba(56,189,248,0.15)]"
                      : "border-white/5 bg-white/5 opacity-60 hover:opacity-100"
                  }`}
                >
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
                    <span>{activePlan ? `${activePlan.steps.length} steps` : "3 steps"}</span>
                    <span>1,420 tokens</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="border-t border-white/10 pt-2 text-[10px] font-mono text-slate-500 flex justify-between">
              <span>Security Tier: Enforced</span>
              <span>SO_PEERCRED</span>
            </div>
          </aside>

          {/* Center Canvas (Chat Stream, Plan DAG & Approval Card) */}
          <main className="flex-1 flex flex-col justify-between overflow-y-auto p-4 bg-[#080B10]">
            <div className="space-y-4 max-w-4xl mx-auto w-full flex-1 flex flex-col">
              {/* Mission Header Card */}
              <GlassPanel recipe="pane" className="p-3.5 space-y-1">
                <div className="flex items-center justify-between">
                  <div>
                    <h2 className="text-sm font-bold text-slate-100">
                      {activePlan?.title || "Tutorial Ep4 Screencast Assembly"}
                    </h2>
                    <p className="text-xs text-slate-400">
                      Goal: {activePlan?.summary || "Automate multi-app workflow spanning Easy Edits Pro, ScreenStudio Pro, and Audio Labs Pro."}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    {activePlan && (
                      <button
                        type="button"
                        onClick={() => setShowPlanViewer(!showPlanViewer)}
                        className="flex items-center gap-1 text-[11px] font-mono rounded-lg border border-white/10 bg-white/5 px-2.5 py-1 text-slate-300 hover:bg-white/10 transition-colors"
                      >
                        {showPlanViewer ? <EyeOff className="h-3 w-3 text-cyan-400" /> : <Eye className="h-3 w-3 text-cyan-400" />}
                        <span>{showPlanViewer ? "Hide Plan DAG" : "Show Plan DAG"}</span>
                      </button>
                    )}
                    <span className="font-mono text-xs rounded-full bg-cyan-500/10 text-cyan-300 border border-cyan-500/30 px-3 py-1 flex items-center gap-1.5">
                      <span className="h-2 w-2 rounded-full bg-cyan-400 animate-pulse" />
                      <span>Engine: {activeModel?.name}</span>
                    </span>
                  </div>
                </div>
              </GlassPanel>

              {/* Execution Plan DAG Pipeline Viewer */}
              {activePlan && showPlanViewer && (
                <PlanWorkflowViewer plan={activePlan} />
              )}

              {/* Tier 2 / Tier 3 Active Approval Gate Card */}
              {(() => {
                const pendingStep =
                  activePlan?.steps.find(
                    (s) => s.riskTier >= 2 && s.approvalStatus === "pending"
                  ) ??
                  (approvalGate
                    ? {
                        stepId: approvalGate.stepId,
                        stepIndex: 0,
                        appId: approvalGate.appId,
                        toolName: approvalGate.actionTitle,
                        riskTier: approvalGate.riskTier,
                        approvalStatus: "pending" as const,
                        executionStatus: "pending" as const,
                        diffSummary: approvalGate.diffSummary,
                        arguments: {},
                        signatureToken: approvalGate.signatureToken,
                      }
                    : null);

                if (!pendingStep) return null;
                return (
                  <ActionApprovalCard
                    missionId={activeMissionId}
                    step={pendingStep}
                    planTitle={activePlan?.title}
                    onApprove={handleApproveStep}
                    onReject={handleRejectStep}
                  />
                );
              })()}

              {/* Chat Canvas with Real-Time Streaming */}
              <ChatCanvas
                missionId={activeMissionId}
                isStreaming={isStreaming}
                onStreamFinished={() => setIsStreaming(false)}
              />
            </div>

            {/* Bottom Instruction Bar */}
            <div className="mt-3 max-w-4xl mx-auto w-full">
              <div className="relative flex items-center rounded-2xl border border-white/15 bg-[#0E131B]/90 backdrop-blur-xl p-2 shadow-[0_8px_32px_rgba(0,0,0,0.5)]">
                <input
                  type="text"
                  value={prompt}
                  disabled={isStreaming}
                  onChange={(e) => setPrompt(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSendPrompt();
                    }
                  }}
                  placeholder={
                    isStreaming
                      ? "Streaming completion tokens..."
                      : "Enter instruction or workflow goal (e.g., 'Record screen with SSP and clean audio with ALP')..."
                  }
                  className="flex-1 bg-transparent px-3 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none disabled:opacity-50"
                />
                <button
                  disabled={isStreaming || !prompt.trim()}
                  onClick={handleSendPrompt}
                  className="flex h-8 items-center gap-1.5 rounded-xl bg-cyan-400 hover:bg-cyan-300 px-3.5 text-xs font-semibold text-slate-950 shadow-[0_0_12px_rgba(56,189,248,0.35)] transition-transform active:scale-95 disabled:opacity-50"
                >
                  <Send className="h-3.5 w-3.5" />
                  <span>{isStreaming ? "Streaming..." : "Send"}</span>
                </button>
              </div>
            </div>
          </main>

          {/* Right Topology Drawer (300px) */}
          <AppTopologyDrawer />
        </div>
      )}

      {/* Cloud Provider Credentials Modal */}
      <ModelSettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
    </div>
  );
};
