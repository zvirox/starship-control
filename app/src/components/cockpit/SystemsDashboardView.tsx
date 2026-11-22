import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Cpu,
  Database as DbIcon,
  HardDrive,
  Sliders,
  Flame,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  X,
  Gauge,
  Layers,
} from "lucide-react";
import type {
  HealthAlert,
  HealthHistoryPayload,
  HealthHistoryResponse,
  HealthSample,
  HealthSnapshotResponse,
  HealthState,
  HealthStateChangedEvent,
  OverloadThresholds,
  GovernorDirective,
} from "../../types/ipc";

interface SystemsDashboardViewProps {
  onRefresh?: () => void;
}

export const SystemsDashboardView: React.FC<SystemsDashboardViewProps> = () => {
  const [snapshot, setSnapshot] = useState<HealthSnapshotResponse | null>(null);
  const [historySamples, setHistorySamples] = useState<HealthSample[]>([]);
  const [alerts, setAlerts] = useState<HealthAlert[]>([]);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [thresholds, setThresholds] = useState<OverloadThresholds | null>(null);
  const [savingThresholds, setSavingThresholds] = useState(false);
  const [selectedRange, setSelectedRange] = useState<"15m" | "1h" | "24h">("15m");

  const unlistenFns = useRef<(() => void)[]>([]);

  // 1. Initial Load: Snapshot, Thresholds, History
  useEffect(() => {
    let isMounted = true;

    async function loadInitialData() {
      try {
        const snap = await invoke<HealthSnapshotResponse>("cmd_health_get_snapshot");
        if (isMounted) setSnapshot(snap);

        const th = await invoke<OverloadThresholds>("cmd_health_get_thresholds");
        if (isMounted) setThresholds(th);

        const hist = await invoke<HealthHistoryResponse>("cmd_health_get_history", {
          payload: { range: selectedRange } as HealthHistoryPayload,
        });
        if (isMounted && hist.samples.length > 0) {
          setHistorySamples(hist.samples);
        }
      } catch (err) {
        console.error("Failed to load initial system health data:", err);
      }
    }

    loadInitialData();

    // 2. Real-time Tauri Push Events
    async function setupListeners() {
      try {
        const unlistenSample = await listen<HealthSample>("evt://health/sample", (evt) => {
          if (!isMounted) return;
          setSnapshot((prev) =>
            prev
              ? { ...prev, sample: evt.payload }
              : {
                  sample: evt.payload,
                  state: "normal",
                  governorDirective: {
                    currentHealthState: "normal",
                    maxParallelism: 2,
                    allowHeavySteps: true,
                    allowLocalInference: true,
                    allowBackgroundPrefetch: true,
                    shouldPause: false,
                    shouldResume: false,
                    shouldUnloadIdleModel: false,
                    actionMessage: "Standard execution",
                  },
                }
          );
          setHistorySamples((prev) => {
            const next = [...prev, evt.payload];
            return next.length > 900 ? next.slice(next.length - 900) : next;
          });
        });

        const unlistenState = await listen<HealthStateChangedEvent>(
          "evt://health/state_changed",
          (evt) => {
            if (!isMounted) return;
            setSnapshot((prev) =>
              prev
                ? {
                    ...prev,
                    state: evt.payload.currentState,
                    governorDirective: evt.payload.directive,
                  }
                : null
            );
          }
        );

        const unlistenAlert = await listen<HealthAlert>("evt://health/alert", (evt) => {
          if (!isMounted) return;
          setAlerts((prev) => [evt.payload, ...prev.slice(0, 19)]);
        });

        unlistenFns.current = [unlistenSample, unlistenState, unlistenAlert];
      } catch (err) {
        console.error("Failed to bind health event listeners:", err);
      }
    }

    setupListeners();

    return () => {
      isMounted = false;
      unlistenFns.current.forEach((fn) => fn());
      unlistenFns.current = [];
    };
  }, [selectedRange]);

  const handleSaveThresholds = async () => {
    if (!thresholds) return;
    setSavingThresholds(true);
    try {
      const updated = await invoke<OverloadThresholds>("cmd_health_set_thresholds", {
        thresholds,
      });
      setThresholds(updated);
      setIsSettingsOpen(false);
    } catch (err) {
      console.error("Failed to update overload thresholds:", err);
    } finally {
      setSavingThresholds(false);
    }
  };

  const sample = snapshot?.sample;
  const state: HealthState = snapshot?.state || "normal";
  const directive: GovernorDirective | undefined = snapshot?.governorDirective;

  // State styling helper
  const stateBadge = {
    normal: {
      label: "Normal",
      className: "text-emerald-400 bg-emerald-500/10 border-emerald-500/20",
      icon: CheckCircle2,
    },
    elevated: {
      label: "Elevated",
      className: "text-amber-400 bg-amber-500/10 border-amber-500/20",
      icon: AlertTriangle,
    },
    constrained: {
      label: "Constrained",
      className: "text-orange-400 bg-orange-500/10 border-orange-500/20",
      icon: ShieldAlert,
    },
    critical: {
      label: "Critical",
      className: "text-rose-400 bg-rose-500/10 border-rose-500/20 animate-pulse",
      icon: ShieldAlert,
    },
  }[state];

  const StateIcon = stateBadge.icon;

  return (
    <div className="flex-1 flex flex-col h-full bg-[#0B0D11] text-slate-100 overflow-y-auto p-6 select-none font-sans">
      <div className="max-w-6xl mx-auto w-full space-y-6">
        {/* 1. Header & Overall State Chip */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-white/[0.03] border border-white/[0.08] backdrop-blur-md">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-sky-500/10 border border-sky-500/20">
              <Activity className="h-5 w-5 text-sky-400" />
            </div>
            <div>
              <div className="flex items-center gap-2.5">
                <h2 className="text-base font-semibold text-slate-100 tracking-tight">
                  System Health & Resource Governor
                </h2>
                <div
                  className={`flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium border ${stateBadge.className}`}
                >
                  <StateIcon className="h-3.5 w-3.5" />
                  <span>{stateBadge.label}</span>
                </div>
              </div>
              <p className="text-xs text-slate-400 mt-0.5">
                {directive?.actionMessage || "Real-time non-blocking system monitoring and overload protection"}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="flex items-center rounded-xl bg-white/[0.03] border border-white/[0.08] p-1 text-xs">
              {(["15m", "1h", "24h"] as const).map((r) => (
                <button
                  key={r}
                  onClick={() => setSelectedRange(r)}
                  className={`px-3 py-1 rounded-lg transition-colors cursor-pointer ${
                    selectedRange === r
                      ? "bg-sky-500/20 text-sky-300 font-medium"
                      : "text-slate-400 hover:text-slate-200"
                  }`}
                >
                  {r}
                </button>
              ))}
            </div>

            <button
              onClick={() => setIsSettingsOpen(true)}
              className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-white/[0.04] border border-white/[0.08] text-xs text-slate-300 hover:bg-white/[0.08] transition-colors cursor-pointer"
            >
              <Sliders className="h-3.5 w-3.5 text-slate-400" />
              <span>Thresholds</span>
            </button>
          </div>
        </div>

        {/* 2. Primary Gauge Row (CPU, RAM, GPU, VRAM, Disk, Temp) */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3.5">
          {/* CPU Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">CPU</span>
              <Cpu className="h-3.5 w-3.5 text-sky-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.cpu ? `${Math.round(sample.cpu.globalUtilPct)}%` : "0%"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5">
                {sample?.cpu?.avgFreqMhz ? `${sample.cpu.avgFreqMhz} MHz` : "1.0 Hz"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>PSI Stall</span>
                <span>{sample?.psi ? `${sample.psi.cpuSomeAvg10.toFixed(1)}%` : "0.0%"}</span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-sky-400 transition-all duration-300"
                  style={{ width: `${Math.min(100, (sample?.psi?.cpuSomeAvg10 || 0) * 2)}%` }}
                />
              </div>
            </div>
          </div>

          {/* Memory Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">Memory</span>
              <DbIcon className="h-3.5 w-3.5 text-indigo-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.memory ? `${Math.round(100 - sample.memory.availablePct)}%` : "0%"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5">
                {sample?.memory
                  ? `${(sample.memory.usedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
                      sample.memory.totalBytes / 1024 / 1024 / 1024
                    ).toFixed(1)} GB`
                  : "0 GB"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>MemAvail</span>
                <span>{sample?.memory ? `${sample.memory.availablePct.toFixed(0)}%` : "N/A"}</span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-indigo-400 transition-all duration-300"
                  style={{ width: `${Math.max(0, 100 - (sample?.memory?.availablePct || 0))}%` }}
                />
              </div>
            </div>
          </div>

          {/* GPU Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">GPU</span>
              <Gauge className="h-3.5 w-3.5 text-emerald-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.gpu ? `${Math.round(sample.gpu.utilPct)}%` : "0%"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5 truncate">
                {sample?.gpu?.deviceName || "Integrated Graphics"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>Compute</span>
                <span>{sample?.gpu ? `${sample.gpu.utilPct.toFixed(0)}%` : "Idle"}</span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-emerald-400 transition-all duration-300"
                  style={{ width: `${sample?.gpu?.utilPct || 0}%` }}
                />
              </div>
            </div>
          </div>

          {/* VRAM Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">VRAM</span>
              <Layers className="h-3.5 w-3.5 text-teal-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.gpu?.vramTotalBytes
                  ? `${Math.round(sample.gpu.vramUsedPct)}%`
                  : "N/A"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5">
                {sample?.gpu?.vramTotalBytes
                  ? `${(sample.gpu.vramUsedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
                      sample.gpu.vramTotalBytes / 1024 / 1024 / 1024
                    ).toFixed(1)} GB`
                  : "Shared System Memory"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>Allocation</span>
                <span>{sample?.gpu?.vramTotalBytes ? `${sample.gpu.vramUsedPct.toFixed(0)}%` : "0%"}</span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-teal-400 transition-all duration-300"
                  style={{ width: `${sample?.gpu?.vramUsedPct || 0}%` }}
                />
              </div>
            </div>
          </div>

          {/* Disk Free Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">Disk Free</span>
              <HardDrive className="h-3.5 w-3.5 text-amber-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.disk ? `${Math.round(sample.disk.freePct)}%` : "N/A"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5">
                {sample?.disk
                  ? `${(sample.disk.freeBytes / 1024 / 1024 / 1024).toFixed(0)} GB Free`
                  : "Root Partition"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>I/O Throughput</span>
                <span>
                  {sample?.disk
                    ? `${((sample.disk.readBytesPerSec + sample.disk.writeBytesPerSec) / 1024 / 1024).toFixed(1)} MB/s`
                    : "0.0 MB/s"}
                </span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-amber-400 transition-all duration-300"
                  style={{ width: `${Math.min(100, 100 - (sample?.disk?.freePct || 0))}%` }}
                />
              </div>
            </div>
          </div>

          {/* Thermal Metric */}
          <div className="p-4 rounded-2xl bg-white/[0.03] border border-white/[0.08] flex flex-col justify-between">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span className="font-medium text-slate-300">Thermal</span>
              <Flame className="h-3.5 w-3.5 text-rose-400" />
            </div>
            <div className="my-3">
              <div className="text-2xl font-bold text-slate-100 tracking-tight">
                {sample?.thermal ? `${Math.round(sample.thermal.maxTempCelsius)}°C` : "N/A"}
              </div>
              <div className="text-[11px] text-slate-500 mt-0.5">
                {sample?.thermal?.fanRpm ? `${sample.thermal.fanRpm} RPM` : "Δ to Max: 25°C"}
              </div>
            </div>
            <div className="space-y-1">
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>Margin</span>
                <span>
                  {sample?.thermal ? `${sample.thermal.deltaToMax.toFixed(0)}°C safety` : "Safe"}
                </span>
              </div>
              <div className="h-1 w-full bg-white/[0.06] rounded-full overflow-hidden">
                <div
                  className="h-full bg-rose-400 transition-all duration-300"
                  style={{
                    width: `${Math.min(100, ((sample?.thermal?.maxTempCelsius || 40) / 100) * 100)}%`,
                  }}
                />
              </div>
            </div>
          </div>
        </div>

        {/* 3. 15-Minute Sparkline Trend with Mission Marker Overlay */}
        <div className="p-5 rounded-2xl bg-white/[0.03] border border-white/[0.08] backdrop-blur-md">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-sm font-semibold text-slate-200">System Telemetry Sparkline (15 min)</h3>
              <p className="text-xs text-slate-500 mt-0.5">
                Continuous 1 Hz ring buffer metrics with automated safe-pause checkpoints
              </p>
            </div>
            <div className="flex items-center gap-4 text-xs">
              <div className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-sky-400" />
                <span className="text-slate-400">CPU %</span>
              </div>
              <div className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full bg-indigo-400" />
                <span className="text-slate-400">Memory %</span>
              </div>
            </div>
          </div>

          <div className="h-36 w-full relative">
            <svg className="w-full h-full overflow-visible" viewBox="0 0 800 120" preserveAspectRatio="none">
              {/* Grid lines */}
              <line x1="0" y1="30" x2="800" y2="30" stroke="rgba(255,255,255,0.04)" strokeDasharray="4 4" />
              <line x1="0" y1="60" x2="800" y2="60" stroke="rgba(255,255,255,0.04)" strokeDasharray="4 4" />
              <line x1="0" y1="90" x2="800" y2="90" stroke="rgba(255,255,255,0.04)" strokeDasharray="4 4" />

              {/* Render CPU polyline */}
              {historySamples.length > 1 && (
                <polyline
                  fill="none"
                  stroke="#38BDF8"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  points={historySamples
                    .map((s, idx) => {
                      const x = (idx / (historySamples.length - 1)) * 800;
                      const cpuUtil = s.cpu?.globalUtilPct || 0;
                      const y = 110 - (cpuUtil / 100) * 100;
                      return `${x},${y}`;
                    })
                    .join(" ")}
                />
              )}

              {/* Render Memory polyline */}
              {historySamples.length > 1 && (
                <polyline
                  fill="none"
                  stroke="#818CF8"
                  strokeWidth="1.5"
                  strokeDasharray="3 3"
                  points={historySamples
                    .map((s, idx) => {
                      const x = (idx / (historySamples.length - 1)) * 800;
                      const memUtil = s.memory ? 100 - s.memory.availablePct : 0;
                      const y = 110 - (memUtil / 100) * 100;
                      return `${x},${y}`;
                    })
                    .join(" ")}
                />
              )}
            </svg>
          </div>
        </div>

        {/* 4. Top Processes & Footprint Table */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="lg:col-span-2 p-5 rounded-2xl bg-white/[0.03] border border-white/[0.08] backdrop-blur-md">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold text-slate-200">Top Resource Consumers</h3>
              <span className="text-xs text-slate-500">Auto-tagged Starship sub-apps</span>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-white/[0.06] text-slate-500 pb-2">
                    <th className="py-2 font-medium">Process</th>
                    <th className="py-2 font-medium">PID</th>
                    <th className="py-2 font-medium">CPU %</th>
                    <th className="py-2 font-medium">Memory</th>
                    <th className="py-2 font-medium text-right">Tag</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/[0.04]">
                  {sample?.topProcesses && sample.topProcesses.length > 0 ? (
                    sample.topProcesses.map((p) => (
                      <tr
                        key={p.pid}
                        className={`transition-colors ${
                          p.isStarshipOwned ? "bg-sky-500/10 text-sky-200" : "text-slate-300 hover:bg-white/[0.02]"
                        }`}
                      >
                        <td className="py-2 font-mono font-medium">{p.name}</td>
                        <td className="py-2 text-slate-500">{p.pid}</td>
                        <td className="py-2 font-medium">{p.cpuPct.toFixed(1)}%</td>
                        <td className="py-2 font-mono text-slate-400">
                          {(p.memBytes / 1024 / 1024).toFixed(0)} MB
                        </td>
                        <td className="py-2 text-right">
                          {p.isStarshipOwned ? (
                            <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-sky-500/20 text-sky-300 border border-sky-500/30">
                              Starship App
                            </span>
                          ) : (
                            <span className="text-slate-600 text-[10px]">Host</span>
                          )}
                        </td>
                      </tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={5} className="py-4 text-center text-slate-500">
                        Gathering process metrics...
                      </td>
                    </tr>
                  )}

                  {/* Starship Footprint Total Row */}
                  {sample?.ownFootprint && (
                    <tr className="border-t-2 border-white/[0.1] bg-sky-500/15 font-semibold text-sky-200">
                      <td className="py-2.5">Starship Total Footprint</td>
                      <td className="py-2.5 text-sky-400">{sample.ownFootprint.processCount} procs</td>
                      <td className="py-2.5">{sample.ownFootprint.totalCpuPct.toFixed(1)}%</td>
                      <td className="py-2.5 font-mono">
                        {(sample.ownFootprint.totalRssBytes / 1024 / 1024).toFixed(0)} MB
                      </td>
                      <td className="py-2.5 text-right text-[10px] uppercase tracking-wider text-sky-300">
                        Combined
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* 5. Alerts Timeline Drawer */}
          <div className="p-5 rounded-2xl bg-white/[0.03] border border-white/[0.08] backdrop-blur-md flex flex-col">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold text-slate-200">Governor Alerts</h3>
              <span className="text-xs text-slate-500">{alerts.length} logged</span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2.5 max-h-[300px] pr-1">
              {alerts.length > 0 ? (
                alerts.map((a, idx) => (
                  <div
                    key={idx}
                    className="p-2.5 rounded-xl bg-white/[0.02] border border-white/[0.06] text-xs space-y-1"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-semibold text-slate-200 uppercase tracking-wide text-[10px]">
                        {a.signal}
                      </span>
                      <span className="text-[10px] text-amber-400 font-medium capitalize">
                        {a.action}
                      </span>
                    </div>
                    <div className="flex items-center justify-between text-slate-400 text-[11px]">
                      <span>Level: {a.level}</span>
                      <span>Value: {a.value.toFixed(1)}</span>
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center h-32 text-center text-slate-500 text-xs">
                  <CheckCircle2 className="h-6 w-6 text-emerald-500/50 mb-2" />
                  <span>No overload alerts tripped</span>
                  <span className="text-[10px] text-slate-600 mt-0.5">System operating within thresholds</span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* 6. Settings Drawer for Thresholds */}
      {isSettingsOpen && thresholds && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg rounded-2xl bg-[#12141A] border border-white/[0.1] p-6 shadow-2xl space-y-5">
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <div className="flex items-center gap-2">
                <Sliders className="h-4 w-4 text-sky-400" />
                <h3 className="text-sm font-semibold text-slate-100">Overload Detector Thresholds</h3>
              </div>
              <button
                onClick={() => setIsSettingsOpen(false)}
                className="text-slate-400 hover:text-slate-200 p-1 rounded-lg cursor-pointer"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <div className="space-y-4 text-xs text-slate-300 max-h-[60vh] overflow-y-auto pr-1">
              <div>
                <label className="block text-slate-400 mb-1">CPU Elevated Threshold (%)</label>
                <input
                  type="number"
                  value={thresholds.cpuElevatedPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, cpuElevatedPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.04] border border-white/[0.08] px-3 py-2 text-slate-100"
                />
              </div>

              <div>
                <label className="block text-slate-400 mb-1">CPU Critical Threshold (%)</label>
                <input
                  type="number"
                  value={thresholds.cpuCriticalPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, cpuCriticalPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.04] border border-white/[0.08] px-3 py-2 text-slate-100"
                />
              </div>

              <div>
                <label className="block text-slate-400 mb-1">MemAvailable Critical (%)</label>
                <input
                  type="number"
                  value={thresholds.memAvailCriticalPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, memAvailCriticalPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.04] border border-white/[0.08] px-3 py-2 text-slate-100"
                />
              </div>

              <div>
                <label className="block text-slate-400 mb-1">Thermal Critical Delta (°C to Max)</label>
                <input
                  type="number"
                  value={thresholds.thermalDeltaCritical}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, thermalDeltaCritical: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.04] border border-white/[0.08] px-3 py-2 text-slate-100"
                />
              </div>

              <div>
                <label className="block text-slate-400 mb-1">Anti-Flapping Exit Counter (Samples)</label>
                <input
                  type="number"
                  value={thresholds.exitConsecutiveSamples}
                  onChange={(e) =>
                    setThresholds({
                      ...thresholds,
                      exitConsecutiveSamples: parseInt(e.target.value, 10) || 30,
                    })
                  }
                  className="w-full rounded-xl bg-white/[0.04] border border-white/[0.08] px-3 py-2 text-slate-100"
                />
              </div>
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-white/[0.08]">
              <button
                onClick={() => setIsSettingsOpen(false)}
                className="px-4 py-2 rounded-xl text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveThresholds}
                disabled={savingThresholds}
                className="px-4 py-2 rounded-xl text-xs font-semibold bg-sky-500 hover:bg-sky-400 text-slate-900 transition-colors cursor-pointer disabled:opacity-50"
              >
                {savingThresholds ? "Saving..." : "Save Thresholds"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
