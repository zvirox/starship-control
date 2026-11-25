import React from "react";
import { ChevronRight } from "lucide-react";

export interface BreadcrumbSegment {
  label: string;
  onClick?: () => void;
}

export interface BreadcrumbProps {
  segments: BreadcrumbSegment[];
  actions?: React.ReactNode;
  className?: string;
}

export const Breadcrumb: React.FC<BreadcrumbProps> = ({
  segments,
  actions,
  className = "",
}) => {
  return (
    <div
      className={`h-11 w-full bg-[var(--bg-surface)] border-b border-[var(--border)] px-5 flex items-center justify-between gap-4 shrink-0 ${className}`}
    >
      <nav aria-label="Breadcrumb" className="flex items-center gap-1.5 min-w-0">
        {segments.map((seg, idx) => {
          const isLast = idx === segments.length - 1;
          return (
            <React.Fragment key={`${seg.label}-${idx}`}>
              {idx > 0 && (
                <ChevronRight
                  className="size-3.5 text-[var(--text-faint)] shrink-0"
                  strokeWidth={1.75}
                  aria-hidden="true"
                />
              )}
              {seg.onClick && !isLast ? (
                <button
                  type="button"
                  onClick={seg.onClick}
                  className="text-[13px] text-[var(--text-muted)] hover:text-[var(--text)] transition-colors duration-150 focus-ring rounded-[4px] px-1 cursor-pointer truncate"
                >
                  {seg.label}
                </button>
              ) : (
                <span
                  className={`text-[13px] truncate ${
                    isLast
                      ? "font-medium text-[var(--text)]"
                      : "text-[var(--text-muted)]"
                  }`}
                >
                  {seg.label}
                </span>
              )}
            </React.Fragment>
          );
        })}
      </nav>

      {actions && (
        <div className="flex items-center gap-2 shrink-0">{actions}</div>
      )}
    </div>
  );
};
