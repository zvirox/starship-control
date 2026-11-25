import React from "react";

export interface RangeTabsProps {
  options?: string[];
  active: string;
  onChange: (tab: string) => void;
  className?: string;
}

const DEFAULT_OPTIONS = ["1W", "1M", "3W", "YTD", "Total"];

export const RangeTabs: React.FC<RangeTabsProps> = ({
  options = DEFAULT_OPTIONS,
  active,
  onChange,
  className = "",
}) => {
  return (
    <div
      role="tablist"
      className={`inline-flex items-center gap-0.5 p-0.5 rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] select-none ${className}`}
    >
      {options.map((tab) => {
        const isActive = tab === active;
        return (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(tab)}
            className={`px-2.5 py-1 text-[12px] rounded-[4px] font-medium transition-colors duration-150 ease-out focus-ring cursor-pointer ${
              isActive
                ? "surface-panel text-[var(--text)]"
                : "text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-bar-idle)] border border-transparent"
            }`}
          >
            {tab}
          </button>
        );
      })}
    </div>
  );
};
