import React from "react";
import { ArrowUpRight, ArrowDownRight } from "lucide-react";
import { LinearCard } from "./LinearCard";
import { Sparkline } from "./Sparkline";

export type AccentTone =
  | "blue"
  | "sky"
  | "green"
  | "red"
  | "amber"
  | "teal"
  | "pink"
  | "purple"
  | "orange";

const TONE_CLASSES: Record<AccentTone, { bg: string; fg: string }> = {
  blue: { bg: "bg-[var(--tint-blue)]", fg: "text-[var(--accent-blue)]" },
  sky: { bg: "bg-[var(--tint-sky)]", fg: "text-[var(--accent-sky)]" },
  green: { bg: "bg-[var(--tint-green)]", fg: "text-[var(--accent-green)]" },
  red: { bg: "bg-[var(--tint-red)]", fg: "text-[var(--accent-red)]" },
  amber: { bg: "bg-[var(--tint-amber)]", fg: "text-[var(--accent-amber)]" },
  teal: { bg: "bg-[var(--tint-teal)]", fg: "text-[var(--accent-teal)]" },
  pink: { bg: "bg-[var(--tint-pink)]", fg: "text-[var(--accent-pink)]" },
  purple: { bg: "bg-[var(--tint-purple)]", fg: "text-[var(--accent-purple)]" },
  orange: { bg: "bg-[var(--tint-orange)]", fg: "text-[var(--accent-orange)]" },
};

export interface StatCardProps {
  title: string;
  value: string | number;
  unit?: string;
  delta?: {
    value: string | number;
    positive?: boolean;
    comparison?: string;
  };
  sparkline?: number[];
  icon?: React.ReactNode;
  tone?: AccentTone;
  className?: string;
}

export const StatCard: React.FC<StatCardProps> = ({
  title,
  value,
  unit,
  delta,
  sparkline,
  icon,
  tone = "blue",
  className = "",
}) => {
  const chipTone = TONE_CLASSES[tone];
  const deltaPositive = delta?.positive ?? true;

  return (
    <LinearCard
      level={1}
      className={`p-5 flex flex-col justify-between gap-3 ${className}`}
    >
      {/* Top row: 12px uppercase label + solid precomputed icon chip */}
      <div className="flex items-center justify-between gap-2">
        <span className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
          {title}
        </span>
        {icon && (
          <div
            className={`size-7 rounded-[6px] flex items-center justify-center shrink-0 ${chipTone.bg} ${chipTone.fg}`}
          >
            {icon}
          </div>
        )}
      </div>

      {/* Middle row: 28px semibold tabular figure + 12px muted unit + sparkline */}
      <div className="flex items-end justify-between gap-2 min-w-0">
        <div className="flex items-baseline gap-1.5 min-w-0 flex-wrap">
          <span className="text-[28px] font-semibold tracking-tight text-[var(--text)] tabular-nums leading-none shrink-0">
            {value}
          </span>
          {unit && (
            <span className="text-[12px] font-medium text-[var(--text-muted)] whitespace-nowrap">
              {unit}
            </span>
          )}
        </div>
        {sparkline && sparkline.length > 1 && (
          <Sparkline data={sparkline} positive={deltaPositive} width={60} height={24} />
        )}
      </div>

      {/* Bottom row: solid delta pill (4px radius) + comparison */}
      {delta && (
        <div className="flex items-center gap-2 min-w-0">
          <span
            className={`inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded-[4px] text-[12px] font-medium tabular-nums whitespace-nowrap shrink-0 ${
              deltaPositive
                ? "bg-[var(--tint-green)] text-[var(--accent-green)]"
                : "bg-[var(--tint-red)] text-[var(--accent-red)]"
            }`}
          >
            {deltaPositive ? (
              <ArrowUpRight className="size-3 shrink-0" aria-hidden="true" />
            ) : (
              <ArrowDownRight className="size-3 shrink-0" aria-hidden="true" />
            )}
            {delta.value}
          </span>
          {delta.comparison && (
            <span className="text-[12px] text-[var(--text-muted)] truncate">
              {delta.comparison}
            </span>
          )}
        </div>
      )}
    </LinearCard>
  );
};
