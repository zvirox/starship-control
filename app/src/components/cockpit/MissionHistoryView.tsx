import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  History,
  RotateCcw,
  CheckCircle2,
  XCircle,
  Clock,
  Play,
  ArrowRight,
  Search,
  AlertTriangle,
  Cpu,
  Layers,
  Flame,
  Activity,
  ShieldAlert,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type { MissionHistorySummary, ReplayMissionResponse } from "../../types/ipc";

interface MissionHistoryViewProps {
  onReplayMission: (missionId: number) => void;
}

export const MissionHistoryView: React.FC<MissionHistoryViewProps> = ({
  onReplayMission,
}) => {
  const [missions, setMissions] = useState<MissionHistorySummary[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [selectedMission, setSelectedMission] = useState<MissionHistorySummary | null>(null);
  const [replayData, setReplayData] = useState<ReplayMissionResponse | null>(null);
  const [replaying, setReplaying] = useState<boolean>(false);

  const fetchHistory = useCallback(async () => {
    try {
      setLoading(true);
      const data = await invoke<MissionHistorySummary[]>("cmd_mission_list_history");
      setMissions(data);
      if (data.length > 0 && !selectedMission) {
        setSelectedMission(data[0]);
      }
    } catch (err) {
      console.error("Failed to load mission history:", err);
    } finally {
      setLoading(false);
    }
  }, [selectedMission]);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const handleInspectReplay = async (m: MissionHistorySummary) => {
    setSelectedMission(m);
    try {
      setReplaying(true);
      const data = await invoke<ReplayMissionResponse>("cmd_mission_replay", {
        payload: { missionId: m.id },
      });
      setReplayData(data);
    } catch (err) {
      console.error("Failed to replay mission state:", err);
    } finally {
      setReplaying(false);
    }
  };

  const filteredMissions = missions.filter((m) => {
    const matchesSearch =
      m.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.userGoal.toLowerCase().includes(searchQuery.toLowerCase()) ||
      m.activeModelId.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesStatus =
      statusFilter === "all" ||
      (statusFilter === "completed" && (m.status === "completed" || m.status === "completed_final")) ||
      (statusFilter === "failed" && (m.status === "failed" || m.status === "failed_final")) ||
      (statusFilter === "running" && (m.status === "executing" || m.status === "running"));
    return matchesSearch && matchesStatus;
  });

  const formatDuration = (ms: number): string => {
    if (ms <= 0) return "0s";
    const secs = (ms / 1000).toFixed(1);
    if (Number(secs) < 60) return `${secs}s`;
    const mins = Math.floor(Number(secs) / 60);
    const remSecs = Math.round(Number(secs) % 60);
    return `${mins}m ${remSecs}s`;
  };

  return (
    <div className="space-y-6">
      {/* Top Header & Search Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-medium text-text-primary flex items-center gap-2">
            <History className="h-4 w-4" />
            <span>Mission history and event replay</span>
          </h2>
          <p className="text-xs text-text-secondary mt-1">
            Event-sourced audit trail and deterministic state reconstruction from SQLite run logs
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Search Input */}
          <div className="relative">
            <Search className="h-3.5 w-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-text-secondary" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search past missions..."
              className="bg-surface-solid border border-border-subtle rounded-xl pl-8 pr-3 py-2 text-xs font-medium text-text-primary placeholder:text-text-secondary focus:outline-none focus:border-accent w-48 transition-colors"
            />
          </div>

          {/* Status Filter */}
          <div className="flex items-center rounded-xl bg-white/[0.03] border border-border-subtle p-0.5 text-xs font-medium">
            {["all", "completed", "failed"].map((st) => (
              <button
                key={st}
                onClick={() => setStatusFilter(st)}
                className={`px-2 py-1 rounded-lg transition-colors cursor-pointer ${
                  statusFilter === st
                    ? "bg-accent-muted text-accent font-semibold"
                    : "text-text-secondary hover:text-text-primary"
                }`}
              >
                {st.charAt(0).toUpperCase() + st.slice(1)}
              </button>
            ))}
          </div>

          {/* Refresh Button */}
          <button
            onClick={fetchHistory}
            className="flex items-center gap-1.5 px-3 py-2 rounded-xl border border-border-subtle bg-white/[0.03] hover:bg-white/[0.06] text-xs font-semibold text-text-secondary hover:text-text-primary transition-colors cursor-pointer"
          >
            <RotateCcw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Main Content Grid: Mission Cards List (Left) + Selected Replay Overview (Right) */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left Column: Mission History Cards (2 cols) */}
        <div className="lg:col-span-2 space-y-3">
          {loading && missions.length === 0 ? (
            <div className="flex items-center justify-center p-12 text-xs font-medium text-text-secondary">
              Loading mission records...
            </div>
          ) : filteredMissions.length === 0 ? (
            <GlassPanel recipe="pane" className="p-8 text-center text-xs font-medium text-text-secondary rounded-2xl">
              No missions found matching your filter
            </GlassPanel>
          ) : (
            filteredMissions.map((m) => {
              const isSelected = selectedMission?.id === m.id;
              const isSuccess = m.status === "completed" || m.status === "completed_final";
              const isFailed = m.status === "failed" || m.status === "failed_final";

              return (
                <GlassPanel
                  key={m.id}
                  recipe="pane"
                  className={`p-4 rounded-2xl border transition-colors cursor-pointer ${
                    isSelected
                      ? "border-accent/30 bg-accent-muted"
                      : "border-border-subtle hover:border-border-glass"
                  }`}
                  onClick={() => handleInspectReplay(m)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="space-y-2 min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[11px] font-semibold text-text-secondary">
                          #{m.id}
                        </span>
                        <h3 className="text-sm font-semibold text-text-primary truncate">
                          {m.title || "Untitled Mission"}
                        </h3>

                        {/* Status Chip */}
                        <span
                          className={`flex items-center gap-1 px-2 py-0.5 rounded-lg text-[11px] font-semibold border ${
                            isSuccess
                              ? "bg-signal-success-bg text-signal-success border-signal-success/30"
                              : isFailed
                              ? "bg-signal-gate-bg text-signal-gate border-signal-gate/30"
                              : "bg-accent-muted text-accent border-accent/30"
                          }`}
                        >
                          {isSuccess ? (
                            <CheckCircle2 className="h-3.5 w-3.5" />
                          ) : isFailed ? (
                            <XCircle className="h-3.5 w-3.5" />
                          ) : (
                            <Clock className="h-3.5 w-3.5" />
                          )}
                          <span className="capitalize">{m.status.replace("_", " ")}</span>
                        </span>
                      </div>

                      <p className="text-xs text-text-secondary font-medium line-clamp-2 leading-relaxed">
                        {m.userGoal}
                      </p>

                      {/* Metadata Row */}
                      <div className="flex flex-wrap items-center gap-3 pt-1 text-[11px] font-mono text-text-secondary font-medium">
                        <span className="flex items-center gap-1 text-text-secondary">
                          <Clock className="h-3 w-3" />
                          <span>{formatDuration(m.durationMs)}</span>
                        </span>

                        <span className="flex items-center gap-1 text-text-secondary">
                          <Layers className="h-3 w-3" />
                          <span>{m.outcome}</span>
                        </span>

                        {m.fallbackCount > 0 ? (
                          <span className="flex items-center gap-1 text-signal-thinking bg-signal-thinking-bg px-2 py-0.5 rounded-lg border border-signal-thinking/30">
                            <AlertTriangle className="h-3.5 w-3.5" />
                            <span>
                              {m.fallbackCount} fallback{m.fallbackCount === 1 ? "" : "s"}
                            </span>
                          </span>
                        ) : (
                          <span className="text-text-secondary">0 fallbacks</span>
                        )}

                        <span className="flex items-center gap-1">
                          <Cpu className="h-3 w-3" />
                          <span>{m.activeModelId}</span>
                        </span>

                        {/* Health Summary Metrics (GAP-CLOSURE-PLAN.md §14.11) */}
                        {m.peakCpuPct > 0 && (
                          <span className="flex items-center gap-1 text-accent bg-accent-muted px-2 py-0.5 rounded-lg border border-accent/30 font-semibold">
                            <Activity className="h-3.5 w-3.5" />
                            <span>{Math.round(m.peakCpuPct)}% CPU</span>
                          </span>
                        )}

                        {m.peakRamBytes > 0 && (
                          <span className="text-text-secondary">
                            {(m.peakRamBytes / 1024 / 1024).toFixed(0)} MB RAM
                          </span>
                        )}

                        {m.peakTempCelsius > 0 && (
                          <span className="flex items-center gap-1 text-signal-gate bg-signal-gate-bg px-2 py-0.5 rounded-lg border border-signal-gate/30 font-semibold">
                            <Flame className="h-3.5 w-3.5" />
                            <span>{Math.round(m.peakTempCelsius)}°C</span>
                          </span>
                        )}

                        {m.governorInterventions > 0 && (
                          <span className="flex items-center gap-1 text-signal-thinking bg-signal-thinking-bg px-2 py-0.5 rounded-lg border border-signal-thinking/30">
                            <ShieldAlert className="h-3.5 w-3.5" />
                            <span>
                              {m.governorInterventions} governor action
                              {m.governorInterventions === 1 ? "" : "s"}
                            </span>
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Replay Timeline Button */}
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onReplayMission(m.id);
                      }}
                      className="flex items-center gap-1.5 px-3 py-2 rounded-xl bg-accent-muted hover:bg-accent/20 text-accent border border-accent/30 text-xs font-medium transition-colors cursor-pointer shrink-0 mt-0.5"
                    >
                      <Play className="h-3 w-3 fill-current" />
                      <span>Timeline</span>
                    </button>
                  </div>
                </GlassPanel>
              );
            })
          )}
        </div>

        {/* Right Column: Deterministic Replay Inspector Card (1 col) */}
        <div>
          <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-border-subtle space-y-4 sticky top-6">
            <div className="flex items-center justify-between border-b border-border-subtle pb-3">
              <div className="flex items-center gap-2">
                <RotateCcw className={`h-4 w-4 ${replaying ? "animate-spin" : ""}`} />
                <h3 className="text-sm font-medium text-text-primary">Event replay state</h3>
              </div>
              {selectedMission && (
                <span className="font-mono text-xs text-text-secondary">
                  Mission #{selectedMission.id}
                </span>
              )}
            </div>

            {selectedMission ? (
              <div className="space-y-4">
                <div>
                  <h4 className="text-xs font-medium text-text-primary">{selectedMission.title}</h4>
                  <p className="text-[11px] text-text-secondary mt-1">{selectedMission.userGoal}</p>
                </div>

                <div className="rounded-xl bg-black/30 border border-border-subtle p-3 space-y-2 text-xs">
                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Rebuilt mission state</span>
                    <span className="font-mono font-medium text-signal-success">
                      {replayData?.missionState || selectedMission.status}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Event log depth</span>
                    <span className="font-mono text-text-primary">
                      {replayData?.events.length ?? 0} events
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Execution duration</span>
                    <span className="font-mono text-text-primary">
                      {formatDuration(selectedMission.durationMs)}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Fallback recoveries</span>
                    <span className="font-mono text-signal-thinking">
                      {selectedMission.fallbackCount}
                    </span>
                  </div>
                </div>

                {/* Health & Governor Summary Box (GAP-CLOSURE-PLAN.md §14.11) */}
                <div className="rounded-xl bg-black/30 border border-border-subtle p-3 space-y-2 text-xs">
                  <div className="text-[11px] font-medium text-text-secondary flex items-center gap-1.5 pb-1 border-b border-border-subtle">
                    <Activity className="h-3.5 w-3.5" />
                    <span>Resource and governor footprint</span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Peak CPU</span>
                    <span className="font-mono text-text-primary tabular">
                      {selectedMission.peakCpuPct > 0 ? `${selectedMission.peakCpuPct.toFixed(1)}%` : "0.0%"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Peak RAM</span>
                    <span className="font-mono text-text-primary tabular">
                      {selectedMission.peakRamBytes > 0
                        ? `${(selectedMission.peakRamBytes / 1024 / 1024).toFixed(0)} MB`
                        : "0 MB"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Peak temperature</span>
                    <span className="font-mono text-text-primary tabular">
                      {selectedMission.peakTempCelsius > 0
                        ? `${selectedMission.peakTempCelsius.toFixed(1)}°C`
                        : "N/A"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-text-secondary">
                    <span>Governor interventions</span>
                    <span
                      className={`font-mono font-medium tabular ${
                        selectedMission.governorInterventions > 0 ? "text-signal-thinking" : "text-signal-success"
                      }`}
                    >
                      {selectedMission.governorInterventions}
                    </span>
                  </div>
                </div>

                {/* Open Full Timeline */}
                <button
                  type="button"
                  onClick={() => onReplayMission(selectedMission.id)}
                  className="w-full flex items-center justify-center gap-2 py-2 px-4 rounded-xl bg-white/[0.03] hover:bg-white/[0.06] text-text-primary text-xs font-medium border border-border-subtle transition-colors cursor-pointer"
                >
                  <span>Open full DAG timeline</span>
                  <ArrowRight className="h-3.5 w-3.5" />
                </button>
              </div>
            ) : (
              <div className="py-8 text-center text-xs font-medium text-text-secondary">
                Select a mission from the list to inspect its event replay.
              </div>
            )}
          </GlassPanel>
        </div>
      </div>
    </div>
  );
};
