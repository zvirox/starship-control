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
    <GlassPanel recipe="pane" className="p-4 rounded-2xl border border-border-subtle mb-4">
      {/* Plan Header */}
      <div className="flex items-center justify-between pb-3 border-b border-border-subtle">
        <div className="flex items-center gap-2">
          <Layers className="h-4 w-4 text-text-secondary" />
          <h4 className="font-sans text-xs font-medium text-text-primary">
            {plan.title || "Execution plan (tasksheet/1)"}
          </h4>
          <span className="text-[11px] font-mono text-text-secondary bg-white/[0.05] px-2 py-1 rounded-lg border border-border-subtle">
            v{plan.version}
          </span>
        </div>

        <div className="flex items-center gap-2 text-[11px] font-mono text-text-secondary">
          <span className="flex items-center gap-1">
            Status
            <span
              className={`font-medium ${
                plan.status === "completed"
                  ? "text-signal-success"
                  : plan.status === "pending_approval"
                  ? "text-signal-gate"
                  : "text-signal-thinking"
              }`}
            >
              {plan.status.replace("_", " ")}
            </span>
          </span>
        </div>
      </div>

      <p className="mt-2 text-xs text-text-secondary font-sans leading-relaxed">
        {plan.summary}
      </p>

      {/* Steps Pipeline Visualizer */}
      <div className="mt-4 space-y-2">
        <div className="text-[11px] font-medium text-text-secondary">
          Plan steps
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
                className={`flex items-center justify-between rounded-xl p-3 border transition-colors cursor-pointer ${
                  isPendingApproval
                    ? "border-signal-gate/40 bg-signal-gate-bg"
                    : isRunning
                    ? "border-accent/40 bg-accent-muted"
                    : isCompleted
                    ? "border-signal-success/30 bg-signal-success-bg"
                    : isFailed
                    ? "border-signal-gate/30 bg-signal-gate-bg"
                    : "border-border-subtle bg-white/[0.03] hover:bg-white/[0.05]"
                }`}
              >
                <div className="flex items-center gap-3">
                  {/* Step Index Badge */}
                  <div
                    className={`flex h-6 w-6 items-center justify-center rounded-lg font-mono text-xs font-medium ${
                      isCompleted
                        ? "bg-signal-success-bg text-signal-success"
                        : isRunning
                        ? "bg-accent-muted text-accent"
                        : isPendingApproval
                        ? "bg-signal-gate-bg text-signal-gate"
                        : "bg-raised text-text-secondary"
                    }`}
                  >
                    {isCompleted ? (
                      <CheckCircle2 className="h-3.5 w-3.5" />
                    ) : isRunning ? (
                      <Play className="h-3.5 w-3.5" />
                    ) : isFailed ? (
                      <XCircle className="h-3.5 w-3.5" />
                    ) : (
                      idx + 1
                    )}
                  </div>

                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-medium text-text-primary">
                        {step.appId}
                      </span>
                      <ArrowRight className="h-3.5 w-3.5 text-text-disabled" />
                      <span className="font-mono text-xs text-text-secondary">
                        {step.toolName}
                      </span>
                    </div>
                    <div className="text-[11px] text-text-secondary mt-1 line-clamp-1">
                      {step.diffSummary || "Autonomous inspection or action execution"}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  {/* Risk Badge */}
                  <span
                    className={`text-[11px] font-mono px-2 py-1 rounded-lg border ${
                      step.riskTier === 3
                        ? "bg-signal-gate-bg text-signal-gate border-signal-gate/40"
                        : step.riskTier === 2
                        ? "bg-signal-thinking-bg text-signal-thinking border-signal-thinking/40"
                        : "bg-signal-success-bg text-signal-success border-signal-success/40"
                    }`}
                  >
                    Tier {step.riskTier}
                  </span>

                  {/* Status Indicator */}
                  {isPendingApproval ? (
                    <span className="flex items-center gap-1 text-[11px] font-medium text-signal-gate bg-signal-gate-bg px-2 py-1 rounded-lg border border-signal-gate/40">
                      <AlertTriangle className="h-3.5 w-3.5" />
                      Approval gate
                    </span>
                  ) : isRunning ? (
                    <span className="flex items-center gap-1 text-[11px] font-mono text-accent">
                      <Clock className="h-3.5 w-3.5" />
                      Running
                    </span>
                  ) : isCompleted ? (
                    <span className="flex items-center gap-1 text-[11px] font-mono text-signal-success">
                      <Shield className="h-3.5 w-3.5" />
                      Passed
                    </span>
                  ) : (
                    <span className="text-[11px] font-mono text-text-disabled">
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
