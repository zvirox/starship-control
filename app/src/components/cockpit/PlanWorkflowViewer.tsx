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
import { LinearCard } from "../linear";
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
    <LinearCard level={1} className="p-5 mb-4">
      <div className="flex items-center justify-between pb-3 border-b border-[var(--border)]">
        <div className="flex items-center gap-2">
          <Layers className="size-4 text-[var(--text-muted)]" />
          <h4 className="font-sans text-[13px] font-semibold text-[var(--text)]">
            {plan.title || "Execution plan (tasksheet/1)"}
          </h4>
          <span className="text-[12px] font-mono text-[var(--text-muted)] bg-[var(--bg-surface-2)] px-2 py-0.5 rounded-[4px] border border-[var(--border)]">
            v{plan.version}
          </span>
        </div>

        <div className="flex items-center gap-2 text-[12px] font-mono text-[var(--text-muted)]">
          <span className="flex items-center gap-1">
            Status
            <span
              className={`font-medium ${
                plan.status === "completed"
                  ? "text-[var(--accent-green)]"
                  : plan.status === "pending_approval"
                  ? "text-[var(--accent-red)]"
                  : "text-[var(--accent-amber)]"
              }`}
            >
              {plan.status.replace("_", " ")}
            </span>
          </span>
        </div>
      </div>

      <p className="mt-2 text-[12px] text-[var(--text-muted)] font-sans leading-relaxed">
        {plan.summary}
      </p>

      <div className="mt-4 space-y-2">
        <div className="text-[12px] font-medium uppercase tracking-wider text-[var(--text-muted)]">
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
                className={`flex items-center justify-between rounded-[8px] p-3 border transition-colors cursor-pointer ${
                  isPendingApproval
                    ? "border-[var(--accent-red)] bg-[var(--tint-red)]"
                    : isRunning
                    ? "border-[var(--accent-blue)] bg-[var(--tint-blue)]"
                    : isCompleted
                    ? "border-[var(--border)] bg-[var(--tint-green)]"
                    : isFailed
                    ? "border-[var(--accent-red)] bg-[var(--tint-red)]"
                    : "border-[var(--border)] bg-[var(--bg-surface-2)] hover:border-[var(--border-strong)]"
                }`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`flex size-6 items-center justify-center rounded-[6px] font-mono text-[12px] font-medium ${
                      isCompleted
                        ? "bg-[var(--tint-green)] text-[var(--accent-green)]"
                        : isRunning
                        ? "bg-[var(--tint-blue)] text-[var(--accent-blue)]"
                        : isPendingApproval
                        ? "bg-[var(--tint-red)] text-[var(--accent-red)]"
                        : "bg-[var(--bg-surface)] text-[var(--text-muted)]"
                    }`}
                  >
                    {isCompleted ? (
                      <CheckCircle2 className="size-3.5" />
                    ) : isRunning ? (
                      <Play className="size-3.5" />
                    ) : isFailed ? (
                      <XCircle className="size-3.5" />
                    ) : (
                      idx + 1
                    )}
                  </div>

                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-[12px] font-semibold text-[var(--text)]">
                        {step.appId}
                      </span>
                      <ArrowRight className="size-3.5 text-[var(--text-muted)]" />
                      <span className="font-mono text-[12px] text-[var(--text-muted)]">
                        {step.toolName}
                      </span>
                    </div>
                    <div className="text-[12px] text-[var(--text-muted)] mt-1 line-clamp-1">
                      {step.diffSummary || "Autonomous inspection or action execution"}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <span
                    className={`text-[12px] font-mono font-semibold px-2 py-0.5 rounded-[4px] border ${
                      step.riskTier === 3
                        ? "bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--accent-red)]"
                        : step.riskTier === 2
                        ? "bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]"
                        : "bg-[var(--tint-green)] text-[var(--accent-green)] border-[var(--accent-green)]"
                    }`}
                  >
                    Tier {step.riskTier}
                  </span>

                  {isPendingApproval ? (
                    <span className="flex items-center gap-1 text-[12px] font-semibold text-[var(--accent-red)] bg-[var(--tint-red)] px-2 py-0.5 rounded-[4px] border border-[var(--accent-red)]">
                      <AlertTriangle className="size-3.5" />
                      Approval gate
                    </span>
                  ) : isRunning ? (
                    <span className="flex items-center gap-1 text-[12px] font-mono font-medium text-[var(--accent-blue)]">
                      <Clock className="size-3.5" />
                      Running
                    </span>
                  ) : isCompleted ? (
                    <span className="flex items-center gap-1 text-[12px] font-mono font-medium text-[var(--accent-green)]">
                      <Shield className="size-3.5" />
                      Passed
                    </span>
                  ) : (
                    <span className="text-[12px] font-mono text-[var(--text-muted)]">
                      Pending
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </LinearCard>
  );
};
