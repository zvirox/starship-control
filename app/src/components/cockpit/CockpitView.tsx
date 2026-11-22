import React, { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  MessageSquare,
  Cpu,
  FileText,
  Plus,
  Send,
  ExternalLink,
  Search,
  Settings,
  ChevronDown,
  Wrench,
  Workflow,
  AppWindow,
  Trash2,
} from "lucide-react";
import { ChatCanvas } from "./ChatCanvas";
import { ModelSettingsModal } from "./ModelSettingsModal";
import { ModelControlView } from "./ModelControlView";
import { AppTopologyView } from "./AppTopologyView";
import { AppTopologyDrawer } from "./AppTopologyDrawer";
import { McpToolInspector } from "./McpToolInspector";
import { ActionApprovalCard } from "./ActionApprovalCard";
import { AuditLogView } from "./AuditLogView";
import { PipelineWorkflowView } from "./PipelineWorkflowView";
import type {
  HelloResponse,
  ModelRecord,
  MissionRecord,
  CreateMissionResponse,
  SendPromptResponse,
  PlanActiveResponse,
  ApprovalNeededEvent,
  ApproveStepResponse,
  RejectStepResponse,
} from "../../types/ipc";

export const CockpitView: React.FC = () => {
  const [navTab, setNavTab] = useState<"chat" | "workflows" | "apps" | "tools" | "models" | "activity">("chat");
  const [prompt, setPrompt] = useState("");
  const [hudActive, setHudActive] = useState(false);
  const [systemInfo, setSystemInfo] = useState<HelloResponse | null>(null);
  const [models, setModels] = useState<ModelRecord[]>([]);
  const [activeModel, setActiveModel] = useState<ModelRecord | null>(null);
  const [missions, setMissions] = useState<MissionRecord[]>([]);
  const [activeMissionId, setActiveMissionId] = useState<number>(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [isStreaming, setIsStreaming] = useState<boolean>(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState<boolean>(false);
  const [modelDropdownOpen, setModelDropdownOpen] = useState<boolean>(false);
  const [activePlan, setActivePlan] = useState<PlanActiveResponse | null>(null);
  const [approvalGate, setApprovalGate] = useState<ApprovalNeededEvent | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);

  // Load available models and system info on mount
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

  // Load missions from SQLite
  const loadMissions = useCallback(async () => {
    try {
      const list = await invoke<MissionRecord[]>("cmd_mission_list");
      setMissions(list);
      if (list.length > 0 && activeMissionId === 0) {
        setActiveMissionId(list[0].id);
      }
    } catch (err) {
      console.warn("cmd_mission_list call failed:", err);
    }
  }, [activeMissionId]);

  useEffect(() => {
    loadMissions();
  }, [loadMissions]);

  // Load active plan for current mission
  const fetchActivePlan = useCallback(async (missionId: number) => {
    if (!missionId) return;
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
    if (activeMissionId) {
      fetchActivePlan(activeMissionId);
    }
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

  const handleCreateMission = async () => {
    try {
      const createRes = await invoke<CreateMissionResponse>("cmd_mission_create", {
        payload: {
          title: "New Conversation",
          userGoal: "General assistance",
          modelId: activeModel?.id ?? "qwen2.5-coder-7b-q4",
        },
      });
      setActiveMissionId(createRes.missionId);
      await loadMissions();
      inputRef.current?.focus();
    } catch (err) {
      console.error("Failed to create mission:", err);
    }
  };

  const handleDeleteMission = async (id: number, e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await invoke("cmd_mission_delete", {
        payload: { missionId: id },
      });
      const remaining = missions.filter((m) => m.id !== id);
      setMissions(remaining);
      if (activeMissionId === id) {
        setActiveMissionId(remaining.length > 0 ? remaining[0].id : 0);
      }
    } catch (err) {
      console.error("Failed to delete mission:", err);
    }
  };

  const handleSendPrompt = async (textToSend?: string) => {
    const rawPrompt = (textToSend ?? prompt).trim();
    if (!rawPrompt || isStreaming) return;

    setPrompt("");
    setIsStreaming(true);

    try {
      let mId = activeMissionId;
      if (!mId || mId === 0) {
        const createRes = await invoke<CreateMissionResponse>("cmd_mission_create", {
          payload: {
            title: rawPrompt.length > 30 ? rawPrompt.slice(0, 30) + "..." : rawPrompt,
            userGoal: rawPrompt,
            modelId: activeModel?.id ?? "qwen2.5-coder-7b-q4",
          },
        });
        mId = createRes.missionId;
        setActiveMissionId(mId);
        await loadMissions();
      }

      await invoke<SendPromptResponse>("cmd_mission_send_prompt", {
        payload: {
          missionId: mId,
          prompt: rawPrompt,
        },
      });
    } catch (err) {
      console.error("Failed to dispatch prompt:", err);
      setIsStreaming(false);
    }
  };

  const filteredMissions = missions.filter((m) =>
    m.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    m.user_goal.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex h-screen w-screen flex-col bg-[#0B0D11] text-slate-100 antialiased select-none font-sans overflow-hidden">
      {/* 1. Clean Top Header */}
      <header className="flex h-13 items-center justify-between border-b border-white/[0.08] px-4 bg-[#0E1015]/95 backdrop-blur-md">
        {/* Left: Logo & Brand */}
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-white/[0.05] border border-white/[0.08]">
            <img src="/logo.png" alt="Starship Logo" className="h-5 w-5 object-contain" />
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold text-slate-100 tracking-tight">Starship Control</span>
            <span className="text-[10px] font-medium text-slate-400 bg-white/[0.05] border border-white/[0.08] px-1.5 py-0.5 rounded-md">
              {systemInfo?.version ? `v${systemInfo.version}` : "v0.1.0"}
            </span>
          </div>
        </div>

        {/* Center: Model Selector & Status */}
        <div className="flex items-center gap-2.5">
          {/* Model Dropdown */}
          <div className="relative">
            <button
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="flex items-center gap-2 rounded-xl border border-white/[0.08] bg-white/[0.03] px-3 py-1.5 text-xs text-slate-200 hover:bg-white/[0.06] hover:border-white/[0.15] transition-colors cursor-pointer"
            >
              <Cpu className="h-3.5 w-3.5 text-sky-400" />
              <span className="font-medium">{activeModel?.name || "Select Model"}</span>
              <ChevronDown className="h-3 w-3 text-slate-400" />
            </button>

            {modelDropdownOpen && (
              <div className="absolute left-0 mt-1 w-64 rounded-xl border border-white/[0.1] bg-[#14171E] p-1.5 shadow-2xl z-50">
                <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-slate-500">
                  Select Engine
                </div>
                {models.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => {
                      setActiveModel(m);
                      setModelDropdownOpen(false);
                    }}
                    className={`w-full flex items-center justify-between rounded-lg px-2.5 py-1.5 text-xs text-left cursor-pointer ${
                      activeModel?.id === m.id
                        ? "bg-sky-500/15 text-sky-300 font-medium"
                        : "text-slate-300 hover:bg-white/[0.05]"
                    }`}
                  >
                    <span>{m.name}</span>
                    <span className="text-[10px] text-slate-500 capitalize">{m.provider_type.replace("cloud_", "")}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Status Dot */}
          <div className="flex items-center gap-1.5 text-[11px] text-slate-400 bg-white/[0.02] border border-white/[0.06] px-2.5 py-1.5 rounded-xl">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            <span>Ready</span>
          </div>
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsSettingsOpen(true)}
            title="Model Settings & API Keys"
            className="flex h-8 w-8 items-center justify-center rounded-xl border border-white/[0.08] bg-white/[0.03] hover:bg-white/[0.08] text-slate-300 transition-colors cursor-pointer"
          >
            <Settings className="h-3.5 w-3.5" />
          </button>

          <button
            onClick={handleToggleHud}
            className={`flex items-center gap-1.5 rounded-xl border px-3 py-1.5 text-xs transition-colors cursor-pointer ${
              hudActive
                ? "border-sky-400/50 bg-sky-500/15 text-sky-200"
                : "border-white/[0.08] bg-white/[0.03] text-slate-300 hover:bg-white/[0.08]"
            }`}
          >
            <ExternalLink className="h-3.5 w-3.5 text-sky-400" />
            <span>{hudActive ? "HUD Active" : "Launch HUD"}</span>
          </button>
        </div>
      </header>

      {/* 2. Simplified Clean Navigation Bar */}
      <nav className="flex h-11 items-center justify-between border-b border-white/[0.06] px-4 bg-[#0B0D11]">
        <div className="flex items-center gap-1 text-xs">
          {[
            { id: "chat", label: "Chat", icon: MessageSquare },
            { id: "workflows", label: "Workflows", icon: Workflow },
            { id: "apps", label: "Connected Apps", icon: AppWindow },
            { id: "tools", label: "Tools", icon: Wrench },
            { id: "models", label: "Models", icon: Cpu },
            { id: "activity", label: "Activity", icon: FileText },
          ].map((tab) => {
            const Icon = tab.icon;
            const isActive = navTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setNavTab(tab.id as typeof navTab)}
                className={`flex items-center gap-2 rounded-lg px-3 py-1.5 transition-colors cursor-pointer ${
                  isActive
                    ? "bg-white/[0.08] text-slate-100 font-medium"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.03]"
                }`}
              >
                <Icon className="h-3.5 w-3.5 text-slate-400" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-3 text-xs text-slate-500">
          <span>Local First Engine</span>
        </div>
      </nav>

      {/* 3. Main Workspace Area */}
      {navTab === "workflows" ? (
        <div className="flex-1 overflow-y-auto p-6 bg-[#0B0D11]">
          <div className="max-w-5xl mx-auto w-full">
            <PipelineWorkflowView
              missionId={activeMissionId}
              activePlan={activePlan}
              approvalGate={approvalGate}
              onPlanUpdated={setActivePlan}
              onApprovalGateUpdated={setApprovalGate}
            />
          </div>
        </div>
      ) : navTab === "apps" ? (
        <AppTopologyView />
      ) : navTab === "tools" ? (
        <McpToolInspector />
      ) : navTab === "models" ? (
        <ModelControlView
          activeModelId={activeModel?.id}
          onSelectModel={(selected) => {
            setActiveModel(selected);
            setNavTab("chat");
          }}
        />
      ) : navTab === "activity" ? (
        <AuditLogView />
      ) : (
        /* Chat View with Sidebar */
        <div className="flex flex-1 overflow-hidden">
          {/* Left Sidebar: Conversations (260px) */}
          <aside className="w-[260px] border-r border-white/[0.06] bg-[#0E1015]/60 flex flex-col justify-between p-3 select-none">
            <div className="space-y-3">
              {/* New Chat Button */}
              <button
                onClick={handleCreateMission}
                className="w-full flex items-center justify-center gap-2 rounded-xl bg-white/[0.06] hover:bg-white/[0.1] border border-white/[0.08] px-3 py-2 text-xs font-medium text-slate-100 transition-colors cursor-pointer"
              >
                <Plus className="h-3.5 w-3.5 text-sky-400" />
                <span>New Chat</span>
              </button>

              {/* Search */}
              <div className="relative">
                <Search className="absolute left-2.5 top-2 h-3.5 w-3.5 text-slate-500" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search chats..."
                  className="w-full rounded-xl border border-white/[0.06] bg-black/20 pl-8 pr-2 py-1.5 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-white/[0.15]"
                />
              </div>

              {/* Conversation List */}
              <div className="space-y-1 overflow-y-auto max-h-[calc(100vh-250px)]">
                {filteredMissions.length === 0 ? (
                  <div className="py-6 text-center text-xs text-slate-500">
                    No conversations yet
                  </div>
                ) : (
                  filteredMissions.map((m) => (
                    <div
                      key={m.id}
                      onClick={() => setActiveMissionId(m.id)}
                      className={`group flex items-center justify-between rounded-xl px-2.5 py-2 text-xs cursor-pointer transition-colors ${
                        activeMissionId === m.id
                          ? "bg-white/[0.08] text-slate-100 font-medium"
                          : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.03]"
                      }`}
                    >
                      <div className="flex items-center gap-2 truncate">
                        <MessageSquare className="h-3.5 w-3.5 shrink-0 text-slate-500 group-hover:text-slate-400" />
                        <span className="truncate">{m.title || "Untitled Chat"}</span>
                      </div>
                      <button
                        onClick={(e) => handleDeleteMission(m.id, e)}
                        title="Delete chat"
                        className="opacity-0 group-hover:opacity-100 text-slate-500 hover:text-rose-400 p-1 rounded transition-opacity cursor-pointer"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>

            {/* Sidebar Bottom: Connected Apps status */}
            <div className="border-t border-white/[0.06] pt-3 text-[11px] text-slate-400 flex items-center justify-between">
              <span className="flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
                <span>Ecosystem active</span>
              </span>
              <button
                onClick={() => setNavTab("apps")}
                className="text-slate-400 hover:text-slate-200 text-[11px] underline cursor-pointer"
              >
                View apps
              </button>
            </div>
          </aside>

          {/* Center: Chat Stream & Action Cards */}
          <main className="flex-1 flex flex-col justify-between overflow-hidden bg-[#0B0D11]">
            <div className="flex-1 flex flex-col overflow-y-auto p-4 max-w-4xl mx-auto w-full">
              {/* Pending Action Approval Card if required */}
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
                  <div className="mb-4">
                    <ActionApprovalCard
                      missionId={activeMissionId}
                      step={pendingStep}
                      planTitle={activePlan?.title}
                      onApprove={handleApproveStep}
                      onReject={handleRejectStep}
                    />
                  </div>
                );
              })()}

              {/* Chat Canvas */}
              <ChatCanvas
                missionId={activeMissionId}
                isStreaming={isStreaming}
                onStreamFinished={() => setIsStreaming(false)}
                onSelectSuggestion={(selectedPrompt) => handleSendPrompt(selectedPrompt)}
                onNavigateTab={(tab) => setNavTab(tab as typeof navTab)}
              />
            </div>

            {/* Modern Floating Bottom Input Bar */}
            <div className="p-4 max-w-4xl mx-auto w-full">
              <div className="relative flex items-center rounded-2xl border border-white/[0.1] bg-[#12151B] p-2 shadow-lg focus-within:border-sky-500/50 transition-colors">
                <input
                  ref={inputRef}
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
                      ? "Generating response..."
                      : "Type your message or instruction (e.g. 'Record screen demo and clean vocal track')..."
                  }
                  className="flex-1 bg-transparent px-3 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none disabled:opacity-50"
                />

                <button
                  disabled={isStreaming || !prompt.trim()}
                  onClick={() => handleSendPrompt()}
                  className="flex h-8 items-center gap-1.5 rounded-xl bg-white/[0.08] hover:bg-white/[0.15] text-slate-100 px-3.5 text-xs font-medium transition-colors disabled:opacity-30 cursor-pointer"
                >
                  <Send className="h-3.5 w-3.5 text-sky-400" />
                  <span>{isStreaming ? "Thinking..." : "Send"}</span>
                </button>
              </div>
            </div>
          </main>

          {/* Right Topology Drawer */}
          <AppTopologyDrawer />
        </div>
      )}

      {/* Model Settings Modal */}
      <ModelSettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
    </div>
  );
};
