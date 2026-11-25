import React, { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  Sliders,
  CheckCircle2,
  Cpu,
  Layers,
  Tv,
  Flame,
  HardDrive,
  Thermometer,
} from "lucide-react";
import {
  LinearCard,
  LinearButton,
  LinearDialog,
  RangeTabs,
  StatusPill,
  LINEAR_COLORS,
} from "../linear";
import { LinearTelemetryChart } from "../common/LinearTelemetryChart";
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

  const stateBadge = {
    normal: {
      label: "Normal",
      status: "online" as const,
    },
    elevated: {
      label: "Elevated",
      status: "warning" as const,
    },
    constrained: {
      label: "Constrained",
      status: "warning" as const,
    },
    critical: {
      label: "Critical",
      status: "alert" as const,
    },
  }[state];

  const statCards = [
    {
      label: "CPU Load",
      value: sample?.cpu ? `${Math.round(sample.cpu.globalUtilPct)}%` : "0%",
      sub: sample?.cpu?.avgFreqMhz ? `${sample.cpu.avgFreqMhz} MHz` : "1.0 GHz",
      percent: sample?.cpu ? Math.round(sample.cpu.globalUtilPct) : 0,
      icon: Cpu,
      color: "text-[var(--accent-sky)]",
      bgColor: "bg-[var(--tint-sky)]",
      barColor: "bg-[var(--accent-sky)]",
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
      color: "text-[var(--accent-purple)]",
      bgColor: "bg-[var(--tint-purple)]",
      barColor: "bg-[var(--accent-purple)]",
    },
    {
      label: "GPU Compute",
      value: sample?.gpu ? `${Math.round(sample.gpu.utilPct)}%` : "0%",
      sub: sample?.gpu?.deviceName || "Hardware Acceleration",
      percent: sample?.gpu ? Math.round(sample.gpu.utilPct) : 0,
      icon: Tv,
      color: "text-[var(--accent-teal)]",
      bgColor: "bg-[var(--tint-teal)]",
      barColor: "bg-[var(--accent-teal)]",
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
      color: "text-[var(--accent-amber)]",
      bgColor: "bg-[var(--tint-amber)]",
      barColor: "bg-[var(--accent-amber)]",
    },
    {
      label: "Disk Free",
      value: sample?.disk ? `${Math.round(sample.disk.freePct)}%` : "N/A",
      sub: sample?.disk
        ? `${(sample.disk.freeBytes / 1024 / 1024 / 1024).toFixed(0)} GB free`
        : "Root partition",
      percent: sample?.disk ? Math.round(sample.disk.freePct) : 100,
      icon: HardDrive,
      color: "text-[var(--accent-green)]",
      bgColor: "bg-[var(--tint-green)]",
      barColor: "bg-[var(--accent-green)]",
    },
    {
      label: "Thermal Core",
      value: sample?.thermal ? `${Math.round(sample.thermal.maxTempCelsius)}°C` : "Normal",
      sub: sample?.thermal?.fanRpm ? `${sample.thermal.fanRpm} RPM` : "Within safe margin",
      percent: sample?.thermal
        ? Math.min(100, Math.round((sample.thermal.maxTempCelsius / 100) * 100))
        : 35,
      icon: Thermometer,
      color: "text-[var(--accent-red)]",
      bgColor: "bg-[var(--tint-red)]",
      barColor: "bg-[var(--accent-red)]",
    },
  ];

  const recentSamples =
    historySamples.length > 1
      ? historySamples.slice(-24)
      : sample
      ? [sample, sample]
      : [];
  const cpuSeries = recentSamples.map((s) =>
    Math.round(s.cpu?.globalUtilPct ?? 0)
  );
  const memSeries = recentSamples.map((s) =>
    Math.round(s.memory ? 100 - s.memory.availablePct : 0)
  );
  const timeLabels = recentSamples.map((s, idx) =>
    idx === recentSamples.length - 1
      ? "now"
      : new Date(s.timestampMs).toLocaleTimeString([], {
          minute: "2-digit",
          second: "2-digit",
        })
  );

  return (
    <div className="flex-1 flex flex-col h-full bg-[var(--bg-app)] text-[var(--text)] overflow-y-auto p-5 select-none font-sans">
      <div className="max-w-7xl mx-auto w-full space-y-4">
        <LinearCard level={1} className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-[6px] bg-[var(--tint-blue)] text-[var(--accent-blue)]">
              <Activity className="size-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-[16px] font-semibold text-[var(--text)]">
                  System health and resource governor
                </h2>
                <StatusPill label={stateBadge.label} status={stateBadge.status} />
              </div>
              <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                {directive?.actionMessage || "Real-time system monitoring and overload protection"}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <RangeTabs
              options={["15m", "1h", "24h"]}
              active={selectedRange}
              onChange={(r) => setSelectedRange(r as "15m" | "1h" | "24h")}
            />

            <LinearButton
              variant="secondary"
              size="sm"
              onClick={() => setIsSettingsOpen(true)}
            >
              <Sliders className="size-3.5" />
              <span>Thresholds</span>
            </LinearButton>
          </div>
        </LinearCard>

        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {statCards.map((card) => {
            const Icon = card.icon;
            return (
              <LinearCard
                key={card.label}
                level={1}
                className="p-4 flex flex-col justify-between"
              >
                <div className="flex items-center justify-between">
                  <span className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
                    {card.label}
                  </span>
                  <div className={`flex size-6 items-center justify-center rounded-[6px] ${card.bgColor} ${card.color}`}>
                    <Icon className="size-3.5" />
                  </div>
                </div>
                <div className="my-2.5">
                  <div className="text-[24px] font-semibold tabular-nums text-[var(--text)] tracking-tight leading-none">
                    {card.value}
                  </div>
                  <div className="w-full h-1 rounded-[2px] bg-[var(--bg-bar-idle)] mt-3 overflow-hidden">
                    <div
                      className={`h-full rounded-[2px] ${card.barColor} transition-all duration-150`}
                      style={{ width: `${Math.max(4, Math.min(100, card.percent))}%` }}
                    />
                  </div>
                  <div className="text-[12px] text-[var(--text-muted)] mt-2 truncate font-mono" title={card.sub}>
                    {card.sub}
                  </div>
                </div>
              </LinearCard>
            );
          })}
        </div>

        <LinearCard level={1} className="p-5">
          {recentSamples.length > 1 ? (
            <LinearTelemetryChart
              title="System telemetry"
              subtitle={`Continuous 1 Hz readings · last ${selectedRange}`}
              timestamps={timeLabels}
              unit="%"
              height="h-64"
              series={[
                { name: "CPU Utilization", data: cpuSeries, color: LINEAR_COLORS.sky },
                { name: "Memory Usage", data: memSeries, color: LINEAR_COLORS.purple },
              ]}
            />
          ) : (
            <div className="flex h-56 items-center justify-center text-[12px] font-mono text-[var(--text-muted)]">
              Awaiting telemetry samples
            </div>
          )}
        </LinearCard>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <LinearCard level={1} className="lg:col-span-2 p-5">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-[14px] font-semibold text-[var(--text)]">Top resource consumers</h3>
              <span className="text-[12px] text-[var(--text-muted)]">Auto-tagged Starship sub-apps</span>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left text-[12px] border-collapse">
                <thead>
                  <tr className="border-b border-[var(--border)] text-[var(--text-muted)] uppercase tracking-wider">
                    <th className="py-2.5 font-medium">Process</th>
                    <th className="py-2.5 font-medium">PID</th>
                    <th className="py-2.5 font-medium">CPU %</th>
                    <th className="py-2.5 font-medium">Memory</th>
                    <th className="py-2.5 font-medium text-right">Tag</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--border)]">
                  {sample?.topProcesses && sample.topProcesses.length > 0 ? (
                    sample.topProcesses.map((p) => {
                      const subAppMatch = p.name.toLowerCase();
                      let tagColor = "bg-[var(--tint-blue)] text-[var(--accent-blue)] border-[var(--accent-blue)]";
                      let tagLabel = "Starship app";
                      if (subAppMatch.includes("easy-edits") || subAppMatch.includes("eep")) {
                        tagColor = "bg-[var(--tint-blue)] text-[var(--accent-blue)] border-[var(--accent-blue)]";
                        tagLabel = "Easy Edits";
                      } else if (subAppMatch.includes("screen-studio") || subAppMatch.includes("ssp")) {
                        tagColor = "bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]";
                        tagLabel = "ScreenStudio";
                      } else if (subAppMatch.includes("audio-labs") || subAppMatch.includes("alp")) {
                        tagColor = "bg-[var(--tint-green)] text-[var(--accent-green)] border-[var(--accent-green)]";
                        tagLabel = "Audio Labs";
                      } else if (subAppMatch.includes("net-watch") || subAppMatch.includes("nwp")) {
                        tagColor = "bg-[var(--tint-sky)] text-[var(--accent-sky)] border-[var(--accent-sky)]";
                        tagLabel = "Net Watch";
                      } else if (subAppMatch.includes("agentic-tuneup") || subAppMatch.includes("atp")) {
                        tagColor = "bg-[var(--tint-purple)] text-[var(--accent-purple)] border-[var(--accent-purple)]";
                        tagLabel = "Tuneup Pro";
                      }

                      return (
                        <tr
                          key={p.pid}
                          className={`transition-colors ${
                            p.isStarshipOwned
                              ? "bg-[var(--tint-blue)] text-[var(--text)]"
                              : "text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)]"
                          }`}
                        >
                          <td className="py-2.5 font-mono font-medium text-[var(--text)]">{p.name}</td>
                          <td className="py-2.5 font-mono text-[var(--text-muted)] tabular-nums">{p.pid}</td>
                          <td className="py-2.5 font-medium tabular-nums text-[var(--text)]">
                            <div className="flex items-center gap-2">
                              <span>{p.cpuPct.toFixed(1)}%</span>
                              <div className="w-12 h-1 rounded-[2px] bg-[var(--bg-bar-idle)] overflow-hidden">
                                <div
                                  className="h-full bg-[var(--accent-sky)] rounded-[2px]"
                                  style={{ width: `${Math.min(100, Math.max(4, p.cpuPct))}%` }}
                                />
                              </div>
                            </div>
                          </td>
                          <td className="py-2.5 font-mono text-[var(--text-muted)] tabular-nums">
                            {(p.memBytes / 1024 / 1024).toFixed(0)} MB
                          </td>
                          <td className="py-2.5 text-right">
                            {p.isStarshipOwned ? (
                              <span className={`inline-flex items-center px-2 py-0.5 rounded-[4px] text-[12px] font-semibold border ${tagColor}`}>
                                {tagLabel}
                              </span>
                            ) : (
                              <span className="text-[var(--text-muted)] text-[12px]">Host system</span>
                            )}
                          </td>
                        </tr>
                      );
                    })
                  ) : (
                    <tr>
                      <td colSpan={5} className="py-4 text-center text-[var(--text-muted)]">
                        Gathering process metrics
                      </td>
                    </tr>
                  )}

                  {sample?.ownFootprint && (
                    <tr className="border-t border-[var(--border-strong)] bg-[var(--bg-surface-2)] font-semibold text-[var(--text)]">
                      <td className="py-2.5">Starship total footprint</td>
                      <td className="py-2.5 font-mono text-[var(--text-muted)] tabular-nums">
                        {sample.ownFootprint.processCount} procs
                      </td>
                      <td className="py-2.5 tabular-nums text-[var(--accent-sky)]">
                        {sample.ownFootprint.totalCpuPct.toFixed(1)}%
                      </td>
                      <td className="py-2.5 font-mono tabular-nums text-[var(--accent-purple)]">
                        {(sample.ownFootprint.totalRssBytes / 1024 / 1024).toFixed(0)} MB
                      </td>
                      <td className="py-2.5 text-right text-[12px] text-[var(--text-muted)]">Combined</td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </LinearCard>

          <LinearCard level={1} className="p-5 flex flex-col">
            <div className="flex items-center justify-between mb-3">
              <h3 className="text-[14px] font-semibold text-[var(--text)]">Governor alerts</h3>
              <span className="text-[12px] text-[var(--text-muted)]">{alerts.length} logged</span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 max-h-[300px] pr-1">
              {alerts.length > 0 ? (
                alerts.map((a, idx) => (
                  <div
                    key={idx}
                    className="p-3 rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] text-[12px] space-y-1"
                  >
                    <div className="flex items-center justify-between">
                      <span className="font-mono font-semibold text-[var(--text)]">{a.signal}</span>
                      <span className="text-[var(--accent-amber)] font-medium capitalize">{a.action}</span>
                    </div>
                    <div className="flex items-center justify-between text-[var(--text-muted)]">
                      <span>Level: {a.level}</span>
                      <span className="tabular-nums">Value: {a.value.toFixed(1)}</span>
                    </div>
                  </div>
                ))
              ) : (
                <div className="flex flex-col items-center justify-center h-32 text-center text-[var(--text-muted)] text-[12px]">
                  <CheckCircle2 className="size-5 text-[var(--accent-green)] mb-2" />
                  <span className="font-medium text-[var(--text)]">No overload alerts tripped</span>
                  <span className="mt-1">System operating within normal parameters</span>
                </div>
              )}
            </div>
          </LinearCard>
        </div>
      </div>

      {thresholds && (
        <LinearDialog
          open={isSettingsOpen}
          onClose={() => setIsSettingsOpen(false)}
          title="Overload detector thresholds"
          icon={<Sliders className="size-4 text-[var(--text-muted)]" />}
          footer={
            <>
              <span className="text-[12px] text-[var(--text-muted)] font-mono">
                Hysteresis guard
              </span>
              <div className="flex items-center gap-2">
                <LinearButton variant="secondary" size="sm" onClick={() => setIsSettingsOpen(false)}>
                  Cancel
                </LinearButton>
                <LinearButton
                  variant="primary"
                  size="sm"
                  onClick={handleSaveThresholds}
                  disabled={savingThresholds}
                >
                  {savingThresholds ? "Saving" : "Save thresholds"}
                </LinearButton>
              </div>
            </>
          }
        >
          <div className="space-y-4 text-[12px] text-[var(--text-muted)] max-h-[60vh] overflow-y-auto pr-1">
            <div>
              <label className="block mb-1">CPU elevated threshold (%)</label>
              <input
                type="number"
                value={thresholds.cpuElevatedPct}
                onChange={(e) =>
                  setThresholds({ ...thresholds, cpuElevatedPct: parseFloat(e.target.value) || 0 })
                }
                className="w-full rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] px-3 py-2 text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>

            <div>
              <label className="block mb-1">CPU critical threshold (%)</label>
              <input
                type="number"
                value={thresholds.cpuCriticalPct}
                onChange={(e) =>
                  setThresholds({ ...thresholds, cpuCriticalPct: parseFloat(e.target.value) || 0 })
                }
                className="w-full rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] px-3 py-2 text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>

            <div>
              <label className="block mb-1">Memory available critical (%)</label>
              <input
                type="number"
                value={thresholds.memAvailCriticalPct}
                onChange={(e) =>
                  setThresholds({ ...thresholds, memAvailCriticalPct: parseFloat(e.target.value) || 0 })
                }
                className="w-full rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] px-3 py-2 text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>

            <div>
              <label className="block mb-1">Thermal critical delta (°C)</label>
              <input
                type="number"
                value={thresholds.thermalDeltaCritical}
                onChange={(e) =>
                  setThresholds({ ...thresholds, thermalDeltaCritical: parseFloat(e.target.value) || 0 })
                }
                className="w-full rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] px-3 py-2 text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>

            <div>
              <label className="block mb-1">Anti-flapping exit counter (samples)</label>
              <input
                type="number"
                value={thresholds.exitConsecutiveSamples}
                onChange={(e) =>
                  setThresholds({
                    ...thresholds,
                    exitConsecutiveSamples: parseInt(e.target.value, 10) || 30,
                  })
                }
                className="w-full rounded-[6px] bg-[var(--bg-surface-2)] border border-[var(--border)] px-3 py-2 text-[var(--text)] focus-ring focus:border-[var(--border-strong)]"
              />
            </div>
          </div>
        </LinearDialog>
      )}
    </div>
  );
};
