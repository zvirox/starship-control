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

  // Compact, neutral stat cards — big tabular number, label, one quiet sub-metric.
  // The telemetry chart below is the only rich data moment.
  const statCards = [
    {
      label: "CPU",
      value: sample?.cpu ? `${Math.round(sample.cpu.globalUtilPct)}%` : "0%",
      sub: sample?.cpu?.avgFreqMhz ? `${sample.cpu.avgFreqMhz} MHz` : "1.0 Hz",
    },
    {
      label: "Memory",
      value: sample?.memory ? `${Math.round(100 - sample.memory.availablePct)}%` : "0%",
      sub: sample?.memory
        ? `${(sample.memory.usedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
            sample.memory.totalBytes / 1024 / 1024 / 1024
          ).toFixed(1)} GB`
        : "0 GB",
    },
    {
      label: "GPU",
      value: sample?.gpu ? `${Math.round(sample.gpu.utilPct)}%` : "0%",
      sub: sample?.gpu?.deviceName || "Integrated Graphics",
    },
    {
      label: "VRAM",
      value: sample?.gpu?.vramTotalBytes ? `${Math.round(sample.gpu.vramUsedPct)}%` : "N/A",
      sub: sample?.gpu?.vramTotalBytes
        ? `${(sample.gpu.vramUsedBytes / 1024 / 1024 / 1024).toFixed(1)} / ${(
            sample.gpu.vramTotalBytes / 1024 / 1024 / 1024
          ).toFixed(1)} GB`
        : "Shared system memory",
    },
    {
      label: "Disk free",
      value: sample?.disk ? `${Math.round(sample.disk.freePct)}%` : "N/A",
      sub: sample?.disk
        ? `${(sample.disk.freeBytes / 1024 / 1024 / 1024).toFixed(0)} GB free`
        : "Root partition",
    },
    {
      label: "Thermal",
      value: sample?.thermal ? `${Math.round(sample.thermal.maxTempCelsius)}°C` : "N/A",
      sub: sample?.thermal?.fanRpm ? `${sample.thermal.fanRpm} RPM` : "Δ to max: 25°C",
    },
  ];

  // Telemetry chart geometry — muted grid, one accent CPU trace, one neutral memory trace.
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
                <h2 className="text-base font-medium text-text-primary">
                  System health and resource governor
                </h2>
                <div
                  className={`flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs font-medium border ${stateBadge.className}`}
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
                  className={`px-3 py-1 rounded-lg transition-colors cursor-pointer ${
                    selectedRange === r
                      ? "bg-accent-muted text-accent font-medium"
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

        {/* 2. Neutral stat cards — the chart, not these, owns the richness */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {statCards.map((card) => (
            <div
              key={card.label}
              className="p-4 rounded-xl bg-white/[0.03] border border-border-subtle flex flex-col justify-between"
            >
              <div className="text-xs text-text-secondary">{card.label}</div>
              <div className="my-3">
                <div className="text-2xl font-medium tabular text-text-primary">{card.value}</div>
                <div className="text-[11px] text-text-disabled mt-1 truncate" title={card.sub}>
                  {card.sub}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* 3. System telemetry — the single rich data moment */}
        <div className="p-5 rounded-2xl bg-white/[0.03] border border-border-subtle">
          <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
            <div>
              <h3 className="text-sm font-medium text-text-primary">System telemetry</h3>
              <p className="text-xs text-text-secondary mt-1">
                Continuous 1 Hz readings · last {selectedRange}
              </p>
            </div>
            <div className="flex items-center gap-4 text-[11px] text-text-secondary">
              <span className="flex items-center gap-2">
                <span className="h-0.5 w-4 rounded-full bg-accent" />
                CPU
              </span>
              <span className="flex items-center gap-2">
                <span className="w-4 border-t border-dashed border-text-secondary" />
                Memory
              </span>
            </div>
          </div>

          <div className="relative h-64 w-full">
            <svg
              className="w-full h-full overflow-visible"
              viewBox={`0 0 ${chart.width} ${chart.height}`}
              preserveAspectRatio="none"
            >
              {/* Muted hairline grid */}
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
                    fill="var(--color-text-disabled)"
                    className="font-mono tabular"
                    fontSize="11"
                  >
                    {tick}
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
                      fill="var(--color-text-disabled)"
                      className="font-mono"
                      fontSize="11"
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

              {/* One accent CPU trace */}
              {historySamples.length > 1 && (
                <polyline
                  fill="none"
                  className="text-accent"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  points={historySamples
                    .map((s, idx) => `${xFor(idx, historySamples.length)},${yFor(s.cpu?.globalUtilPct || 0)}`)
                    .join(" ")}
                />
              )}

              {/* One neutral, dashed memory comparison */}
              {historySamples.length > 1 && (
                <polyline
                  fill="none"
                  className="text-text-secondary"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeDasharray="4 4"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  points={historySamples
                    .map((s, idx) =>
                      `${xFor(idx, historySamples.length)},${yFor(s.memory ? 100 - s.memory.availablePct : 0)}`
                    )
                    .join(" ")}
                />
              )}
            </svg>
            {historySamples.length <= 1 && (
              <div className="absolute inset-0 flex items-center justify-center text-[11px] font-mono text-text-disabled">
                Awaiting telemetry samples
              </div>
            )}
          </div>
        </div>

        {/* 4. Top Processes & Footprint Table */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="lg:col-span-2 p-5 rounded-2xl bg-white/[0.03] border border-border-subtle">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-text-primary">Top resource consumers</h3>
              <span className="text-xs text-text-disabled">Auto-tagged Starship sub-apps</span>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-border-subtle text-text-disabled">
                    <th className="py-2 font-medium">Process</th>
                    <th className="py-2 font-medium">PID</th>
                    <th className="py-2 font-medium">CPU %</th>
                    <th className="py-2 font-medium">Memory</th>
                    <th className="py-2 font-medium text-right">Tag</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border-subtle">
                  {sample?.topProcesses && sample.topProcesses.length > 0 ? (
                    sample.topProcesses.map((p) => (
                      <tr
                        key={p.pid}
                        className={`transition-colors ${
                          p.isStarshipOwned
                            ? "bg-accent-muted text-text-primary"
                            : "text-text-secondary hover:bg-white/[0.03]"
                        }`}
                      >
                        <td className="py-2 font-mono font-medium">{p.name}</td>
                        <td className="py-2 font-mono text-text-disabled tabular">{p.pid}</td>
                        <td className="py-2 font-medium tabular">{p.cpuPct.toFixed(1)}%</td>
                        <td className="py-2 font-mono text-text-secondary tabular">
                          {(p.memBytes / 1024 / 1024).toFixed(0)} MB
                        </td>
                        <td className="py-2 text-right">
                          {p.isStarshipOwned ? (
                            <span className="inline-flex items-center px-2 py-0.5 rounded-lg text-[11px] font-medium bg-accent-muted text-accent border border-accent/30">
                              Starship app
                            </span>
                          ) : (
                            <span className="text-text-disabled text-[11px]">Host</span>
                          )}
                        </td>
                      </tr>
                    ))
                  ) : (
                    <tr>
                      <td colSpan={5} className="py-4 text-center text-text-disabled">
                        Gathering process metrics
                      </td>
                    </tr>
                  )}

                  {/* Starship Footprint Total Row */}
                  {sample?.ownFootprint && (
                    <tr className="border-t border-border-glass bg-white/[0.03] font-medium text-text-primary">
                      <td className="py-2">Starship total footprint</td>
                      <td className="py-2 font-mono text-text-secondary tabular">
                        {sample.ownFootprint.processCount} procs
                      </td>
                      <td className="py-2 tabular">{sample.ownFootprint.totalCpuPct.toFixed(1)}%</td>
                      <td className="py-2 font-mono tabular">
                        {(sample.ownFootprint.totalRssBytes / 1024 / 1024).toFixed(0)} MB
                      </td>
                      <td className="py-2 text-right text-[11px] text-text-secondary">Combined</td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* 5. Alerts Timeline Drawer */}
          <div className="p-5 rounded-2xl bg-white/[0.03] border border-border-subtle flex flex-col">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-sm font-medium text-text-primary">Governor alerts</h3>
              <span className="text-xs text-text-disabled">{alerts.length} logged</span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 max-h-[300px] pr-1">
              {alerts.length > 0 ? (
                alerts.map((a, idx) => (
                  <div
                    key={idx}
                    className="p-3 rounded-xl bg-white/[0.03] border border-border-subtle text-xs space-y-1"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-mono text-[11px] font-medium text-text-primary">
                        {a.signal}
                      </span>
                      <span className="text-[11px] text-signal-thinking capitalize">
                        {a.action}
                      </span>
                    </div>
                    <div className="flex items-center justify-between text-text-secondary text-[11px]">
                      <span>Level: {a.level}</span>
                      <span className="tabular">Value: {a.value.toFixed(1)}</span>
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center h-32 text-center text-text-secondary text-xs">
                  <CheckCircle2 className="h-5 w-5 text-signal-success mb-2" />
                  <span>No overload alerts tripped</span>
                  <span className="text-[11px] text-text-disabled mt-1">
                    System operating within thresholds
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
