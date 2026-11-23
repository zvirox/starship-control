import React from "react";
import { GlassPanel } from "./GlassPanel";

export const DesignVerification: React.FC = () => {
  return (
    <div className="space-y-4 p-4 text-xs font-sans">
      <div className="flex items-center justify-between border-b border-border-subtle pb-2">
        <h3 className="font-medium text-text-secondary">Design token verification</h3>
        <span className="font-mono text-accent">Spec v1.0</span>
      </div>

      {/* Palette Swatches */}
      <div className="grid grid-cols-6 gap-2">
        <div className="rounded-lg border border-border-subtle bg-canvas p-2 text-center">
          <div className="h-6 w-full rounded bg-canvas border border-border-subtle mb-1" />
          <span className="text-[11px] font-mono text-text-secondary">Canvas</span>
        </div>
        <div className="rounded-lg border border-border-subtle bg-raised p-2 text-center">
          <div className="h-6 w-full rounded bg-raised border border-border-subtle mb-1" />
          <span className="text-[11px] font-mono text-text-secondary">Raised</span>
        </div>
        <div className="rounded-lg border border-border-subtle bg-surface-solid p-2 text-center">
          <div className="h-6 w-full rounded bg-accent mb-1" />
          <span className="text-[11px] font-mono text-accent">Accent</span>
        </div>
        <div className="rounded-lg border border-border-subtle bg-surface-solid p-2 text-center">
          <div className="h-6 w-full rounded bg-signal-success mb-1" />
          <span className="text-[11px] font-mono text-signal-success">Success</span>
        </div>
        <div className="rounded-lg border border-border-subtle bg-surface-solid p-2 text-center">
          <div className="h-6 w-full rounded bg-signal-thinking mb-1" />
          <span className="text-[11px] font-mono text-signal-thinking">Thinking</span>
        </div>
        <div className="rounded-lg border border-border-subtle bg-surface-solid p-2 text-center">
          <div className="h-6 w-full rounded bg-signal-gate mb-1" />
          <span className="text-[11px] font-mono text-signal-gate">Gate</span>
        </div>
      </div>

      {/* Glass Surface Recipes */}
      <div className="grid grid-cols-3 gap-3">
        <GlassPanel recipe="pane" className="p-3 text-center">
          <span className="font-medium text-text-primary">Pane recipe</span>
          <p className="mt-1 text-[11px] text-text-secondary">24px blur · Sat 160% · flat border</p>
        </GlassPanel>
        <GlassPanel recipe="float" className="p-3 text-center">
          <span className="font-medium text-text-primary">Float recipe</span>
          <p className="mt-1 text-[11px] text-text-secondary">32px blur · Sat 190% · soft elevation</p>
        </GlassPanel>
        <GlassPanel recipe="modal" className="p-3 text-center">
          <span className="font-medium text-text-primary">Modal recipe</span>
          <p className="mt-1 text-[11px] text-text-secondary">36px blur · High elevation shadow</p>
        </GlassPanel>
      </div>
    </div>
  );
};
