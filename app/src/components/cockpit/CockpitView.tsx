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
  Clock,
  History,
  Trash2,
  Activity,
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
import { MissionTimelineView } from "./MissionTimelineView";
import { MissionHistoryView } from "./MissionHistoryView";
import { SystemsDashboardView } from "./SystemsDashboardView";
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
  const [navTab, setNavTab] = useState<"chat" | "timeline" | "history" | "workflows" | "apps" | "tools" | "models" | "systems" | "activity">("chat");
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
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-canvas font-sans text-text-primary antialiased select-none">
      {/* 1. Top header */}
      <header className="flex h-13 items-center justify-between border-b border-border-subtle bg-raised px-4">
        {/* Left: logo and brand */}
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl border border-border-subtle bg-white/[0.05]">
            <img src="/logo.png" alt="Starship Logo" className="h-5 w-5 object-contain" />
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-text-primary">Starship Control</span>
            <span className="quiet-chip tabular">{systemInfo?.version ? `v${systemInfo.version}` : "v0.1.0"}</span>
          </div>
        </div>

        {/* Center: model selector and status */}
        <div className="flex items-center gap-2">
          <div className="relative">
            <button
              onClick={() => setModelDropdownOpen(!modelDropdownOpen)}
              className="flex items-center gap-2 rounded-xl border border-border-subtle bg-white/[0.03] px-3 py-1.5 text-xs text-text-primary transition-colors duration-150 hover:bg-white/[0.05] focus-ring cursor-pointer"
            >
              <Cpu className="h-3.5 w-3.5 text-accent" strokeWidth={1.5} />
              <span className="font-medium">{activeModel?.name || "Select model"}</span>
              <ChevronDown className="h-3.5 w-3.5 text-text-disabled" strokeWidth={1.5} />
            </button>

            {modelDropdownOpen && (
              <div className="absolute left-0 z-50 mt-1 w-64 rounded-xl border border-border-subtle bg-surface-solid p-2">
                <div className="px-2 py-1 text-[11px] text-text-disabled">Select engine</div>
                {models.map((m) => (
                  <button
                    key={m.id}
                    onClick={() => {
                      setActiveModel(m);
                      setModelDropdownOpen(false);
                    }}
                    className={`flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-left text-xs transition-colors duration-150 focus-ring cursor-pointer ${
                      activeModel?.id === m.id
                        ? "bg-accent-muted text-accent font-medium"
                        : "text-text-secondary hover:bg-white/[0.05]"
                    }`}
                  >
                    <span>{m.name}</span>
                    <span className="text-[11px] text-text-disabled capitalize">{m.provider_type.replace("cloud_", "")}</span>
                  </button>
                ))}
              </div>
            )}
          </div>

          <span className="quiet-chip">
            <span className="h-1.5 w-1.5 rounded-full bg-signal-success" />
            Ready
          </span>
        </div>

        {/* Right actions */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsSettingsOpen(true)}
            title="Model settings and API keys"
            className="flex h-8 w-8 items-center justify-center rounded-xl border border-border-subtle bg-white/[0.03] text-text-secondary transition-colors duration-150 hover:bg-white/[0.05] hover:text-text-primary focus-ring cursor-pointer"
          >
            <Settings className="h-4 w-4" strokeWidth={1.5} />
          </button>

          <button
            onClick={handleToggleHud}
            className={`flex items-center gap-2 rounded-xl border px-3 py-1.5 text-xs transition-colors duration-150 focus-ring cursor-pointer ${
              hudActive
                ? "border-accent/50 bg-accent-muted text-accent"
                : "border-border-subtle bg-white/[0.03] text-text-secondary hover:bg-white/[0.05]"
            }`}
          >
            <ExternalLink className="h-4 w-4" strokeWidth={1.5} />
            <span>{hudActive ? "HUD active" : "Launch HUD"}</span>
          </button>
        </div>
      </header>

      {/* 2. Navigation strip */}
      <nav className="flex h-11 items-center justify-between gap-4 border-b border-border-subtle bg-canvas px-4">
        <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto text-xs [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          {[
            { id: "chat", label: "Chat", icon: MessageSquare },
            { id: "timeline", label: "Timeline", icon: Clock },
            { id: "history", label: "Missions", icon: History },
            { id: "workflows", label: "Workflows", icon: Workflow },
            { id: "apps", label: "Connected Apps", icon: AppWindow },
            { id: "tools", label: "Tools", icon: Wrench },
            { id: "models", label: "Models", icon: Cpu },
            { id: "systems", label: "Systems", icon: Activity },
            { id: "activity", label: "Activity", icon: FileText },
          ].map((tab) => {
            const Icon = tab.icon;
            const isActive = navTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setNavTab(tab.id as typeof navTab)}
                className={`flex shrink-0 items-center gap-2 rounded-lg px-3 py-1.5 transition-colors duration-150 focus-ring cursor-pointer ${
                  isActive
                    ? "bg-white/[0.08] text-text-primary font-medium"
                    : "text-text-secondary hover:bg-white/[0.03] hover:text-text-primary"
                }`}
              >
                <Icon className="h-3.5 w-3.5" strokeWidth={1.5} />
                <span className="whitespace-nowrap">{tab.label}</span>
              </button>
            );
          })}
        </div>

        <span className="quiet-chip shrink-0">Local-first engine</span>
      </nav>

      {/* 3. Main workspace area */}
      {navTab === "timeline" ? (
        <div className="flex-1 overflow-y-auto bg-canvas p-6">
          <div className="mx-auto w-full max-w-5xl">
            <MissionTimelineView
              missionId={activeMissionId}
              missionTitle={activePlan?.title || missions.find((m) => m.id === activeMissionId)?.title}
              missionGoal={activePlan?.summary || missions.find((m) => m.id === activeMissionId)?.user_goal}
              activePlan={activePlan}
            />
          </div>
        </div>
      ) : navTab === "history" ? (
        <div className="flex-1 overflow-y-auto bg-canvas p-6">
          <div className="mx-auto w-full max-w-5xl">
            <MissionHistoryView
              onReplayMission={(mId) => {
                setActiveMissionId(mId);
                setNavTab("timeline");
              }}
            />
          </div>
        </div>
      ) : navTab === "workflows" ? (
        <div className="flex-1 overflow-y-auto bg-canvas p-6">
          <div className="mx-auto w-full max-w-5xl">
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
      ) : navTab === "systems" ? (
        <SystemsDashboardView />
      ) : navTab === "activity" ? (
        <AuditLogView />
      ) : (
        /* Chat view with mission sidebar */
        <div className="flex flex-1 overflow-hidden">
          {/* Left sidebar: missions (260px) */}
          <aside className="flex w-[260px] shrink-0 flex-col justify-between border-r border-border-subtle bg-raised/60 p-3 select-none">
            <div className="space-y-3">
              <button
                onClick={handleCreateMission}
                className="flex w-full items-center justify-center gap-2 rounded-xl border border-border-subtle bg-white/[0.05] px-3 py-2 text-xs font-medium text-text-primary transition-colors duration-150 hover:bg-white/[0.08] focus-ring cursor-pointer"
              >
                <Plus className="h-4 w-4 text-text-secondary" strokeWidth={1.5} />
                <span>New mission</span>
              </button>

              <div className="relative">
                <Search className="absolute left-3 top-2 h-4 w-4 text-text-disabled" strokeWidth={1.5} />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search"
                  className="w-full rounded-xl border border-border-subtle bg-black/20 py-1.5 pl-9 pr-3 text-xs text-text-primary placeholder:text-text-disabled focus-ring focus:border-border-glass"
                />
              </div>

              <div className="max-h-[calc(100vh-250px)] space-y-1 overflow-y-auto">
                {filteredMissions.length === 0 ? (
                  <div className="py-6 text-center text-xs text-text-disabled">No missions yet</div>
                ) : (
                  filteredMissions.map((m) => (
                    <div
                      key={m.id}
                      onClick={() => setActiveMissionId(m.id)}
                      className={`group flex cursor-pointer items-center justify-between rounded-xl px-2 py-2 text-xs transition-colors duration-150 ${
                        activeMissionId === m.id
                          ? "bg-white/[0.08] text-text-primary font-medium"
                          : "text-text-secondary hover:bg-white/[0.03] hover:text-text-primary"
                      }`}
                    >
                      <div className="flex items-center gap-2 truncate">
                        <MessageSquare className="h-3.5 w-3.5 shrink-0 text-text-disabled" strokeWidth={1.5} />
                        <span className="truncate">{m.title || "Untitled mission"}</span>
                      </div>
                      <button
                        onClick={(e) => handleDeleteMission(m.id, e)}
                        title="Delete mission"
                        className="rounded p-1 text-text-disabled opacity-0 transition-opacity duration-150 hover:text-signal-gate group-hover:opacity-100 focus-ring cursor-pointer"
                      >
                        <Trash2 className="h-3.5 w-3.5" strokeWidth={1.5} />
                      </button>
                    </div>
                  ))
                )}
              </div>
            </div>

            <div className="flex items-center justify-between border-t border-border-subtle pt-3 text-[11px] text-text-secondary">
              <span className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-signal-success" />
                <span>Ecosystem active</span>
              </span>
              <button
                onClick={() => setNavTab("apps")}
                className="text-[11px] text-text-secondary underline hover:text-text-primary focus-ring cursor-pointer"
              >
                View apps
              </button>
            </div>
          </aside>

          {/* Center: chat stream and action cards */}
          <main className="flex min-w-0 flex-1 flex-col justify-between overflow-hidden bg-canvas">
            <div className="mx-auto flex w-full min-w-0 max-w-4xl flex-1 flex-col overflow-y-auto p-4">
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

              <ChatCanvas
                missionId={activeMissionId}
                isStreaming={isStreaming}
                onStreamFinished={() => setIsStreaming(false)}
                onSelectSuggestion={(selectedPrompt) => handleSendPrompt(selectedPrompt)}
                onNavigateTab={(tab) => setNavTab(tab as typeof navTab)}
              />
            </div>

            <div className="mx-auto w-full min-w-0 max-w-4xl p-4">
              <div className="relative flex items-center rounded-2xl border border-border-subtle bg-surface-solid p-2 transition-colors duration-150 focus-within:border-accent/50">
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
                  placeholder={isStreaming ? "Generating response" : "Message or instruction"}
                  className="flex-1 bg-transparent px-3 text-xs text-text-primary placeholder:text-text-disabled focus:outline-none disabled:opacity-50"
                />

                <button
                  disabled={isStreaming || !prompt.trim()}
                  onClick={() => handleSendPrompt()}
                  className="flex h-8 items-center gap-2 rounded-xl bg-white/[0.08] px-3 text-xs font-medium text-text-primary transition-colors duration-150 hover:bg-white/[0.12] focus-ring disabled:opacity-40 cursor-pointer"
                >
                  <Send className="h-4 w-4 text-text-secondary" strokeWidth={1.5} />
                  <span>{isStreaming ? "Thinking" : "Send"}</span>
                </button>
              </div>
            </div>
          </main>

          {/* Right topology drawer */}
          <AppTopologyDrawer />
        </div>
      )}

      <ModelSettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} />
    </div>
  );
};
