import * as echarts from "echarts";

export const LINEAR_COLORS = {
  blue: "#3355FF",
  sky: "#38B6F0",
  green: "#22C55E",
  amber: "#EAB308",
  teal: "#1FB5A5",
  pink: "#EC4899",
  purple: "#7C5CE6",
  orange: "#F97316",
  red: "#F43F5E",
  idleDark: "#202128",
  idleLight: "#EAF0FF",
  borderDark: "#1F2125",
  borderStrongDark: "#2C2E33",
  borderLight: "#E5E7EB",
  borderStrongLight: "#D1D5DB",
  textDark: "#F5F5F6",
  textMutedDark: "#8A8B91",
  textLight: "#0F1115",
  textMutedLight: "#6B7280",
  surfaceDark: "#111213",
  surfaceLight: "#FFFFFF",
};

/** Exact Plan §1 series order: blue, sky, green, amber, teal, pink, purple, orange */
export const LINEAR_SERIES_ORDER = [
  LINEAR_COLORS.blue,
  LINEAR_COLORS.sky,
  LINEAR_COLORS.green,
  LINEAR_COLORS.amber,
  LINEAR_COLORS.teal,
  LINEAR_COLORS.pink,
  LINEAR_COLORS.purple,
  LINEAR_COLORS.orange,
] as const;

export function createLinearAreaGradient(hexColor: string, topOpacity = 0.2) {
  const clean = hexColor.startsWith("#") ? hexColor.slice(0, 7) : `#${hexColor}`;
  const alphaByte = Math.round(Math.min(1, Math.max(0, topOpacity)) * 255)
    .toString(16)
    .padStart(2, "0");
  return new echarts.graphic.LinearGradient(0, 0, 0, 1, [
    { offset: 0, color: `${clean}${alphaByte}` },
    { offset: 1, color: `${clean}00` },
  ]);
}

export const getLinearEChartsOption = (isDark: boolean = true) => ({
  backgroundColor: "transparent",
  textStyle: {
    fontFamily: "Inter, system-ui, -apple-system, sans-serif",
    fontSize: 12,
    color: isDark ? LINEAR_COLORS.textMutedDark : LINEAR_COLORS.textMutedLight,
  },
  grid: {
    top: 20,
    right: 12,
    bottom: 24,
    left: 44,
    containLabel: false,
  },
  tooltip: {
    backgroundColor: isDark
      ? LINEAR_COLORS.surfaceDark
      : LINEAR_COLORS.surfaceLight,
    borderColor: isDark
      ? LINEAR_COLORS.borderStrongDark
      : LINEAR_COLORS.borderLight,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: {
      color: isDark ? LINEAR_COLORS.textDark : LINEAR_COLORS.textLight,
      fontSize: 12,
      fontFamily: "Inter, system-ui, -apple-system, sans-serif",
    },
    extraCssText: isDark
      ? "border-radius: 4px; box-shadow: none;"
      : "border-radius: 4px; box-shadow: 0 1px 2px rgb(16 24 40 / 5%);",
  },
  xAxis: {
    axisLine: {
      lineStyle: {
        color: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight,
      },
    },
    axisLabel: {
      fontSize: 12,
      color: isDark
        ? LINEAR_COLORS.textMutedDark
        : LINEAR_COLORS.textMutedLight,
    },
    splitLine: { show: false },
    axisTick: { show: false },
  },
  yAxis: {
    axisLine: { show: false },
    axisLabel: {
      fontSize: 12,
      color: isDark
        ? LINEAR_COLORS.textMutedDark
        : LINEAR_COLORS.textMutedLight,
    },
    splitLine: {
      lineStyle: {
        type: "dotted" as const,
        color: isDark ? LINEAR_COLORS.borderDark : LINEAR_COLORS.borderLight,
      },
    },
  },
  color: [...LINEAR_SERIES_ORDER],
});
