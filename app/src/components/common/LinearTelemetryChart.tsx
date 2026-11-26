import React, { useEffect, useRef } from "react";
import * as echarts from "echarts";
import {
  getLinearEChartsOption,
  getLinearSeriesOrder,
  createLinearAreaGradient,
} from "../linear/echarts-linear-theme";
import { useThemeStore } from "../../stores/themeStore";

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
  isDark,
}) => {
  const storeTheme = useThemeStore((s) => s.theme);
  const darkMode = isDark ?? storeTheme === "dark";
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current) return;

    if (!chartInstance.current) {
      chartInstance.current = echarts.init(chartRef.current);
    }

    const baseOption = getLinearEChartsOption(darkMode);
    const option = {
      ...baseOption,
      tooltip: {
        ...baseOption.tooltip,
        trigger: "axis",
        valueFormatter: (value: unknown) => `${value} ${unit}`,
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
          ...baseOption.yAxis.axisLabel,
          formatter: `{value}${unit}`,
        },
      },
      series: series.map((s, idx) => {
        const seriesOrder = getLinearSeriesOrder();
        const seriesColor = s.color || seriesOrder[idx % seriesOrder.length];
        return {
          name: s.name,
          type: "line",
          smooth: true,
          showSymbol: false,
          lineStyle: { width: 2, color: seriesColor },
          areaStyle: {
            color: createLinearAreaGradient(seriesColor, 0.2),
          },
          data: s.data,
          color: seriesColor,
        };
      }),
    };

    chartInstance.current.setOption(option, true);

    const handleResize = () => chartInstance.current?.resize();
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [timestamps, series, unit, darkMode]);

  useEffect(() => {
    return () => {
      chartInstance.current?.dispose();
      chartInstance.current = null;
    };
  }, []);

  return (
    <div className={`w-full ${className}`}>
      {(title || subtitle) && (
        <div className="flex flex-wrap items-center justify-between gap-3 mb-3">
          <div>
            {title && (
              <h4 className="text-[13px] font-semibold text-[var(--text)]">
                {title}
              </h4>
            )}
            {subtitle && (
              <p className="text-[12px] text-[var(--text-muted)]">{subtitle}</p>
            )}
          </div>
          <div className="flex items-center gap-3 text-[12px] font-mono">
            {series.map((s, i) => {
              const seriesOrder = getLinearSeriesOrder();
              const seriesColor = s.color || seriesOrder[i % seriesOrder.length];
              return (
                <span key={s.name} className="flex items-center gap-1.5">
                  <span
                    className="size-2 rounded-[2px] shrink-0"
                    style={{ backgroundColor: seriesColor }}
                    aria-hidden="true"
                  />
                  <span className="text-[var(--text-muted)]">{s.name}</span>
                </span>
              );
            })}
          </div>
        </div>
      )}
      <div ref={chartRef} className={`w-full ${height}`} />
    </div>
  );
};
