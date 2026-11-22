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
    <div className="space-y-6 animate-in fade-in duration-300">
      {/* Top Header & Search Bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-slate-100 flex items-center gap-2">
            <History className="h-5 w-5 text-sky-400" />
            <span>Mission History & Event Replay</span>
          </h2>
          <p className="text-xs text-slate-400 mt-0.5">
            Event-sourced audit trail and deterministic state reconstruction from SQLite run logs
          </p>
        </div>

        <div className="flex items-center gap-2">
          {/* Search Input */}
          <div className="relative">
            <Search className="h-3.5 w-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-500" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search past missions..."
              className="bg-black/30 border border-white/[0.08] rounded-xl pl-8 pr-3 py-1.5 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-sky-400/40 w-48 transition-colors"
            />
          </div>

          {/* Status Filter */}
          <div className="flex items-center rounded-xl bg-white/[0.03] border border-white/[0.08] p-0.5 text-xs">
            {["all", "completed", "failed"].map((st) => (
              <button
                key={st}
                onClick={() => setStatusFilter(st)}
                className={`px-2.5 py-1 rounded-lg capitalize transition-colors cursor-pointer ${
                  statusFilter === st
                    ? "bg-white/[0.08] text-slate-100 font-medium"
                    : "text-slate-400 hover:text-slate-200"
                }`}
              >
                {st}
              </button>
            ))}
          </div>

          {/* Refresh Button */}
          <button
            onClick={fetchHistory}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl border border-white/[0.08] bg-white/[0.03] hover:bg-white/[0.08] text-xs text-slate-300 transition-colors cursor-pointer"
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
            <div className="flex items-center justify-center p-12 text-xs text-slate-500">
              Loading mission records...
            </div>
          ) : filteredMissions.length === 0 ? (
            <GlassPanel recipe="pane" className="p-8 text-center text-xs text-slate-500 rounded-2xl">
              No missions found matching your filter.
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
                  className={`p-4 rounded-2xl border transition-all cursor-pointer ${
                    isSelected
                      ? "border-sky-400/40 bg-sky-500/[0.04]"
                      : "border-white/[0.06] hover:border-white/[0.12]"
                  }`}
                  onClick={() => handleInspectReplay(m)}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="space-y-1.5 min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[11px] font-semibold text-slate-400">
                          #{m.id}
                        </span>
                        <h3 className="text-sm font-semibold text-slate-100 truncate">
                          {m.title || "Untitled Mission"}
                        </h3>

                        {/* Status Chip */}
                        <span
                          className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium border ${
                            isSuccess
                              ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                              : isFailed
                              ? "bg-rose-500/15 text-rose-300 border-rose-500/30"
                              : "bg-sky-500/15 text-sky-300 border-sky-500/30"
                          }`}
                        >
                          {isSuccess ? (
                            <CheckCircle2 className="h-2.5 w-2.5" />
                          ) : isFailed ? (
                            <XCircle className="h-2.5 w-2.5" />
                          ) : (
                            <Clock className="h-2.5 w-2.5" />
                          )}
                          <span className="capitalize">{m.status.replace("_", " ")}</span>
                        </span>
                      </div>

                      <p className="text-xs text-slate-400 line-clamp-2 leading-relaxed">
                        {m.userGoal}
                      </p>

                      {/* Metadata Row */}
                      <div className="flex flex-wrap items-center gap-3 pt-1 text-[11px] font-mono text-slate-500">
                        <span className="flex items-center gap-1 text-slate-400">
                          <Clock className="h-3 w-3 text-slate-500" />
                          <span>{formatDuration(m.durationMs)}</span>
                        </span>

                        <span className="flex items-center gap-1 text-slate-400">
                          <Layers className="h-3 w-3 text-slate-500" />
                          <span>{m.outcome}</span>
                        </span>

                        {m.fallbackCount > 0 ? (
                          <span className="flex items-center gap-1 text-amber-300 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/30">
                            <AlertTriangle className="h-2.5 w-2.5" />
                            <span>{m.fallbackCount} fallback(s)</span>
                          </span>
                        ) : (
                          <span className="text-slate-500">0 fallbacks</span>
                        )}

                        <span className="flex items-center gap-1 text-slate-500">
                          <Cpu className="h-3 w-3" />
                          <span>{m.activeModelId}</span>
                        </span>
                      </div>
                    </div>

                    {/* Replay Timeline Button */}
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onReplayMission(m.id);
                      }}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-sky-500/15 hover:bg-sky-500/25 text-sky-300 border border-sky-400/30 text-xs font-medium transition-colors cursor-pointer shrink-0 mt-0.5"
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
          <GlassPanel recipe="pane" className="p-5 rounded-2xl border border-white/[0.08] space-y-4 sticky top-6">
            <div className="flex items-center justify-between border-b border-white/[0.06] pb-3">
              <div className="flex items-center gap-2">
                <RotateCcw className={`h-4 w-4 text-sky-400 ${replaying ? "animate-spin" : ""}`} />
                <h3 className="text-sm font-semibold text-slate-100">Event Replay State</h3>
              </div>
              {selectedMission && (
                <span className="font-mono text-xs text-slate-400">
                  Mission #{selectedMission.id}
                </span>
              )}
            </div>

            {selectedMission ? (
              <div className="space-y-4">
                <div>
                  <h4 className="text-xs font-medium text-slate-300">{selectedMission.title}</h4>
                  <p className="text-[11px] text-slate-400 mt-1">{selectedMission.userGoal}</p>
                </div>

                <div className="rounded-xl bg-black/30 border border-white/[0.06] p-3 space-y-2 text-xs">
                  <div className="flex justify-between items-center text-slate-400">
                    <span>Rebuilt Mission State:</span>
                    <span className="font-mono font-semibold text-emerald-300">
                      {replayData?.missionState || selectedMission.status}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-slate-400">
                    <span>Event Log Depth:</span>
                    <span className="font-mono text-slate-200">
                      {replayData?.events.length ?? 0} events
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-slate-400">
                    <span>Execution Duration:</span>
                    <span className="font-mono text-slate-200">
                      {formatDuration(selectedMission.durationMs)}
                    </span>
                  </div>

                  <div className="flex justify-between items-center text-slate-400">
                    <span>Fallback Recoveries:</span>
                    <span className="font-mono text-amber-300">
                      {selectedMission.fallbackCount}
                    </span>
                  </div>
                </div>

                {/* Open Full Timeline */}
                <button
                  type="button"
                  onClick={() => onReplayMission(selectedMission.id)}
                  className="w-full flex items-center justify-center gap-2 py-2.5 px-4 rounded-xl bg-white/[0.06] hover:bg-white/[0.1] text-slate-200 text-xs font-medium border border-white/[0.08] transition-colors cursor-pointer"
                >
                  <span>Open Full DAG Timeline</span>
                  <ArrowRight className="h-3.5 w-3.5 text-sky-400" />
                </button>
              </div>
            ) : (
              <div className="py-8 text-center text-xs text-slate-500">
                Select a mission from the list to inspect its event replay.
              </div>
            )}
          </GlassPanel>
        </div>
      </div>
    </div>
  );
};
