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
import { LinearCard, LinearButton } from "../linear";
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
          <h2 className="text-[16px] font-semibold text-[var(--text)] flex items-center gap-2">
            <History className="size-4 text-[var(--accent-blue)]" />
            <span>Mission history and event replay</span>
          </h2>
          <p className="text-[12px] text-[var(--text-muted)] mt-1">
            Event-sourced audit trail and deterministic state reconstruction from SQLite run logs
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-muted)]" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search past missions..."
              className="bg-[var(--bg-surface)] border border-[var(--border)] rounded-[6px] pl-8 pr-3 py-1.5 text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)] w-48"
            />
          </div>

          <div className="flex items-center rounded-[6px] bg-[var(--bg-surface)] border border-[var(--border)] p-0.5 text-[12px] font-medium">
            {["all", "completed", "failed"].map((st) => (
              <button
                key={st}
                type="button"
                onClick={() => setStatusFilter(st)}
                className={`px-2 py-1 rounded-[4px] transition-colors cursor-pointer ${
                  statusFilter === st
                    ? "bg-[var(--tint-blue)] text-[var(--accent-blue)] font-semibold"
                    : "text-[var(--text-muted)] hover:text-[var(--text)]"
                }`}
              >
                {st.charAt(0).toUpperCase() + st.slice(1)}
              </button>
            ))}
          </div>

          <LinearButton variant="secondary" size="sm" onClick={fetchHistory}>
            <RotateCcw className={`size-3.5 ${loading ? "animate-spin" : ""}`} />
            <span>Refresh</span>
          </LinearButton>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 space-y-3">
          {loading && missions.length === 0 ? (
            <div className="flex items-center justify-center p-12 text-[12px] text-[var(--text-muted)]">
              Loading mission records...
            </div>
          ) : filteredMissions.length === 0 ? (
            <LinearCard level={1} className="p-8 text-center text-[12px] text-[var(--text-muted)]">
              No missions found matching your filter
            </LinearCard>
          ) : (
            filteredMissions.map((m) => {
              const isSelected = selectedMission?.id === m.id;
              const isSuccess = m.status === "completed" || m.status === "completed_final";
              const isFailed = m.status === "failed" || m.status === "failed_final";

              return (
                <LinearCard
                  key={m.id}
                  level={1}
                  className={`p-5 cursor-pointer ${
                    isSelected
                      ? "border-[var(--accent-blue)] bg-[var(--tint-blue)]"
                      : ""
                  }`}
                  onClick={() => handleInspectReplay(m)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="space-y-2 min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[12px] font-semibold text-[var(--text-muted)]">
                          #{m.id}
                        </span>
                        <h3 className="text-[14px] font-semibold text-[var(--text)] truncate">
                          {m.title || "Untitled Mission"}
                        </h3>

                        <span
                          className={`flex items-center gap-1 px-2 py-0.5 rounded-[4px] text-[12px] font-semibold border ${
                            isSuccess
                              ? "bg-[var(--tint-green)] text-[var(--accent-green)] border-[var(--accent-green)]"
                              : isFailed
                              ? "bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--accent-red)]"
                              : "bg-[var(--tint-blue)] text-[var(--accent-blue)] border-[var(--accent-blue)]"
                          }`}
                        >
                          {isSuccess ? (
                            <CheckCircle2 className="size-3.5" />
                          ) : isFailed ? (
                            <XCircle className="size-3.5" />
                          ) : (
                            <Clock className="size-3.5" />
                          )}
                          <span className="capitalize">{m.status.replace("_", " ")}</span>
                        </span>
                      </div>

                      <p className="text-[12px] text-[var(--text-muted)] line-clamp-2 leading-relaxed">
                        {m.userGoal}
                      </p>

                      <div className="flex flex-wrap items-center gap-3 pt-1 text-[12px] font-mono text-[var(--text-muted)]">
                        <span className="flex items-center gap-1">
                          <Clock className="size-3" />
                          <span>{formatDuration(m.durationMs)}</span>
                        </span>

                        <span className="flex items-center gap-1">
                          <Layers className="size-3" />
                          <span>{m.outcome}</span>
                        </span>

                        {m.fallbackCount > 0 ? (
                          <span className="flex items-center gap-1 text-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-amber)]">
                            <AlertTriangle className="size-3.5" />
                            <span>
                              {m.fallbackCount} fallback{m.fallbackCount === 1 ? "" : "s"}
                            </span>
                          </span>
                        ) : (
                          <span>0 fallbacks</span>
                        )}

                        <span className="flex items-center gap-1">
                          <Cpu className="size-3" />
                          <span>{m.activeModelId}</span>
                        </span>

                        {m.peakCpuPct > 0 && (
                          <span className="flex items-center gap-1 text-[var(--accent-blue)] bg-[var(--tint-blue)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-blue)] font-semibold">
                            <Activity className="size-3.5" />
                            <span>{Math.round(m.peakCpuPct)}% CPU</span>
                          </span>
                        )}

                        {m.peakRamBytes > 0 && (
                          <span>{(m.peakRamBytes / 1024 / 1024).toFixed(0)} MB RAM</span>
                        )}

                        {m.peakTempCelsius > 0 && (
                          <span className="flex items-center gap-1 text-[var(--accent-red)] bg-[var(--tint-red)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-red)] font-semibold">
                            <Flame className="size-3.5" />
                            <span>{Math.round(m.peakTempCelsius)}°C</span>
                          </span>
                        )}

                        {m.governorInterventions > 0 && (
                          <span className="flex items-center gap-1 text-[var(--accent-amber)] bg-[var(--tint-amber)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-amber)]">
                            <ShieldAlert className="size-3.5" />
                            <span>
                              {m.governorInterventions} governor action
                              {m.governorInterventions === 1 ? "" : "s"}
                            </span>
                          </span>
                        )}
                      </div>
                    </div>

                    <LinearButton
                      variant="secondary"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onReplayMission(m.id);
                      }}
                      className="shrink-0 mt-0.5"
                    >
                      <Play className="size-3 fill-current" />
                      <span>Timeline</span>
                    </LinearButton>
                  </div>
                </LinearCard>
              );
            })
          )}
        </div>

        <div>
          <LinearCard level={1} className="p-5 space-y-4 sticky top-6">
            <div className="flex items-center justify-between border-b border-[var(--border)] pb-3">
              <div className="flex items-center gap-2">
                <RotateCcw className={`size-4 ${replaying ? "animate-spin" : ""}`} />
                <h3 className="text-[14px] font-semibold text-[var(--text)]">Event replay state</h3>
              </div>
              {selectedMission && (
                <span className="font-mono text-[12px] text-[var(--text-muted)]">
                  Mission #{selectedMission.id}
                </span>
              )}
            </div>

            {selectedMission ? (
              <div className="space-y-4">
                <div>
                  <h4 className="text-[13px] font-semibold text-[var(--text)]">{selectedMission.title}</h4>
                  <p className="text-[12px] text-[var(--text-muted)] mt-1">{selectedMission.userGoal}</p>
                </div>

                <div className="rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] p-3 space-y-2 text-[12px]">
                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Rebuilt mission state</span>
                    <span className="font-mono font-medium text-[var(--accent-green)]">
                      {replayData?.missionState || selectedMission.status}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Event log depth</span>
                    <span className="font-mono text-[var(--text)]">
                      {replayData?.events.length ?? 0} events
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Execution duration</span>
                    <span className="font-mono text-[var(--text)]">
                      {formatDuration(selectedMission.durationMs)}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Fallback recoveries</span>
                    <span className="font-mono text-[var(--accent-amber)]">
                      {selectedMission.fallbackCount}
                    </span>
                  </div>
                </div>

                <div className="rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] p-3 space-y-2 text-[12px]">
                  <div className="text-[12px] font-medium text-[var(--text-muted)] flex items-center gap-1.5 pb-1 border-b border-[var(--border)]">
                    <Activity className="size-3.5" />
                    <span>Resource and governor footprint</span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Peak CPU</span>
                    <span className="font-mono text-[var(--text)] tabular-nums">
                      {selectedMission.peakCpuPct > 0 ? `${selectedMission.peakCpuPct.toFixed(1)}%` : "0.0%"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Peak RAM</span>
                    <span className="font-mono text-[var(--text)] tabular-nums">
                      {selectedMission.peakRamBytes > 0
                        ? `${(selectedMission.peakRamBytes / 1024 / 1024).toFixed(0)} MB`
                        : "0 MB"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Peak temperature</span>
                    <span className="font-mono text-[var(--text)] tabular-nums">
                      {selectedMission.peakTempCelsius > 0
                        ? `${selectedMission.peakTempCelsius.toFixed(1)}°C`
                        : "N/A"}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-[var(--text-muted)]">
                    <span>Governor interventions</span>
                    <span
                      className={`font-mono font-medium tabular-nums ${
                        selectedMission.governorInterventions > 0
                          ? "text-[var(--accent-amber)]"
                          : "text-[var(--accent-green)]"
                      }`}
                    >
                      {selectedMission.governorInterventions}
                    </span>
                  </div>
                </div>

                <LinearButton
                  variant="secondary"
                  size="md"
                  onClick={() => onReplayMission(selectedMission.id)}
                  className="w-full"
                >
                  <span>Open full DAG timeline</span>
                  <ArrowRight className="size-3.5" />
                </LinearButton>
              </div>
            ) : (
              <div className="py-8 text-center text-[12px] text-[var(--text-muted)]">
                Select a mission from the list to inspect its event replay.
              </div>
            )}
          </LinearCard>
        </div>
      </div>
    </div>
  );
};
