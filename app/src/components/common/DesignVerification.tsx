import React from "react";
import { GlassPanel } from "./GlassPanel";

export const DesignVerification: React.FC = () => {
  return (
    <div className="space-y-4 p-4 text-xs">
      <div className="flex items-center justify-between border-b border-white/10 pb-2">
        <h3 className="font-semibold tracking-wider text-slate-200 uppercase">
          Design Token Verification (DESIGN.md)
        </h3>
        <span className="font-mono text-cyan-400">Spec v1.0 Nominal</span>
      </div>

      {/* Palette Swatches */}
      <div className="grid grid-cols-6 gap-2">
        <div className="rounded-lg border border-white/10 bg-[#080B10] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#080B10] border border-white/20 mb-1" />
          <span className="text-[10px] font-mono text-slate-400">Canvas</span>
        </div>
        <div className="rounded-lg border border-white/10 bg-[#0E131B] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#0E131B] border border-white/20 mb-1" />
          <span className="text-[10px] font-mono text-slate-400">Raised</span>
        </div>
        <div className="rounded-lg border border-white/10 bg-[#141B26] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#38BDF8] mb-1" />
          <span className="text-[10px] font-mono text-cyan-400">Cyan</span>
        </div>
        <div className="rounded-lg border border-white/10 bg-[#141B26] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#10B981] mb-1" />
          <span className="text-[10px] font-mono text-emerald-400">Emerald</span>
        </div>
        <div className="rounded-lg border border-white/10 bg-[#141B26] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#F59E0B] mb-1" />
          <span className="text-[10px] font-mono text-amber-400">Amber</span>
        </div>
        <div className="rounded-lg border border-white/10 bg-[#141B26] p-2 text-center">
          <div className="h-6 w-full rounded bg-[#F43F5E] mb-1" />
          <span className="text-[10px] font-mono text-rose-400">Rose</span>
        </div>
      </div>

      {/* Glass Surface Recipes */}
      <div className="grid grid-cols-3 gap-3">
        <GlassPanel recipe="pane" className="p-3 text-center">
          <span className="font-semibold text-slate-200">Pane Recipe</span>
          <p className="mt-1 text-[11px] text-slate-400">24px blur · Sat 160% · 1px specular</p>
        </GlassPanel>
        <GlassPanel recipe="float" className="p-3 text-center">
          <span className="font-semibold text-cyan-300">Float Recipe</span>
          <p className="mt-1 text-[11px] text-slate-400">32px blur · Sat 190% · Inner gradient</p>
        </GlassPanel>
        <GlassPanel recipe="modal" className="p-3 text-center">
          <span className="font-semibold text-slate-100">Modal Recipe</span>
          <p className="mt-1 text-[11px] text-slate-400">36px blur · High elevation shadow</p>
        </GlassPanel>
      </div>
    </div>
  );
};
