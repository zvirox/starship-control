import React, { useEffect, useMemo, useRef, useState } from "react";
import * as echarts from "echarts";
import { invoke } from "@tauri-apps/api/core";
import { Cpu, Zap, HardDrive, ShieldAlert } from "lucide-react";
import {
  LinearCard,
  StatCard,
  RangeTabs,
  DonutLegend,
  ProgressRow,
  DataTable,
  StatusPill,
  LinearButton,
  LINEAR_COLORS,
  getLinearEChartsOption,
  createLinearAreaGradient,
  type DataTableColumn,
} from "../../components/linear";
import { useThemeStore } from "../../stores/themeStore";
import type {
  ModelRecord,
  MissionRecord,
  PlanActiveResponse,
  ApprovalNeededEvent,
  HealthSnapshotResponse,
} from "../../types/ipc";

interface QueueRow {
  id: string;
  task: string;
  app: string;
  status: "online" | "active" | "warning" | "alert" | "idle";
  statusLabel: string;
  tier: string;
  latency: string;
}

export interface CockpitDashboardProps {
  models: ModelRecord[];
  missions: MissionRecord[];
  activePlan: PlanActiveResponse | null;
  approvalGate: ApprovalNeededEvent | null;
  onNavigateTab: (
    tab:
      | "overview"
      | "chat"
      | "timeline"
      | "history"
      | "workflows"
      | "apps"
      | "tools"
      | "models"
      | "systems"
      | "activity"
  ) => void;
}

const THROUGHPUT_SERIES_BY_RANGE: Record<
  string,
  { labels: string[]; main: number[]; secondary: number[] }
> = {
  "1W": {
    labels: ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    main: [44, 52, 49, 61, 58, 67, 74],
    secondary: [28, 33, 31, 39, 36, 44, 48],
  },
  "1M": {
    labels: ["W1", "W2", "W3", "W4"],
    main: [38, 49, 57, 71],
    secondary: [24, 31, 38, 46],
  },
  "3W": {
    labels: ["D1", "D4", "D7", "D10", "D14", "D18", "D21"],
    main: [41, 46, 53, 50, 62, 66, 72],
    secondary: [25, 29, 34, 32, 41, 43, 49],
  },
  YTD: {
    labels: ["Feb", "Apr", "Jun", "Aug", "Oct", "Nov"],
    main: [22, 31, 42, 54, 65, 74],
    secondary: [15, 21, 29, 35, 42, 48],
  },
  Total: {
    labels: ["Q1", "Q2", "Q3", "Q4"],
    main: [29, 45, 59, 74],
    secondary: [18, 28, 39, 48],
  },
};

export const CockpitDashboard: React.FC<CockpitDashboardProps> = ({
  models,
  missions,
  activePlan,
  approvalGate,
  onNavigateTab,
}) => {
  const theme = useThemeStore((s) => s.theme);
  const isDark = theme === "dark";

  const [range, setRange] = useState<string>("1W");
  const [health, setHealth] = useState<HealthSnapshotResponse | null>(null);
  const [activeBarIdx, setActiveBarIdx] = useState<number>(4);

  const lineChartRef = useRef<HTMLDivElement>(null);
  const barChartRef = useRef<HTMLDivElement>(null);
  const donutChartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    async function loadHealth() {
      try {
        const snap = await invoke<HealthSnapshotResponse>(
          "cmd_health_get_snapshot"
        );
        if (!cancelled) setHealth(snap);
      } catch {
        // Ignore outside Tauri runtime
      }
    }
    loadHealth();
    const timer = setInterval(loadHealth, 3000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  const readyModelsCount = models.filter(
    (m) => m.download_status === "ready"
  ).length;
  const loadingModelsCount = models.filter(
    (m) => m.download_status === "downloading"
  ).length;
  const errorModelsCount = models.filter(
    (m) => m.download_status === "failed"
  ).length;
  const idleModelsCount = Math.max(
    1,
    models.length - readyModelsCount - loadingModelsCount - errorModelsCount
  );

  const pendingApprovals = useMemo(() => {
    const planPending =
      activePlan?.steps.filter(
        (s) => s.riskTier >= 2 && s.approvalStatus === "pending"
      ).length ?? 0;
    return Math.max(planPending, approvalGate ? 1 : 0);
  }, [activePlan, approvalGate]);

  const cpuPct = Math.round(health?.sample?.cpu?.globalUtilPct ?? 18);
  const ramUsedPct = Math.round(
    health?.sample?.memory ? 100 - health.sample.memory.availablePct : 44
  );
  const vramPct = Math.round(
    health?.sample?.gpu?.vramUsedPct ?? 45
  );

  const rangeSeries =
    THROUGHPUT_SERIES_BY_RANGE[range] ?? THROUGHPUT_SERIES_BY_RANGE["1W"];

  // 1. Hub Throughput Line Chart (Blue main, Sky secondary, gradient fade under line)
  useEffect(() => {
    if (!lineChartRef.current) return;
    const chart = echarts.init(lineChartRef.current);
    const base = getLinearEChartsOption(isDark);
    chart.setOption({
      ...base,
      tooltip: { ...base.tooltip, trigger: "axis" },
      xAxis: {
        ...base.xAxis,
        type: "category",
        boundaryGap: false,
        data: rangeSeries.labels,
      },
      yAxis: {
        ...base.yAxis,
        type: "value",
        axisLabel: { ...base.yAxis.axisLabel, formatter: "{value} t/s" },
      },
      series: [
        {
          name: "Local Hub",
          type: "line",
          smooth: true,
          showSymbol: false,
          data: rangeSeries.main,
          lineStyle: { width: 2, color: LINEAR_COLORS.blue },
          itemStyle: { color: LINEAR_COLORS.blue },
          areaStyle: {
            color: createLinearAreaGradient(LINEAR_COLORS.blue, 0.2),
          },
        },
        {
          name: "Cloud Relay",
          type: "line",
          smooth: true,
          showSymbol: false,
          data: rangeSeries.secondary,
          lineStyle: { width: 2, color: LINEAR_COLORS.sky },
          itemStyle: { color: LINEAR_COLORS.sky },
          areaStyle: {
            color: createLinearAreaGradient(LINEAR_COLORS.sky, 0.14),
          },
        },
      ],
    });
    const onResize = () => chart.resize();
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      chart.dispose();
    };
  }, [isDark, rangeSeries]);

  // 2. Inference Latency Bar Chart (idle bars muted, active bar blue + sky, bordered tooltip w/ green marker dot)
  useEffect(() => {
    if (!barChartRef.current) return;
    const chart = echarts.init(barChartRef.current);
    const base = getLinearEChartsOption(isDark);
    const idleFill = isDark
      ? LINEAR_COLORS.idleDark
      : LINEAR_COLORS.idleLight;
    const days = ["T-6", "T-5", "T-4", "T-3", "T-2", "T-1", "Now"];
    const primaryLatency = [19, 24, 21, 27, 16, 22, 18];
    const queueOverhead = [6, 8, 7, 9, 5, 7, 6];

    chart.setOption({
      ...base,
      tooltip: {
        ...base.tooltip,
        trigger: "axis",
        axisPointer: { type: "none" },
        formatter: (params: unknown) => {
          const items = Array.isArray(params) ? params : [];
          if (items.length === 0) return "";
          const first = items[0] as { name?: string };
          const rows = items
            .map((it) => {
              const p = it as { seriesName?: string; value?: number };
              return `<div style="display:flex;align-items:center;justify-content:space-between;gap:12px;margin-top:4px;"><span style="display:inline-flex;align-items:center;gap:6px;"><span style="width:6px;height:6px;border-radius:999px;background:${LINEAR_COLORS.green};display:inline-block;"></span>${p.seriesName}</span><strong>${p.value} ms</strong></div>`;
            })
            .join("");
          return `<div style="font-size:12px;"><div style="font-weight:600;margin-bottom:2px;">${first.name}</div>${rows}</div>`;
        },
      },
      xAxis: {
        ...base.xAxis,
        type: "category",
        data: days,
      },
      yAxis: {
        ...base.yAxis,
        type: "value",
        axisLabel: { ...base.yAxis.axisLabel, formatter: "{value}ms" },
      },
      series: [
        {
          name: "Inference",
          type: "bar",
          stack: "latency",
          barWidth: 16,
          data: primaryLatency.map((v, i) => ({
            value: v,
            itemStyle: {
              color: i === activeBarIdx ? LINEAR_COLORS.blue : idleFill,
              borderRadius: [0, 0, 4, 4],
            },
          })),
        },
        {
          name: "Queue",
          type: "bar",
          stack: "latency",
          barWidth: 16,
          data: queueOverhead.map((v, i) => ({
            value: v,
            itemStyle: {
              color: i === activeBarIdx ? LINEAR_COLORS.sky : idleFill,
              borderRadius: [4, 4, 0, 0],
            },
          })),
        },
      ],
    });

    chart.on("click", (params) => {
      if (typeof params.dataIndex === "number") {
        setActiveBarIdx(params.dataIndex);
      }
    });

    const onResize = () => chart.resize();
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      chart.dispose();
    };
  }, [isDark, activeBarIdx]);

  // 3. Model States Donut (rounded arc ends, thin tick ring, center total)
  const donutData = useMemo(
    () => [
      {
        label: "Running",
        value: Math.max(1, readyModelsCount),
        color: LINEAR_COLORS.green,
      },
      {
        label: "Loading",
        value: loadingModelsCount,
        color: LINEAR_COLORS.amber,
      },
      {
        label: "Idle",
        value: idleModelsCount,
        color: LINEAR_COLORS.sky,
      },
      {
        label: "Error",
        value: errorModelsCount,
        color: LINEAR_COLORS.red,
      },
    ],
    [readyModelsCount, loadingModelsCount, idleModelsCount, errorModelsCount]
  );

  const totalModelCount = donutData.reduce((acc, d) => acc + d.value, 0);

  useEffect(() => {
    if (!donutChartRef.current) return;
    const chart = echarts.init(donutChartRef.current);
    const base = getLinearEChartsOption(isDark);
    const borderCol = isDark
      ? LINEAR_COLORS.surfaceDark
      : LINEAR_COLORS.surfaceLight;
    const tickCol = isDark
      ? LINEAR_COLORS.borderStrongDark
      : LINEAR_COLORS.borderStrongLight;

    chart.setOption({
      backgroundColor: "transparent",
      tooltip: {
        ...base.tooltip,
        trigger: "item",
      },
      series: [
        {
          type: "gauge",
          radius: "92%",
          startAngle: 90,
          endAngle: -270,
          pointer: { show: false },
          progress: { show: false },
          axisLine: { show: false },
          splitLine: { show: false },
          axisLabel: { show: false },
          detail: { show: false },
          axisTick: {
            distance: -4,
            length: 4,
            splitNumber: 2,
            lineStyle: { color: tickCol, width: 1 },
          },
          silent: true,
        },
        {
          name: "Model states",
          type: "pie",
          radius: ["62%", "80%"],
          center: ["50%", "50%"],
          avoidLabelOverlap: false,
          padAngle: 3,
          itemStyle: {
            borderRadius: 6,
            borderColor: borderCol,
            borderWidth: 2,
          },
          label: { show: false },
          data: donutData.map((d) => ({
            name: d.label,
            value: d.value,
            itemStyle: { color: d.color },
          })),
        },
      ],
      graphic: [
        {
          type: "text",
          left: "center",
          top: "41%",
          style: {
            text: String(totalModelCount),
            textAlign: "center",
            fill: isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight,
            fontFamily: "Inter, system-ui, sans-serif",
            fontSize: 22,
            fontWeight: 600,
          },
        },
        {
          type: "text",
          left: "center",
          top: "56%",
          style: {
            text: "Models",
            textAlign: "center",
            fill: isDark
              ? LINEAR_COLORS.textMutedDark
              : LINEAR_COLORS.textMutedLight,
            fontFamily: "Inter, system-ui, sans-serif",
            fontSize: 12,
            fontWeight: 500,
          },
        },
      ],
    });
    const onResize = () => chart.resize();
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      chart.dispose();
    };
  }, [isDark, donutData, totalModelCount]);

  // 4. Resource Use per Model (Progress rows with green, amber, red thresholds)
  const modelResourceRows = useMemo(() => {
    if (models.length > 0) {
      return models.slice(0, 4).map((m, idx) => {
        const pct =
          m.provider_type === "local_gguf"
            ? Math.min(95, Math.max(24, vramPct + idx * 14))
            : 18 + idx * 12;
        return {
          name: m.name,
          pct,
          display:
            m.provider_type === "local_gguf"
              ? `${pct}% VRAM`
              : `${pct}% quota`,
        };
      });
    }
    return [
      { name: "Qwen 2.5 Coder 7B (Q4_K_M)", pct: 58, display: "58% VRAM" },
      { name: "Whisper v3 Large (ALP)", pct: 76, display: "76% VRAM" },
      { name: "Llama 3.2 Vision 11B", pct: 92, display: "92% VRAM" },
      { name: "Cloud Fallback SSE", pct: 24, display: "24% quota" },
    ];
  }, [models, vramPct]);

  // 5. Task & Approval Queue Table
  const queueRows: QueueRow[] = useMemo(() => {
    const rows: QueueRow[] = [];
    if (approvalGate) {
      rows.push({
        id: `gate-${approvalGate.stepId}`,
        task: approvalGate.diffSummary || approvalGate.actionTitle,
        app: approvalGate.appId.toUpperCase(),
        status: "alert",
        statusLabel: "Approval needed",
        tier: `Tier ${approvalGate.riskTier}`,
        latency: "Paused",
      });
    }
    if (activePlan) {
      for (const s of activePlan.steps.slice(0, 4)) {
        rows.push({
          id: `step-${s.stepId}`,
          task: s.toolName,
          app: s.appId.toUpperCase(),
          status:
            s.approvalStatus === "pending" && s.riskTier >= 2
              ? "alert"
              : s.executionStatus === "running"
              ? "active"
              : s.executionStatus === "success"
              ? "online"
              : s.executionStatus === "failed"
              ? "alert"
              : "idle",
          statusLabel:
            s.approvalStatus === "pending" && s.riskTier >= 2
              ? "Waiting approval"
              : s.executionStatus,
          tier: `Tier ${s.riskTier}`,
          latency: s.executionStatus === "success" ? "42 ms" : "Live",
        });
      }
    }
    if (rows.length === 0) {
      for (const m of missions.slice(0, 5)) {
        rows.push({
          id: `mission-${m.id}`,
          task: m.title || m.user_goal,
          app: "CORE",
          status:
            m.status === "completed"
              ? "online"
              : m.status === "failed"
              ? "alert"
              : "active",
          statusLabel: m.status,
          tier: "Orchestrator",
          latency: m.active_model_id,
        });
      }
    }
    return rows;
  }, [approvalGate, activePlan, missions]);

  const queueColumns: DataTableColumn<QueueRow>[] = [
    {
      key: "task",
      header: "Task / Operation",
      render: (r) => (
        <span className="font-medium text-[var(--text)] truncate block max-w-xs">
          {r.task}
        </span>
      ),
    },
    {
      key: "app",
      header: "Target",
      render: (r) => (
        <span className="font-mono text-[12px] text-[var(--text-muted)]">
          {r.app}
        </span>
      ),
    },
    {
      key: "status",
      header: "Status",
      render: (r) => <StatusPill label={r.statusLabel} status={r.status} />,
    },
    {
      key: "tier",
      header: "Policy",
      render: (r) => (
        <span className="font-mono text-[12px] text-[var(--text-muted)]">
          {r.tier}
        </span>
      ),
    },
    {
      key: "latency",
      header: "Detail",
      align: "right",
      render: (r) => (
        <span className="font-mono text-[12px] text-[var(--text-muted)] tabular-nums">
          {r.latency}
        </span>
      ),
    },
  ];

  return (
    <div className="flex-1 overflow-y-auto bg-[var(--bg-app)] p-5">
      <div className="mx-auto max-w-7xl space-y-4">
        {/* 4 Stat Cards (Models Running, Tokens/sec, GPU & RAM, Pending Approvals) */}
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
          <StatCard
            title="Models running"
            value={Math.max(1, readyModelsCount)}
            unit={`/ ${Math.max(1, models.length)} ready`}
            tone="green"
            icon={<Cpu className="size-4" strokeWidth={1.75} />}
            delta={{
              value: "+1",
              positive: true,
              comparison: "Vulkan active",
            }}
            sparkline={[1, 1, 2, 2, 2, 3, Math.max(1, readyModelsCount)]}
          />
          <StatCard
            title="Tokens per second"
            value="48.4"
            unit="tok/s"
            tone="blue"
            icon={<Zap className="size-4" strokeWidth={1.75} />}
            delta={{
              value: "+12.4%",
              positive: true,
              comparison: "vs 1h avg",
            }}
            sparkline={[36, 39, 42, 41, 45, 47, 48.4]}
          />
          <StatCard
            title="GPU & RAM use"
            value={`${vramPct}%`}
            unit={`RAM ${ramUsedPct}%`}
            tone="sky"
            icon={<HardDrive className="size-4" strokeWidth={1.75} />}
            delta={{
              value: `CPU ${cpuPct}%`,
              positive: vramPct < 85,
              comparison: "Nominal",
            }}
            sparkline={[38, 41, 44, 43, 46, 45, vramPct]}
          />
          <StatCard
            title="Pending approvals"
            value={pendingApprovals}
            unit="gates"
            tone={pendingApprovals > 0 ? "red" : "teal"}
            icon={<ShieldAlert className="size-4" strokeWidth={1.75} />}
            delta={{
              value: pendingApprovals > 0 ? "Review" : "Clear",
              positive: pendingApprovals === 0,
              comparison: "Safety Arbiter",
            }}
            sparkline={[2, 1, 1, 0, 1, 0, pendingApprovals]}
          />
        </div>

        {/* Charts Row: Hub Throughput (Line) + Inference Latency (Bar) */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <LinearCard level={1} className="lg:col-span-2 p-5 space-y-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h3 className="text-[16px] font-semibold text-[var(--text)]">
                  Hub throughput
                </h3>
                <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                  Token streaming velocity across local Vulkan and cloud relay
                </p>
              </div>
              <RangeTabs active={range} onChange={setRange} />
            </div>
            <div ref={lineChartRef} className="h-56 w-full" />
          </LinearCard>

          <LinearCard level={1} className="p-5 space-y-4">
            <div>
              <h3 className="text-[16px] font-semibold text-[var(--text)]">
                Inference latency
              </h3>
              <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                Click any bar to inspect breakdown (ms)
              </p>
            </div>
            <div ref={barChartRef} className="h-56 w-full" />
          </LinearCard>
        </div>

        {/* Bottom Analytics Row: Model States Donut + Resource Use Progress Rows */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          <LinearCard level={1} className="p-5 space-y-4">
            <div>
              <h3 className="text-[16px] font-semibold text-[var(--text)]">
                Model states
              </h3>
              <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                Engine lifecycle distribution
              </p>
            </div>
            <div className="flex flex-col items-center gap-3">
              <div ref={donutChartRef} className="h-36 w-36 shrink-0" />
              <DonutLegend items={donutData} />
            </div>
          </LinearCard>

          <LinearCard level={1} className="lg:col-span-2 p-5 space-y-4">
            <div className="flex items-center justify-between gap-2">
              <div>
                <h3 className="text-[16px] font-semibold text-[var(--text)]">
                  Resource use per model
                </h3>
                <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                  Threshold bands: green nominal, amber elevated, red critical
                </p>
              </div>
              <LinearButton
                variant="secondary"
                size="sm"
                onClick={() => onNavigateTab("models")}
              >
                Manage models
              </LinearButton>
            </div>
            <div className="space-y-4 pt-1">
              {modelResourceRows.map((row) => (
                <ProgressRow
                  key={row.name}
                  label={row.name}
                  value={row.pct}
                  displayValue={row.display}
                />
              ))}
            </div>
          </LinearCard>
        </div>

        {/* Task and Approval Queue Table */}
        <LinearCard level={1} className="p-5 space-y-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <h3 className="text-[16px] font-semibold text-[var(--text)]">
                Task and approval queue
              </h3>
              <p className="text-[12px] text-[var(--text-muted)] mt-0.5">
                Live orchestrator steps, signed gates, and recent missions
              </p>
            </div>
            <div className="flex items-center gap-2">
              <LinearButton
                variant="secondary"
                size="sm"
                onClick={() => onNavigateTab("timeline")}
              >
                Open timeline
              </LinearButton>
              <LinearButton
                variant="primary"
                size="sm"
                onClick={() => onNavigateTab("chat")}
              >
                Open agent chat
              </LinearButton>
            </div>
          </div>
          <DataTable
            columns={queueColumns}
            rows={queueRows}
            rowKey={(r) => r.id}
            emptyMessage="No queued tasks or pending approvals."
            onRowClick={() => onNavigateTab(approvalGate ? "chat" : "timeline")}
          />
        </LinearCard>
      </div>
    </div>
  );
};
