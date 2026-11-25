import React, { forwardRef } from "react";

export interface LinearCardProps extends React.HTMLAttributes<HTMLDivElement> {
  level?: 1 | 2 | 3 | "pane" | "raised" | "modal" | "overlay";
}

export const LinearCard = forwardRef<HTMLDivElement, LinearCardProps>(
  ({ level = 1, className = "", children, ...props }, ref) => {
    const normalizedLevel: 1 | 2 | 3 =
      typeof level === "number"
        ? level
        : level === "raised"
        ? 2
        : level === "modal" || level === "overlay"
        ? 3
        : 1;

    const levelStyles = {
      1: "surface-panel",
      2: "surface-nested",
      3: "surface-panel border-[var(--border-strong)]",
    }[normalizedLevel];

    return (
      <div
        ref={ref}
        className={`rounded-[8px] text-[var(--text)] transition-colors duration-150 ease-out hover:border-[var(--border-strong)] ${levelStyles} ${className}`}
        {...props}
      >
        {children}
      </div>
    );
  }
);

LinearCard.displayName = "LinearCard";
