import React from "react";
import { LinearCard } from "../linear/LinearCard";

export const DesignVerification: React.FC = () => {
  return (
    <div className="space-y-4 p-4 text-[12px] font-sans">
      <div className="flex items-center justify-between border-b border-[var(--border)] pb-2">
        <h3 className="font-medium text-[var(--text-muted)]">
          Design token verification
        </h3>
        <span className="font-mono text-[var(--accent-blue)]">Linear v2</span>
      </div>

      <div className="grid grid-cols-6 gap-2">
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-app)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--bg-app)] border border-[var(--border)] mb-1" />
          <span className="font-mono text-[var(--text-muted)]">App Base</span>
        </div>
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--bg-surface)] border border-[var(--border)] mb-1" />
          <span className="font-mono text-[var(--text-muted)]">Surface 1</span>
        </div>
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--accent-blue)] mb-1" />
          <span className="font-mono text-[var(--accent-blue)]">Blue</span>
        </div>
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--accent-green)] mb-1" />
          <span className="font-mono text-[var(--accent-green)]">Green</span>
        </div>
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--accent-amber)] mb-1" />
          <span className="font-mono text-[var(--accent-amber)]">Amber</span>
        </div>
        <div className="rounded-[6px] border border-[var(--border)] bg-[var(--bg-surface-2)] p-2 text-center">
          <div className="h-6 w-full rounded-[4px] bg-[var(--accent-red)] mb-1" />
          <span className="font-mono text-[var(--accent-red)]">Red</span>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3">
        <LinearCard level={1} className="p-3 text-center">
          <span className="font-medium text-[var(--text)]">Surface 1</span>
          <p className="mt-1 text-[var(--text-muted)]">
            Solid opaque · 1px border · 8px radius
          </p>
        </LinearCard>
        <LinearCard level={2} className="p-3 text-center">
          <span className="font-medium text-[var(--text)]">Surface 2</span>
          <p className="mt-1 text-[var(--text-muted)]">
            Nested block · 1px border · 8px radius
          </p>
        </LinearCard>
        <LinearCard level={3} className="p-3 text-center">
          <span className="font-medium text-[var(--text)]">Surface 3</span>
          <p className="mt-1 text-[var(--text-muted)]">
            Modal · Strong border · Zero blur
          </p>
        </LinearCard>
      </div>
    </div>
  );
};
