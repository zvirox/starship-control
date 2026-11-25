import React from "react";

export type ProgressTone = "auto" | "blue" | "sky" | "green" | "amber" | "red";

export interface ProgressRowProps {
  label: string;
  value: number; // 0 - 100
  displayValue?: string;
  tone?: ProgressTone;
  color?: string;
  className?: string;
}

function resolveFillClass(value: number, tone: ProgressTone): string {
  if (tone === "blue") return "bg-[var(--accent-blue)]";
  if (tone === "sky") return "bg-[var(--accent-sky)]";
  if (tone === "green") return "bg-[var(--accent-green)]";
  if (tone === "amber") return "bg-[var(--accent-amber)]";
  if (tone === "red") return "bg-[var(--accent-red)]";
  if (value >= 90) return "bg-[var(--accent-red)]";
  if (value >= 70) return "bg-[var(--accent-amber)]";
  return "bg-[var(--accent-green)]";
}

export const ProgressRow: React.FC<ProgressRowProps> = ({
  label,
  value,
  displayValue,
  tone = "auto",
  color,
  className = "",
}) => {
  const clamped = Math.min(100, Math.max(0, value));
  const fillClass = color ? "" : resolveFillClass(clamped, tone);

  return (
    <div className={`flex flex-col gap-1.5 w-full ${className}`}>
      <div className="flex items-center justify-between text-[12px] gap-2">
        <span className="font-medium text-[var(--text)] truncate">{label}</span>
        <span className="font-mono text-[var(--text-muted)] tabular-nums shrink-0">
          {displayValue ?? `${clamped}%`}
        </span>
      </div>
      <div className="h-1 w-full rounded-[2px] bg-[var(--bg-bar-idle)] overflow-hidden">
        <div
          className={`h-full rounded-[2px] transition-all duration-150 ease-out ${fillClass}`}
          style={{
            width: `${clamped}%`,
            ...(color ? { backgroundColor: color } : {}),
          }}
        />
      </div>
    </div>
  );
};
