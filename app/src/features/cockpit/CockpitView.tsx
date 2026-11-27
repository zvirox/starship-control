import React, { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  LayoutDashboard,
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
import {
  TitleBar,
  Sidebar,
  Breadcrumb,
  StatusPill,
  LinearButton,
  type SidebarNavItem,
} from "../../components/linear";
import { CockpitDashboard } from "./CockpitDashboard";
import { ChatCanvas } from "../../components/cockpit/ChatCanvas";
import { ModelSettingsModal } from "../../components/cockpit/ModelSettingsModal";
import { ModelControlView } from "../../components/cockpit/ModelControlView";
import { AppTopologyView } from "../../components/cockpit/AppTopologyView";
import { AppTopologyDrawer } from "../../components/cockpit/AppTopologyDrawer";
import { McpToolInspector } from "../../components/cockpit/McpToolInspector";
import { ActionApprovalCard } from "../../components/cockpit/ActionApprovalCard";
import { AuditLogView } from "../../components/cockpit/AuditLogView";
import { PipelineWorkflowView } from "../../components/cockpit/PipelineWorkflowView";
import { MissionTimelineView } from "../../components/cockpit/MissionTimelineView";
import { MissionHistoryView } from "../../components/cockpit/MissionHistoryView";
import { SystemsDashboardView } from "../../components/cockpit/SystemsDashboardView";
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

type CockpitTab =
  | "overview"
  | "chat"
  | "timeline"
  | "history"
  | "workflows"
  | "apps"
  | "tools"
  | "models"
  | "systems"
  | "activity";

const TAB_LABELS: Record<CockpitTab, string> = {
  overview: "Command Center",
  chat: "Agent Chat",
  timeline: "Mission Timeline",
  history: "Mission History",
  workflows: "Cross-App Workflows",
  apps: "Connected Apps",
  tools: "MCP Tool Inspector",
  models: "Models & Hub",
  systems: "System Health",
  activity: "Audit Trail",
};

export const CockpitView: React.FC = () => {
  const [navTab, setNavTab] = useState<CockpitTab>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [workspaceId, setWorkspaceId] = useState("local");

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
  const [approvalGate, setApprovalGate] = useState<ApprovalNeededEvent | null>(
    null
  );

  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    async function initHandshake() {
      try {
        const hello = await invoke<HelloResponse>("ipc_hello");
        setSystemInfo(hello);
        setModels(hello.available_models);
        if (hello.available_models.length > 0) {
          const defaultModel =
            hello.available_models.find((m) => m.is_default) ||
            hello.available_models[0];
          setActiveModel(defaultModel);
        }
      } catch (err) {
        console.warn("ipc_hello call in browser mode:", err);
      }
    }
    initHandshake();
  }, []);

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

  const fetchActivePlan = useCallback(async (missionId: number) => {
    if (!missionId) return;
    try {
      const plan = await invoke<PlanActiveResponse | null>(
        "cmd_plan_get_active",
        { payload: { missionId } }
      );
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

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    async function setupApprovalListener() {
      unlisten = await listen<ApprovalNeededEvent>(
        "evt://plan/approval_needed",
        (event) => {
          if (event.payload.missionId === activeMissionId) {
            setApprovalGate(event.payload);
            fetchActivePlan(activeMissionId);
          }
        }
      );
    }
    setupApprovalListener().catch(console.warn);
    return () => {
      if (unlisten) unlisten();
    };
  }, [activeMissionId, fetchActivePlan]);

  const handleApproveStep = async (stepId: number, signatureToken: string) => {
    await invoke<ApproveStepResponse>("cmd_plan_approve_step", {
      payload: {
        missionId: activeMissionId,
        stepId,
        signatureToken,
      },
    });
    setApprovalGate(null);
    await fetchActivePlan(activeMissionId);
  };

  const handleRejectStep = async (stepId: number, reason: string) => {
    await invoke<RejectStepResponse>("cmd_plan_reject_step", {
      payload: {
        missionId: activeMissionId,
        stepId,
        reason,
      },
    });
    setApprovalGate(null);
    await fetchActivePlan(activeMissionId);
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
    if (!activeModel) return;
    try {
      const createRes = await invoke<CreateMissionResponse>(
        "cmd_mission_create",
        {
          payload: {
            title: "New Conversation",
            userGoal: "General assistance",
            modelId: activeModel.id,
          },
        }
      );
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
      await invoke("cmd_mission_delete", { payload: { missionId: id } });
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
    if (!rawPrompt || isStreaming || !activeModel) return;

    setPrompt("");
    setIsStreaming(true);

    try {
      let mId = activeMissionId;
      if (!mId || mId === 0) {
        const createRes = await invoke<CreateMissionResponse>(
          "cmd_mission_create",
          {
            payload: {
              title:
                rawPrompt.length > 30
                  ? `${rawPrompt.slice(0, 30)}...`
                  : rawPrompt,
              userGoal: rawPrompt,
              modelId: activeModel.id,
            },
          }
        );
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

  const filteredMissions = missions.filter(
    (m) =>
      m.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.user_goal.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const pendingGateCount =
    activePlan?.steps.filter(
      (s) => s.riskTier >= 2 && s.approvalStatus === "pending"
    ).length ?? (approvalGate ? 1 : 0);

  const sidebarItems: SidebarNavItem[] = [
    {
      id: "overview",
      label: "Command Center",
      icon: LayoutDashboard,
      badge: pendingGateCount > 0 ? pendingGateCount : undefined,
    },
    { id: "chat", label: "Chat", icon: MessageSquare },
    { id: "timeline", label: "Timeline", icon: Clock },
    { id: "history", label: "Missions", icon: History },
    { id: "workflows", label: "Workflows", icon: Workflow },
    { id: "apps", label: "Connected Apps", icon: AppWindow },
    { id: "tools", label: "Tools", icon: Wrench },
    { id: "models", label: "Models", icon: Cpu },
    { id: "systems", label: "Systems", icon: Activity },
    { id: "activity", label: "Activity", icon: FileText },
  ];

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-[var(--bg-app)] font-sans text-[var(--text)] antialiased select-none">
      {/* 0. Window TitleBar (40px, Section 1 spec) */}
      <TitleBar
        title="Starship Control"
        version={systemInfo?.version ?? "0.1.0-beta.1"}
        icon={
          <img
            src="/logo.png"
            alt=""
            className="size-5 object-contain"
            aria-hidden="true"
          />
        }
      />

      {/* 1. Shell Body: Collapsible 240px Sidebar + Breadcrumb + Workspace */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        <Sidebar
          items={sidebarItems}
          activeId={navTab}
          onSelect={(id) => setNavTab(id as CockpitTab)}
          activeWorkspaceId={workspaceId}
          onSelectWorkspace={setWorkspaceId}
          collapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed((c) => !c)}
          footer={
            !sidebarCollapsed ? (
              <div className="flex items-center justify-between px-1 py-1 text-[12px] text-[var(--text-muted)]">
                <StatusPill label="Local Engine" status="online" />
              </div>
            ) : (
              <div className="flex justify-center py-1">
                <span
                  className="size-2 rounded-full bg-[var(--accent-green)]"
                  title="Local Engine Online"
                />
              </div>
            )
          }
        />

        {/* Right Main Column: Breadcrumb Bar + View Content */}
        <div className="flex flex-1 flex-col min-w-0 overflow-hidden">
          <Breadcrumb
            segments={[
              {
                label:
                  workspaceId === "local" ? "Starship Local" : "Hybrid Cloud",
                onClick: () => setNavTab("overview"),
              },
              { label: TAB_LABELS[navTab] },
            ]}
            actions={
              <div className="flex items-center gap-2">
                {/* Active Model Picker */}
                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setModelDropdownOpen((o) => !o)}
                    className="h-7 flex items-center gap-2 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] px-2.5 text-[12px] text-[var(--text)] hover:border-[var(--border-strong)] transition-colors duration-150 focus-ring cursor-pointer"
                  >
                    <Cpu
                      className="size-3.5 text-[var(--accent-blue)]"
                      strokeWidth={1.75}
                    />
                    <span className="font-medium truncate max-w-44">
                      {activeModel?.name || "Select model"}
                    </span>
                    <ChevronDown
                      className="size-3.5 text-[var(--text-muted)]"
                      strokeWidth={1.75}
                    />
                  </button>

                  {modelDropdownOpen && (
                    <div className="surface-panel absolute right-0 z-50 mt-1 w-64 rounded-[8px] border-[var(--border-strong)] p-1.5">
                      <div className="px-2 py-1 text-[12px] font-medium text-[var(--text-muted)]">
                        Select engine
                      </div>
                      {models.map((m) => (
                        <button
                          key={m.id}
                          type="button"
                          onClick={() => {
                            setActiveModel(m);
                            setModelDropdownOpen(false);
                          }}
                          className={`flex w-full items-center justify-between rounded-[6px] px-2 py-1.5 text-left text-[12px] transition-colors duration-150 focus-ring cursor-pointer ${
                            activeModel?.id === m.id
                              ? "bg-[var(--tint-blue)] text-[var(--accent-blue)] font-semibold"
                              : "text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)]"
                          }`}
                        >
                          <span className="font-medium truncate">{m.name}</span>
                          <span className="text-[11px] font-mono text-[var(--text-faint)] capitalize shrink-0 ml-2">
                            {m.provider_type.replace("cloud_", "")}
                          </span>
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                <LinearButton
                  variant="secondary"
                  size="sm"
                  onClick={() => setIsSettingsOpen(true)}
                  aria-label="Model settings and API keys"
                  title="Model settings and API keys"
                >
                  <Settings className="size-3.5" strokeWidth={1.75} />
                </LinearButton>

                <LinearButton
                  variant={hudActive ? "primary" : "secondary"}
                  size="sm"
                  onClick={handleToggleHud}
                >
                  <ExternalLink className="size-3.5" strokeWidth={1.75} />
                  <span>{hudActive ? "HUD active" : "Launch HUD"}</span>
                </LinearButton>
              </div>
            }
          />

          {/* Active Workspace Canvas */}
          {navTab === "overview" ? (
            <CockpitDashboard
              models={models}
              missions={missions}
              activePlan={activePlan}
              approvalGate={approvalGate}
              onNavigateTab={setNavTab}
            />
          ) : navTab === "timeline" ? (
            <div className="flex-1 overflow-y-auto bg-[var(--bg-app)] p-6">
              <div className="mx-auto w-full max-w-5xl">
                <MissionTimelineView
                  missionId={activeMissionId}
                  missionTitle={
                    activePlan?.title ||
                    missions.find((m) => m.id === activeMissionId)?.title
                  }
                  missionGoal={
                    activePlan?.summary ||
                    missions.find((m) => m.id === activeMissionId)?.user_goal
                  }
                  activePlan={activePlan}
                />
              </div>
            </div>
          ) : navTab === "history" ? (
            <div className="flex-1 overflow-y-auto bg-[var(--bg-app)] p-6">
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
            <div className="flex-1 overflow-y-auto bg-[var(--bg-app)] p-6">
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
            /* Chat view with mission drawer + right topology drawer */
            <div className="flex flex-1 min-h-0 overflow-hidden">
              <aside className="flex w-[200px] xl:w-[230px] shrink-0 flex-col justify-between border-r border-[var(--border)] bg-[var(--bg-surface)] p-3 select-none">
                <div className="space-y-3">
                  <LinearButton
                    variant="secondary"
                    size="md"
                    onClick={handleCreateMission}
                    className="w-full"
                  >
                    <Plus className="size-4" strokeWidth={1.75} />
                    <span>New mission</span>
                  </LinearButton>

                  <div className="relative">
                    <Search
                      className="absolute left-2.5 top-2.5 size-3.5 text-[var(--text-muted)]"
                      strokeWidth={1.75}
                    />
                    <input
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="Search missions..."
                      className="w-full rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] py-1.5 pl-8 pr-3 text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
                    />
                  </div>

                  <div className="max-h-[calc(100vh-260px)] space-y-1 overflow-y-auto">
                    {filteredMissions.length === 0 ? (
                      <div className="py-6 text-center text-[12px] text-[var(--text-muted)]">
                        No missions yet
                      </div>
                    ) : (
                      filteredMissions.map((m) => (
                        <div
                          key={m.id}
                          onClick={() => setActiveMissionId(m.id)}
                          className={`group flex cursor-pointer items-center justify-between rounded-[6px] px-2.5 py-2 text-[12px] transition-colors duration-150 ${
                            activeMissionId === m.id
                              ? "bg-[var(--bg-surface-2)] text-[var(--text)] font-medium border border-[var(--border)]"
                              : "text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)] border border-transparent"
                          }`}
                        >
                          <div className="flex items-center gap-2 truncate">
                            <MessageSquare
                              className="size-3.5 shrink-0 text-[var(--text-muted)]"
                              strokeWidth={1.75}
                            />
                            <span className="truncate font-medium">
                              {m.title || "Untitled mission"}
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={(e) => handleDeleteMission(m.id, e)}
                            title="Delete mission"
                            className="rounded-[4px] p-1 text-[var(--text-muted)] opacity-0 transition-opacity duration-150 hover:text-[var(--accent-red)] group-hover:opacity-100 focus-ring cursor-pointer"
                          >
                            <Trash2 className="size-3.5" strokeWidth={1.75} />
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                </div>

                <div className="flex items-center justify-between border-t border-[var(--border)] pt-3 text-[12px] text-[var(--text-muted)]">
                  <span className="flex items-center gap-2">
                    <span className="size-1.5 rounded-full bg-[var(--accent-green)]" />
                    <span>Ecosystem active</span>
                  </span>
                  <button
                    type="button"
                    onClick={() => setNavTab("apps")}
                    className="text-[12px] text-[var(--text-muted)] underline hover:text-[var(--text)] focus-ring cursor-pointer"
                  >
                    View apps
                  </button>
                </div>
              </aside>

              <main className="flex min-w-0 flex-1 flex-col justify-between overflow-hidden bg-[var(--bg-app)]">
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
                    onSelectSuggestion={(selectedPrompt) =>
                      handleSendPrompt(selectedPrompt)
                    }
                    onNavigateTab={(tab) => setNavTab(tab as CockpitTab)}
                  />
                </div>

                <div className="mx-auto w-full min-w-0 max-w-4xl p-4">
                  <div className="relative flex items-center gap-2 rounded-[8px] border border-[var(--border)] bg-[var(--bg-surface)] p-2 transition-colors duration-150 focus-within:border-[var(--accent-blue)] min-w-0">
                    <input
                      ref={inputRef}
                      type="text"
                      value={prompt}
                      disabled={isStreaming || !activeModel}
                      onChange={(e) => setPrompt(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" && !e.shiftKey) {
                          e.preventDefault();
                          handleSendPrompt();
                        }
                      }}
                      placeholder={
                        isStreaming
                          ? "Thinking and coordinating agents..."
                          : "Type instructions or coordinate sub-apps..."
                      }
                      className="min-w-0 flex-1 bg-transparent px-3 text-[13px] font-medium text-[var(--text)] placeholder:text-[var(--text-faint)] focus:outline-none disabled:opacity-50"
                    />

                    <div className="flex items-center gap-2 shrink-0">
                      <span className="quiet-chip tabular font-medium hidden sm:inline-flex">
                        {activeModel ? (
                          activeModel.name
                        ) : (
                          <button
                            type="button"
                            className="text-[var(--accent-blue)] cursor-pointer"
                            onClick={() => setNavTab("models")}
                          >
                            Choose a model
                          </button>
                        )}
                      </span>
                      <LinearButton
                        variant="primary"
                        size="md"
                        disabled={isStreaming || !activeModel || !prompt.trim()}
                        onClick={() => handleSendPrompt()}
                      >
                        <Send className="size-3.5" strokeWidth={2} />
                        <span>{isStreaming ? "Thinking" : "Send"}</span>
                      </LinearButton>
                    </div>
                  </div>
                </div>
              </main>

              <AppTopologyDrawer />
            </div>
          )}
        </div>
      </div>

      <ModelSettingsModal
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
      />
    </div>
  );
};
