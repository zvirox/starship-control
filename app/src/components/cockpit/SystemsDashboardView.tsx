import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Sliders,
  ShieldAlert,
  CheckCircle2,
  AlertTriangle,
  X,
  Cpu,
  Layers,
  Tv,
  Flame,
  HardDrive,
  Thermometer,
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

  // State styling helper — semantic colors only, no looping animation on critical.
  const stateBadge = {
    normal: {
      label: "Normal",
      className: "text-signal-success bg-signal-success-bg border-border-subtle",
      icon: CheckCircle2,
    },
    elevated: {
      label: "Elevated",
      className: "text-signal-thinking bg-signal-thinking-bg border-border-subtle",
      icon: AlertTriangle,
    },
    constrained: {
      label: "Constrained",
      className: "text-signal-thinking bg-signal-thinking-bg border-border-subtle",
      icon: ShieldAlert,
    },
    critical: {
      label: "Critical",
      className: "text-signal-gate bg-signal-gate-bg border-signal-gate/30",
      icon: ShieldAlert,
    },
  }[state];

  const StateIcon = stateBadge.icon;

  // Colorful, high-contrast stat cards with semantic icons, metrics, and tracks
  const statCards = [
    {
      label: "CPU Load",
      value: sample?.cpu ? `${Math.round(sample.cpu.globalUtilPct)}%` : "0%",
      sub: sample?.cpu?.avgFreqMhz ? `${sample.cpu.avgFreqMhz} MHz` : "1.0 GHz",
      percent: sample?.cpu ? Math.round(sample.cpu.globalUtilPct) : 0,
      icon: Cpu,
      color: "text-sky-400",
      bgColor: "bg-sky-500/10",
      borderColor: "border-sky-500/25",
      barColor: "bg-sky-400",
    },
    {
      label: "RAM Used",
      value: sample?.memory ? `${Math.round(100 - sample.memory.availablePct)}%` : "0%",
      sub: sample?.memory
        ? `${(sample.memory.usedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
            sample.memory.totalBytes / 1024 / 1024 / 1024
          ).toFixed(1)} GB`
        : "0 GB",
      percent: sample?.memory ? Math.round(100 - sample.memory.availablePct) : 0,
      icon: Layers,
      color: "text-purple-400",
      bgColor: "bg-purple-500/10",
      borderColor: "border-purple-500/25",
      barColor: "bg-purple-400",
    },
    {
      label: "GPU Compute",
      value: sample?.gpu ? `${Math.round(sample.gpu.utilPct)}%` : "0%",
      sub: sample?.gpu?.deviceName || "Hardware Acceleration",
      percent: sample?.gpu ? Math.round(sample.gpu.utilPct) : 0,
      icon: Tv,
      color: "text-teal-400",
      bgColor: "bg-teal-500/10",
      borderColor: "border-teal-500/25",
      barColor: "bg-teal-400",
    },
    {
      label: "VRAM Used",
      value: sample?.gpu?.vramTotalBytes ? `${Math.round(sample.gpu.vramUsedPct)}%` : "Shared",
      sub: sample?.gpu?.vramTotalBytes
        ? `${(sample.gpu.vramUsedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
            sample.gpu.vramTotalBytes / 1024 / 1024 / 1024
          ).toFixed(1)} GB`
        : "Unified system memory",
      percent: sample?.gpu?.vramTotalBytes ? Math.round(sample.gpu.vramUsedPct) : 0,
      icon: Flame,
      color: "text-amber-400",
      bgColor: "bg-amber-500/10",
      borderColor: "border-amber-500/25",
      barColor: "bg-amber-400",
    },
    {
      label: "Disk Free",
      value: sample?.disk ? `${Math.round(sample.disk.freePct)}%` : "N/A",
      sub: sample?.disk
        ? `${(sample.disk.freeBytes / 1024 / 1024 / 1024).toFixed(0)} GB free`
        : "Root partition",
      percent: sample?.disk ? Math.round(sample.disk.freePct) : 100,
      icon: HardDrive,
      color: "text-emerald-400",
      bgColor: "bg-emerald-500/10",
      borderColor: "border-emerald-500/25",
      barColor: "bg-emerald-400",
    },
    {
      label: "Thermal Core",
      value: sample?.thermal ? `${Math.round(sample.thermal.maxTempCelsius)}°C` : "Normal",
      sub: sample?.thermal?.fanRpm ? `${sample.thermal.fanRpm} RPM` : "Within safe margin",
      percent: sample?.thermal ? Math.min(100, Math.round((sample.thermal.maxTempCelsius / 100) * 100)) : 35,
      icon: Thermometer,
      color: "text-rose-400",
      bgColor: "bg-rose-500/10",
      borderColor: "border-rose-500/25",
      barColor: "bg-rose-400",
    },
  ];

  // Telemetry chart geometry — muted grid, rich gradients, bright traces
  const chart = { width: 800, height: 200, left: 44, right: 12, top: 14, bottom: 32 };
  const plotWidth = chart.width - chart.left - chart.right;
  const plotHeight = chart.height - chart.top - chart.bottom;
  const baselineY = chart.top + plotHeight;
  const xFor = (idx: number, total: number) =>
    chart.left + (total > 1 ? (idx / (total - 1)) * plotWidth : 0);
  const yFor = (pct: number) =>
    baselineY - (Math.min(100, Math.max(0, pct)) / 100) * plotHeight;
  const timeLabels: Record<typeof selectedRange, string[]> = {
    "15m": ["-15m", "-10m", "-5m", "now"],
    "1h": ["-60m", "-40m", "-20m", "now"],
    "24h": ["-24h", "-16h", "-8h", "now"],
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-canvas text-text-primary overflow-y-auto p-6 select-none font-sans">
      <div className="max-w-6xl mx-auto w-full space-y-6">
        {/* 1. Header & Overall State Chip */}
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-white/[0.03] border border-border-subtle">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-accent-muted text-accent border border-border-subtle">
              <Activity className="h-4 w-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-base font-semibold text-text-primary">
                  System health and resource governor
                </h2>
                <div
                  className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-semibold border ${stateBadge.className}`}
                >
                  <StateIcon className="h-3.5 w-3.5" />
                  <span>{stateBadge.label}</span>
                </div>
              </div>
              <p className="text-xs text-text-secondary mt-1">
                {directive?.actionMessage || "Real-time system monitoring and overload protection"}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="flex items-center rounded-xl bg-white/[0.03] border border-border-subtle p-1 text-xs">
              {(["15m", "1h", "24h"] as const).map((r) => (
                <button
                  key={r}
                  onClick={() => setSelectedRange(r)}
                  className={`px-3 py-1 rounded-lg transition-colors cursor-pointer font-medium ${
                    selectedRange === r
                      ? "bg-accent-muted text-accent font-semibold"
                      : "text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {r}
                </button>
              ))}
            </div>

            <button
              onClick={() => setIsSettingsOpen(true)}
              className="flex items-center gap-2 px-3 py-2 rounded-xl bg-white/[0.03] border border-border-subtle text-xs text-text-secondary hover:bg-white/[0.06] hover:text-text-primary transition-colors cursor-pointer"
            >
              <Sliders className="h-3.5 w-3.5" />
              <span>Thresholds</span>
            </button>
          </div>
        </div>

        {/* 2. Color-accented, high-contrast stat cards */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {statCards.map((card) => {
            const Icon = card.icon;
            return (
              <div
                key={card.label}
                className="p-4 rounded-xl bg-white/[0.03] border border-border-subtle flex flex-col justify-between hover:border-border-glass transition-colors group"
              >
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-text-secondary">{card.label}</span>
                  <div className={`flex h-6 w-6 items-center justify-center rounded-lg ${card.bgColor} ${card.color} border ${card.borderColor}`}>
                    <Icon className="h-3.5 w-3.5" />
                  </div>
                </div>
                <div className="my-2.5">
                  <div className="text-2xl font-bold tabular text-text-primary tracking-tight">
                    {card.value}
                  </div>
                  {/* Subtle colorful progress track */}
                  <div className="w-full h-1.5 rounded-full bg-border-subtle mt-2 overflow-hidden">
                    <div
                      className={`h-full rounded-full ${card.barColor} transition-all duration-300`}
                      style={{ width: `${Math.max(4, Math.min(100, card.percent))}%` }}
                    />
                  </div>
                  <div className="text-[11px] text-text-secondary mt-2 truncate font-mono" title={card.sub}>
                    {card.sub}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {/* 3. System telemetry — rich gradients & colorful traces */}
        <div className="p-5 rounded-2xl bg-white/[0.03] border border-border-subtle">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
            <div>
              <h3 className="text-sm font-semibold text-text-primary">System telemetry</h3>
              <p className="text-xs text-text-secondary mt-1">
                Continuous 1 Hz readings · last {selectedRange}
              </p>
            </div>
            <div className="flex items-center gap-4 text-xs font-medium text-text-secondary">
              <span className="flex items-center gap-2">
                <span className="h-1.5 w-4 rounded-full bg-sky-400" />
                <span className="text-text-primary">CPU Utilization</span>
              </span>
              <span className="flex items-center gap-2">
                <span className="h-1.5 w-4 rounded-full bg-purple-400" />
                <span className="text-text-primary">Memory Usage</span>
              </span>
            </div>
          </div>

          <div className="relative h-64 w-full">
            <svg
              className="w-full h-full overflow-visible"
              viewBox={`0 0 ${chart.width} ${chart.height}`}
              preserveAspectRatio="none"
            >
              <defs>
                <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.25" />
                  <stop offset="100%" stopColor="#38bdf8" stopOpacity="0.0" />
                </linearGradient>
                <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#a855f7" stopOpacity="0.20" />
                  <stop offset="100%" stopColor="#a855f7" stopOpacity="0.0" />
                </linearGradient>
              </defs>

              {/* Muted hairline grid with crisp, readable labels */}
              {[0, 25, 50, 75, 100].map((tick) => (
                <g key={tick}>
                  <line
                    x1={chart.left}
                    y1={yFor(tick)}
                    x2={chart.width - chart.right}
                    y2={yFor(tick)}
                    stroke="var(--color-border-subtle)"
                    strokeDasharray="2 4"
                  />
                  <text
                    x={chart.left - 8}
                    y={yFor(tick) + 4}
                    textAnchor="end"
                    fill="var(--color-text-secondary)"
                    className="font-mono tabular"
                    fontSize="11"
                    fontWeight="500"
                  >
                    {tick}%
                  </text>
                </g>
              ))}
              {timeLabels[selectedRange].map((label, idx) => {
                const x = chart.left + (idx / (timeLabels[selectedRange].length - 1)) * plotWidth;
                return (
                  <g key={label}>
                    <line
                      x1={x}
                      y1={chart.top}
                      x2={x}
                      y2={baselineY}
                      stroke="var(--color-border-subtle)"
                      strokeDasharray="2 4"
                    />
                    <text
                      x={x}
                      y={baselineY + 18}
                      textAnchor={idx === 0 ? "start" : idx === timeLabels[selectedRange].length - 1 ? "end" : "middle"}
                      fill="var(--color-text-secondary)"
                      className="font-mono"
                      fontSize="11"
                      fontWeight="500"
                    >
                      {label}
                    </text>
                  </g>
                );
              })}
              <line
                x1={chart.left}
                y1={baselineY}
                x2={chart.width - chart.right}
                y2={baselineY}
                stroke="var(--color-border-glass)"
              />

              {/* Memory Area Fill & Trace */}
              {historySamples.length > 1 && (
                <>
                  <polygon
                    fill="url(#memGradient)"
                    points={`
                      ${xFor(0, historySamples.length)},${baselineY}
                      ${historySamples
                        .map((s, idx) => `${xFor(idx, historySamples.length)},${yFor(s.memory ? 100 - s.memory.availablePct : 0)}`)
                        .join(" ")}
                      ${xFor(historySamples.length - 1, historySamples.length)},${baselineY}
                    `}
                  />
                  <polyline
                    fill="none"
                    stroke="#a855f7"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    points={historySamples
                      .map((s, idx) =>
                        `${xFor(idx, historySamples.length)},${yFor(s.memory ? 100 - s.memory.availablePct : 0)}`
                      )
                      .join(" ")}
                  />
                </>
              )}

              {/* CPU Area Fill & Trace */}
              {historySamples.length > 1 && (
                <>
                  <polygon
                    fill="url(#cpuGradient)"
                    points={`
                      ${xFor(0, historySamples.length)},${baselineY}
                      ${historySamples
                        .map((s, idx) => `${xFor(idx, historySamples.length)},${yFor(s.cpu?.globalUtilPct || 0)}`)
                        .join(" ")}
                      ${xFor(historySamples.length - 1, historySamples.length)},${baselineY}
                    `}
                  />
                  <polyline
                    fill="none"
                    stroke="#38bdf8"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    points={historySamples
                      .map((s, idx) => `${xFor(idx, historySamples.length)},${yFor(s.cpu?.globalUtilPct || 0)}`)
                      .join(" ")}
                  />
                </>
              )}
            </svg>
            {historySamples.length <= 1 && (
              <div className="absolute inset-0 flex items-center justify-center text-xs font-mono font-medium text-text-secondary">
                Awaiting telemetry samples
              </div>
            )}
          </div>
        </div>

        {/* 4. Top Processes & Footprint Table */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="lg:col-span-2 p-5 rounded-2xl bg-white/[0.03] border border-border-subtle">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold text-text-primary">Top resource consumers</h3>
              <span className="text-xs font-medium text-text-secondary">Auto-tagged Starship sub-apps</span>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-border-subtle text-text-secondary font-semibold">
                    <th className="py-2.5">Process</th>
                    <th className="py-2.5">PID</th>
                    <th className="py-2.5">CPU %</th>
                    <th className="py-2.5">Memory</th>
                    <th className="py-2.5 text-right">Tag</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle">
                  {sample?.topProcesses && sample.topProcesses.length > 0 ? (
                    sample.topProcesses.map((p) => {
                      const subAppMatch = p.name.toLowerCase();
                      let tagColor = "bg-accent-muted text-accent border-accent/30";
                      let tagLabel = "Starship app";
                      if (subAppMatch.includes("easy-edits") || subAppMatch.includes("eep")) {
                        tagColor = "bg-blue-500/15 text-blue-400 border-blue-500/30";
                        tagLabel = "Easy Edits";
                      } else if (subAppMatch.includes("screen-studio") || subAppMatch.includes("ssp")) {
                        tagColor = "bg-amber-500/15 text-amber-400 border-amber-500/30";
                        tagLabel = "ScreenStudio";
                      } else if (subAppMatch.includes("audio-labs") || subAppMatch.includes("alp")) {
                        tagColor = "bg-emerald-500/15 text-emerald-400 border-emerald-500/30";
                        tagLabel = "Audio Labs";
                      } else if (subAppMatch.includes("net-watch") || subAppMatch.includes("nwp")) {
                        tagColor = "bg-cyan-500/15 text-cyan-400 border-cyan-500/30";
                        tagLabel = "Net Watch";
                      } else if (subAppMatch.includes("agentic-tuneup") || subAppMatch.includes("atp")) {
                        tagColor = "bg-purple-500/15 text-purple-400 border-purple-500/30";
                        tagLabel = "Tuneup Pro";
                      }

                      return (
                        <tr
                          key={p.pid}
                          className={`transition-colors ${
                            p.isStarshipOwned
                              ? "bg-accent-muted/20 text-text-primary"
                              : "text-text-secondary hover:bg-white/[0.03]"
                          }`}
                        >
                          <td className="py-2.5 font-mono font-medium text-text-primary">{p.name}</td>
                          <td className="py-2.5 font-mono text-text-secondary tabular">{p.pid}</td>
                          <td className="py-2.5 font-medium tabular text-text-primary">
                            <div className="flex items-center gap-2">
                              <span>{p.cpuPct.toFixed(1)}%</span>
                              <div className="w-12 h-1 rounded-full bg-border-subtle overflow-hidden">
                                <div
                                  className="h-full bg-sky-400 rounded-full"
                                  style={{ width: `${Math.min(100, Math.max(4, p.cpuPct))}%` }}
                                />
                              </div>
                            </div>
                          </td>
                          <td className="py-2.5 font-mono text-text-secondary tabular font-medium">
                            {(p.memBytes / 1024 / 1024).toFixed(0)} MB
                          </td>
                          <td className="py-2.5 text-right">
                            {p.isStarshipOwned ? (
                              <span className={`inline-flex items-center px-2 py-0.5 rounded-lg text-[11px] font-semibold border ${tagColor}`}>
                                {tagLabel}
                              </span>
                            ) : (
                              <span className="text-text-secondary text-[11px] font-medium">Host system</span>
                            )}
                          </td>
                        </tr>
                      );
                    })
                  ) : (
                    <tr>
                      <td colSpan={5} className="py-4 text-center text-text-secondary font-medium">
                        Gathering process metrics
                      </td>
                    </tr>
                  )}

                  {/* Starship Footprint Total Row */}
                  {sample?.ownFootprint && (
                    <tr className="border-t border-border-glass bg-white/[0.03] font-semibold text-text-primary">
                      <td className="py-2.5">Starship total footprint</td>
                      <td className="py-2.5 font-mono text-text-secondary tabular font-medium">
                        {sample.ownFootprint.processCount} procs
                      </td>
                      <td className="py-2.5 tabular text-sky-400">{sample.ownFootprint.totalCpuPct.toFixed(1)}%</td>
                      <td className="py-2.5 font-mono tabular text-purple-400">
                        {(sample.ownFootprint.totalRssBytes / 1024 / 1024).toFixed(0)} MB
                      </td>
                      <td className="py-2.5 text-right text-[11px] text-text-secondary font-medium">Combined</td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* 5. Alerts Timeline Drawer */}
          <div className="p-5 rounded-2xl bg-white/[0.03] border border-border-subtle flex flex-col">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-semibold text-text-primary">Governor alerts</h3>
              <span className="text-xs font-medium text-text-secondary">{alerts.length} logged</span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 max-h-[300px] pr-1">
              {alerts.length > 0 ? (
                alerts.map((a, idx) => (
                  <div
                    key={idx}
                    className="p-3 rounded-xl bg-white/[0.03] border border-border-subtle text-xs space-y-1"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-mono text-[11px] font-semibold text-text-primary">
                        {a.signal}
                      </span>
                      <span className="text-[11px] text-signal-thinking font-medium capitalize">
                        {a.action}
                      </span>
                    </div>
                    <div className="flex items-center justify-between text-text-secondary text-[11px] font-medium">
                      <span>Level: {a.level}</span>
                      <span className="tabular">Value: {a.value.toFixed(1)}</span>
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center h-32 text-center text-text-secondary text-xs">
                  <CheckCircle2 className="h-5 w-5 text-signal-success mb-2" />
                  <span className="font-medium text-text-primary">No overload alerts tripped</span>
                  <span className="text-[11px] text-text-secondary mt-1">
                    System operating within normal parameters
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* 6. Settings Drawer for Thresholds */}
      {isSettingsOpen && thresholds && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <div className="w-full max-w-lg rounded-2xl bg-raised border border-border-subtle p-6 space-y-5">
            <div className="flex items-center justify-between border-b border-border-subtle pb-3">
              <div className="flex items-center gap-2">
                <Sliders className="h-4 w-4 text-text-secondary" />
                <h3 className="text-sm font-medium text-text-primary">Overload detector thresholds</h3>
              </div>
              <button
                onClick={() => setIsSettingsOpen(false)}
                className="text-text-secondary hover:text-text-primary p-1 rounded-lg cursor-pointer transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            <div className="space-y-4 text-xs text-text-secondary max-h-[60vh] overflow-y-auto pr-1">
              <div>
                <label className="block text-text-secondary mb-1">CPU elevated threshold (%)</label>
                <input
                  type="number"
                  value={thresholds.cpuElevatedPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, cpuElevatedPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.03] border border-border-subtle px-3 py-2 text-text-primary focus:border-accent/40 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-text-secondary mb-1">CPU critical threshold (%)</label>
                <input
                  type="number"
                  value={thresholds.cpuCriticalPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, cpuCriticalPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.03] border border-border-subtle px-3 py-2 text-text-primary focus:border-accent/40 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-text-secondary mb-1">Memory available critical (%)</label>
                <input
                  type="number"
                  value={thresholds.memAvailCriticalPct}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, memAvailCriticalPct: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.03] border border-border-subtle px-3 py-2 text-text-primary focus:border-accent/40 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-text-secondary mb-1">Thermal critical delta (°C)</label>
                <input
                  type="number"
                  value={thresholds.thermalDeltaCritical}
                  onChange={(e) =>
                    setThresholds({ ...thresholds, thermalDeltaCritical: parseFloat(e.target.value) || 0 })
                  }
                  className="w-full rounded-xl bg-white/[0.03] border border-border-subtle px-3 py-2 text-text-primary focus:border-accent/40 focus:outline-none"
                />
              </div>

              <div>
                <label className="block text-text-secondary mb-1">Anti-flapping exit counter (samples)</label>
                <input
                  type="number"
                  value={thresholds.exitConsecutiveSamples}
                  onChange={(e) =>
                    setThresholds({
                      ...thresholds,
                      exitConsecutiveSamples: parseInt(e.target.value, 10) || 30,
                    })
                  }
                  className="w-full rounded-xl bg-white/[0.03] border border-border-subtle px-3 py-2 text-text-primary focus:border-accent/40 focus:outline-none"
                />
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-3 border-t border-border-subtle">
              <button
                onClick={() => setIsSettingsOpen(false)}
                className="px-4 py-2 rounded-xl text-xs text-text-secondary hover:text-text-primary cursor-pointer transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveThresholds}
                disabled={savingThresholds}
                className="px-4 py-2 rounded-xl text-xs font-medium bg-accent hover:bg-accent-hover text-accent-text transition-colors cursor-pointer disabled:opacity-50"
              >
                {savingThresholds ? "Saving" : "Save thresholds"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
