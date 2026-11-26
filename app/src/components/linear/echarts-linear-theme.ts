import * as echarts from "echarts";

const COLOR_VARIABLES = {
  primary: "--primary",
  blue: "--blue",
  blueText: "--sky-text",
  sky: "--sky",
  skyText: "--sky-text",
  green: "--sky",
  greenText: "--sky-text",
  amber: "--amber",
  teal: "--sky",
  pink: "--pink",
  purple: "--purple",
  orange: "--orange",
  red: "--red",
  redText: "--red-text",
  gray: "--gray",
  idle: "--bg-bar-idle",
  border: "--border",
  borderStrong: "--border-strong",
  grid: "--chart-grid",
  text: "--text",
  textSecondary: "--text-secondary",
  textMuted: "--text-muted",
  surface: "--bg-surface",
  surface2: "--bg-surface-2",
} as const;

type ColorName = keyof typeof COLOR_VARIABLES;

function readColor(name: ColorName): string {
  if (typeof window === "undefined" || typeof getComputedStyle === "undefined") return "";
  return getComputedStyle(document.documentElement)
    .getPropertyValue(COLOR_VARIABLES[name])
    .trim();
}

const LEGACY_ALIASES: Record<string, ColorName> = {
  idleDark: "idle",
  idleLight: "idle",
  surfaceDark: "surface",
  surfaceLight: "surface",
  borderStrongDark: "borderStrong",
  borderStrongLight: "borderStrong",
  textDark: "text",
  textLight: "text",
  textMutedDark: "textMuted",
  textMutedLight: "textMuted",
};

/** Live CSS-variable-backed palette. Values update with the active `data-theme`. */
export const LINEAR_COLORS = new Proxy({} as Record<string, string>, {
  get: (_target, name: string) =>
    readColor(LEGACY_ALIASES[name] ?? (name as ColorName)),
});

/** Starship Control series order (Plan §5): primary, sky, amber, pink, blue.
 *  Purple is skipped as a near-duplicate of the Violet primary. */
const SERIES_NAMES: ColorName[] = [
  "primary",
  "sky",
  "amber",
  "pink",
  "blue",
];

export function getLinearSeriesOrder(): string[] {
  return SERIES_NAMES.map(readColor);
}

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

export const getLinearEChartsOption = (_isDark: boolean = true) => ({
  backgroundColor: "transparent",
  textStyle: {
    fontFamily: "Inter, system-ui, -apple-system, sans-serif",
    fontSize: 12,
    color: LINEAR_COLORS.textMuted,
  },
  grid: {
    top: 20,
    right: 12,
    bottom: 24,
    left: 44,
    containLabel: false,
  },
  tooltip: {
    backgroundColor: LINEAR_COLORS.surface,
    borderColor: LINEAR_COLORS.borderStrong,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: {
      color: LINEAR_COLORS.text,
      fontSize: 12,
      fontFamily: "Inter, system-ui, -apple-system, sans-serif",
    },
    extraCssText: "border-radius: 4px; box-shadow: var(--shadow-card);",
  },
  xAxis: {
    axisLine: {
      lineStyle: {
        color: LINEAR_COLORS.grid,
      },
    },
    axisLabel: {
      fontSize: 12,
      color: LINEAR_COLORS.textMuted,
    },
    splitLine: { show: false },
    axisTick: { show: false },
  },
  yAxis: {
    axisLine: { show: false },
    axisLabel: {
      fontSize: 12,
      color: LINEAR_COLORS.textMuted,
    },
    splitLine: {
      lineStyle: {
        type: "dotted" as const,
        color: LINEAR_COLORS.grid,
      },
    },
  },
  color: getLinearSeriesOrder(),
});
