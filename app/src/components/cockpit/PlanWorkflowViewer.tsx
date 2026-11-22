import React from "react";
import {
  CheckCircle2,
  Clock,
  AlertTriangle,
  Play,
  XCircle,
  Shield,
  Layers,
  ArrowRight,
} from "lucide-react";
import { GlassPanel } from "../common/GlassPanel";
import type { PlanActiveResponse, PlanStepDetail } from "../../types/ipc";

export interface PlanWorkflowViewerProps {
  plan: PlanActiveResponse;
  onSelectStep?: (step: PlanStepDetail) => void;
}

export const PlanWorkflowViewer: React.FC<PlanWorkflowViewerProps> = ({
  plan,
  onSelectStep,
}) => {
  return (
    <GlassPanel recipe="pane" className="p-4 rounded-2xl border border-white/10 mb-4">
      {/* Plan Header */}
      <div className="flex items-center justify-between pb-3 border-b border-white/5">
        <div className="flex items-center gap-2">
          <Layers className="h-4 w-4 text-cyan-400" />
          <h4 className="font-sans text-xs font-bold text-slate-100 tracking-wide uppercase">
            {plan.title || "Execution Plan (tasksheet/1)"}
          </h4>
          <span className="text-[10px] font-mono text-cyan-400 bg-cyan-950/60 px-1.5 py-0.5 rounded border border-cyan-500/30">
            v{plan.version}
          </span>
        </div>

        <div className="flex items-center gap-2 text-[11px] font-mono text-slate-400">
          <span className="flex items-center gap-1">
            Status:
            <span
              className={`font-semibold capitalize ${
                plan.status === "completed"
                  ? "text-emerald-400"
                  : plan.status === "pending_approval"
                  ? "text-rose-400"
                  : "text-amber-400"
              }`}
            >
              {plan.status.replace("_", " ")}
            </span>
          </span>
        </div>
      </div>

      <p className="mt-2 text-xs text-slate-300 font-sans leading-relaxed">
        {plan.summary}
      </p>

      {/* Steps Pipeline Visualizer */}
      <div className="mt-4 space-y-2">
        <div className="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">
          Plan Steps Sequence:
        </div>

        <div className="grid gap-2">
          {plan.steps.map((step, idx) => {
            const isCompleted = step.executionStatus === "success";
            const isRunning = step.executionStatus === "running";
            const isFailed = step.executionStatus === "failed";
            const isPendingApproval =
              step.riskTier >= 2 && step.approvalStatus === "pending";

            return (
              <div
                key={step.stepId}
                onClick={() => onSelectStep?.(step)}
                className={`flex items-center justify-between rounded-xl p-3 border transition-all cursor-pointer ${
                  isPendingApproval
                    ? "border-rose-500/50 bg-rose-500/10 shadow-[0_0_16px_rgba(244,63,94,0.15)]"
                    : isRunning
                    ? "border-cyan-400/50 bg-cyan-500/10 animate-pulse"
                    : isCompleted
                    ? "border-emerald-500/30 bg-emerald-500/5"
                    : isFailed
                    ? "border-rose-500/30 bg-rose-500/5"
                    : "border-white/5 bg-white/5 hover:border-white/20"
                }`}
              >
                <div className="flex items-center gap-3">
                  {/* Step Index Badge */}
                  <div
                    className={`flex h-6 w-6 items-center justify-center rounded-lg font-mono text-xs font-bold ${
                      isCompleted
                        ? "bg-emerald-500/20 text-emerald-300"
                        : isRunning
                        ? "bg-cyan-500/20 text-cyan-300 animate-spin"
                        : isPendingApproval
                        ? "bg-rose-500/20 text-rose-300"
                        : "bg-slate-800 text-slate-400"
                    }`}
                  >
                    {isCompleted ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-emerald-400" />
                    ) : isRunning ? (
                      <Play className="h-3 w-3 text-cyan-400" />
                    ) : isFailed ? (
                      <XCircle className="h-3.5 w-3.5 text-rose-400" />
                    ) : (
                      idx + 1
                    )}
                  </div>

                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-bold uppercase text-slate-200">
                        {step.appId}
                      </span>
                      <ArrowRight className="h-3 w-3 text-slate-600" />
                      <span className="font-mono text-xs text-cyan-300">
                        {step.toolName}
                      </span>
                    </div>
                    <div className="text-[11px] text-slate-400 mt-0.5 line-clamp-1">
                      {step.diffSummary || "Autonomous inspection or action execution"}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  {/* Risk Badge */}
                  <span
                    className={`text-[10px] font-mono px-2 py-0.5 rounded-full border ${
                      step.riskTier === 3
                        ? "bg-rose-500/20 text-rose-300 border-rose-500/40"
                        : step.riskTier === 2
                        ? "bg-amber-500/20 text-amber-300 border-amber-500/40"
                        : "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                    }`}
                  >
                    Tier {step.riskTier}
                  </span>

                  {/* Status Indicator */}
                  {isPendingApproval ? (
                    <span className="flex items-center gap-1 text-[11px] font-semibold text-rose-400 bg-rose-950/60 px-2 py-0.5 rounded-lg border border-rose-500/30">
                      <AlertTriangle className="h-3 w-3 animate-pulse" />
                      Approval Gate
                    </span>
                  ) : isRunning ? (
                    <span className="flex items-center gap-1 text-[11px] font-mono text-cyan-300">
                      <Clock className="h-3 w-3 animate-spin" />
                      Running
                    </span>
                  ) : isCompleted ? (
                    <span className="flex items-center gap-1 text-[11px] font-mono text-emerald-400">
                      <Shield className="h-3 w-3" />
                      Passed
                    </span>
                  ) : (
                    <span className="text-[11px] font-mono text-slate-500">
                      Pending
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </GlassPanel>
  );
};
