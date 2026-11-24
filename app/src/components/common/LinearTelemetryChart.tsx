import React, { useEffect, useRef } from "react";
import * as echarts from "echarts";
import { getLinearEChartsOption } from "../linear/echarts-linear-theme";

export interface TelemetrySeries {
  name: string;
  data: number[];
  color?: string;
}

export interface LinearTelemetryChartProps {
  title?: string;
  subtitle?: string;
  timestamps?: string[];
  series: TelemetrySeries[];
  unit?: string;
  height?: string;
  className?: string;
  isDark?: boolean;
}

export const LinearTelemetryChart: React.FC<LinearTelemetryChartProps> = ({
  title,
  subtitle,
  timestamps = ["-60s", "-50s", "-40s", "-30s", "-20s", "-10s", "now"],
  series,
  unit = "%",
  height = "h-56",
  className = "",
  isDark = true,
}) => {
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current) return;

    if (!chartInstance.current) {
      chartInstance.current = echarts.init(chartRef.current);
    }

    const baseOption = getLinearEChartsOption(isDark);
    const option = {
      ...baseOption,
      tooltip: {
        ...baseOption.tooltip,
        trigger: "axis",
        valueFormatter: (value: any) => `${value} ${unit}`,
      },
      xAxis: {
        ...baseOption.xAxis,
        type: "category",
        data: timestamps,
        boundaryGap: false,
      },
      yAxis: {
        ...baseOption.yAxis,
        type: "value",
        axisLabel: {
          formatter: `{value}${unit}`,
        },
      },
      series: series.map((s, idx) => ({
        name: s.name,
        type: "line",
        smooth: true,
        showSymbol: false,
        areaStyle: {
          opacity: 0.15,
        },
        data: s.data,
        color: s.color || (idx === 0 ? "#22d3ee" : idx === 1 ? "#a855f7" : "#10b981"),
      })),
    };

    chartInstance.current.setOption(option);

    const handleResize = () => chartInstance.current?.resize();
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [timestamps, series, unit, isDark]);

  return (
    <div className={`w-full ${className}`}>
      {(title || subtitle) && (
        <div className="flex items-center justify-between mb-3">
          <div>
            {title && <h4 className="text-xs font-semibold text-text-primary">{title}</h4>}
            {subtitle && <p className="text-[11px] text-text-secondary">{subtitle}</p>}
          </div>
          <div className="flex items-center gap-3 text-xs font-mono">
            {series.map((s, i) => (
              <span key={s.name} className="flex items-center gap-1.5">
                <span
                  className="size-2 rounded-full"
                  style={{
                    backgroundColor:
                      s.color || (i === 0 ? "#22d3ee" : i === 1 ? "#a855f7" : "#10b981"),
                  }}
                />
                <span className="text-text-secondary text-[11px]">{s.name}</span>
              </span>
            ))}
          </div>
        </div>
      )}
      <div ref={chartRef} className={`w-full ${height}`} />
    </div>
  );
};
