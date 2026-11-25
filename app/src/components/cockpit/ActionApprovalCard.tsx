import React, { useState, useEffect, useCallback } from "react";
import {
  AlertTriangle,
  Check,
  X,
  ChevronDown,
  ChevronUp,
  ShieldAlert,
  Loader2,
  FileCode,
  CornerDownLeft,
  KeyRound,
} from "lucide-react";
import { LinearCard, LinearButton } from "../linear";
import type { PlanStepDetail } from "../../types/ipc";

export interface ActionApprovalCardProps {
  missionId: number;
  step: PlanStepDetail;
  planTitle?: string;
  onApprove: (stepId: number, signatureToken: string) => Promise<void>;
  onReject: (stepId: number, reason: string) => Promise<void>;
  isExecuting?: boolean;
}

const APP_NAMES: Record<string, string> = {
  eep: "Easy Edits Pro",
  ssp: "ScreenStudio Pro",
  alp: "Audio Labs Pro",
  nwp: "Net Watch Pro",
  atp: "Agentic Tuneup Pro",
};

export const ActionApprovalCard: React.FC<ActionApprovalCardProps> = ({
  missionId: _missionId,
  step,
  planTitle,
  onApprove,
  onReject,
  isExecuting = false,
}) => {
  const [showDiff, setShowDiff] = useState<boolean>(false);
  const [rejecting, setRejecting] = useState<boolean>(false);
  const [rejectReason, setRejectReason] = useState<string>("User declined action modification");
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [typedConfirmation, setTypedConfirmation] = useState<string>("");

  const appDisplayName = APP_NAMES[step.appId.toLowerCase()] || step.appId.toUpperCase();
  const isTier3 = step.riskTier === 3;
  const isIrreversible =
    step.toolName.toLowerCase().includes("purge_package_cache") ||
    step.toolName.toLowerCase().includes("clean_cache") ||
    step.toolName.toLowerCase().includes("format") ||
    step.toolName.toLowerCase().includes("wipe");

  const canApprove =
    !isSubmitting &&
    !isExecuting &&
    Boolean(step.signatureToken) &&
    (!isIrreversible || typedConfirmation.trim().toUpperCase() === "CONFIRM");

  const handleApprove = useCallback(async (e?: React.MouseEvent | React.KeyboardEvent | KeyboardEvent) => {
    // GAP-CLOSURE-PLAN.md §10.4: Frontend approval handler requires event.isTrusted
    if (e && !e.isTrusted) {
      console.warn("Security warning: synthetic untrusted event rejected for action approval");
      return;
    }
    if (isSubmitting || isExecuting || !step.signatureToken) return;
    if (isIrreversible && typedConfirmation.trim().toUpperCase() !== "CONFIRM") return;
    try {
      setIsSubmitting(true);
      await onApprove(step.stepId, step.signatureToken);
    } catch (err) {
      console.error("Approval execution error:", err);
    } finally {
      setIsSubmitting(false);
    }
  }, [isSubmitting, isExecuting, onApprove, step.signatureToken, step.stepId, isIrreversible, typedConfirmation]);

  const handleReject = useCallback(async (e?: React.MouseEvent | React.KeyboardEvent | KeyboardEvent) => {
    if (e && !e.isTrusted) {
      console.warn("Security warning: synthetic untrusted event rejected for action rejection");
      return;
    }
    if (isSubmitting || isExecuting) return;
    try {
      setIsSubmitting(true);
      await onReject(step.stepId, rejectReason);
      setRejecting(false);
    } catch (err) {
      console.error("Rejection error:", err);
    } finally {
      setIsSubmitting(false);
    }
  }, [isSubmitting, isExecuting, onReject, rejectReason, step.stepId]);

  // Global keyboard shortcuts: Enter to Approve, Esc to Reject
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't trigger if user is actively typing in an input
      if (
        document.activeElement?.tagName === "INPUT" ||
        document.activeElement?.tagName === "TEXTAREA"
      ) {
        if (e.key === "Escape" && rejecting) {
          setRejecting(false);
          e.preventDefault();
        }
        return;
      }

      if (e.key === "Enter" && !rejecting) {
        e.preventDefault();
        handleApprove();
      } else if (e.key === "Escape") {
        e.preventDefault();
        if (!rejecting) {
          setRejecting(true);
        } else {
          handleReject();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleApprove, handleReject, rejecting]);

  return (
    <div
      role="alertdialog"
      aria-modal="true"
      aria-labelledby={`approval-title-${step.stepId}`}
      className="surface-panel relative my-4 overflow-hidden rounded-[8px] border border-[var(--accent-red)] p-5 transition-colors duration-150"
    >
      <div className="mb-4 flex items-center justify-between border-b border-[var(--border)] pb-3">
        <div className="flex items-center gap-2">
          <div className="flex size-8 items-center justify-center rounded-[6px] bg-[var(--tint-red)] text-[var(--accent-red)] border border-[var(--accent-red)]">
            <ShieldAlert className="size-4" />
          </div>
          <div>
            <h3
              id={`approval-title-${step.stepId}`}
              className="font-sans text-[14px] font-semibold text-[var(--text)] flex items-center gap-2"
            >
              <span>Approval required</span>
              <span className="text-[var(--text-muted)]">·</span>
              <span className="text-[var(--text-muted)] font-medium">{appDisplayName}</span>
            </h3>
            {planTitle && (
              <p className="font-sans text-[12px] text-[var(--text-muted)]">Mission plan: {planTitle}</p>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {isIrreversible && (
            <span className="flex items-center gap-1 rounded-[4px] px-2 py-0.5 text-[12px] font-mono font-semibold border border-[var(--accent-red)] bg-[var(--tint-red)] text-[var(--accent-red)]">
              <AlertTriangle className="size-3.5" />
              <span>Irreversible</span>
            </span>
          )}
          <span
            className={`flex items-center gap-1 rounded-[4px] px-2 py-0.5 text-[12px] font-mono font-semibold border ${
              isTier3
                ? "bg-[var(--tint-red)] text-[var(--accent-red)] border-[var(--accent-red)]"
                : "bg-[var(--tint-amber)] text-[var(--accent-amber)] border-[var(--accent-amber)]"
            }`}
          >
            <AlertTriangle className="size-3.5" />
            <span>{isTier3 ? "Tier 3 · destructive" : "Tier 2 · mutating"}</span>
          </span>
        </div>
      </div>

      <div className="space-y-3 font-sans text-[12px] leading-relaxed text-[var(--text)]">
        <div className="flex items-start gap-2">
          <span className="font-semibold text-[var(--text-muted)] min-w-[70px]">Action:</span>
          <span className="font-semibold text-[var(--text)]">
            {step.diffSummary || `Execute tool: ${step.toolName}`}
          </span>
        </div>

        <div className="flex items-start gap-2">
          <span className="font-semibold text-[var(--text-muted)] min-w-[70px]">Tool:</span>
          <span className="font-mono text-[12px] text-[var(--text)] font-medium bg-[var(--bg-surface-2)] px-2 py-0.5 rounded-[4px] border border-[var(--border)]">
            {step.toolName}
          </span>
          <span className="text-[var(--text-muted)] ml-1">(Step #{step.stepIndex + 1})</span>
        </div>

        <div className="flex items-center gap-3 pt-2 text-[12px] text-[var(--text-muted)] border-t border-[var(--border)]">
          <span className="flex items-center gap-1">
            <KeyRound className="size-3.5 text-[var(--accent-green)]" />
            <span>Signed token:</span>
            <code className="font-mono text-[var(--accent-green)] bg-[var(--tint-green)] px-2 py-0.5 rounded-[4px] text-[12px]">
              {step.signatureToken ? `${step.signatureToken.slice(0, 16)}...` : "None"}
            </code>
          </span>
          <span>·</span>
          <span>Checkpoint: <strong className="font-medium text-[var(--text)]">#bk-01 (Auto-save)</strong></span>
        </div>
      </div>

      <div className="mt-4">
        <button
          type="button"
          onClick={() => setShowDiff((prev) => !prev)}
          className="flex items-center gap-2 font-mono text-[12px] text-[var(--text-muted)] hover:text-[var(--text)] transition-colors py-1 focus-ring rounded-[4px] cursor-pointer"
        >
          {showDiff ? <ChevronUp className="size-3.5" /> : <ChevronDown className="size-3.5" />}
          <span className="flex items-center gap-1">
            <FileCode className="size-3.5" />
            {showDiff ? "Hide parameters and diff" : "View parameters and diff"}
          </span>
        </button>

        {showDiff && (
          <LinearCard
            level={2}
            className="mt-2 p-3 font-mono text-[12px] text-[var(--text-muted)] max-h-48 overflow-y-auto"
          >
            <div className="text-[var(--text-muted)] font-medium mb-1">// Parameters for {step.toolName}</div>
            <pre className="whitespace-pre-wrap text-[var(--text)]">
              {JSON.stringify(step.arguments, null, 2)}
            </pre>
          </LinearCard>
        )}
      </div>

      {rejecting && (
        <div className="mt-4 p-3 rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] space-y-2">
          <label htmlFor={`reject-reason-${step.stepId}`} className="block text-[12px] font-semibold text-[var(--accent-red)]">
            Rejection reason or guidance for planner
          </label>
          <input
            id={`reject-reason-${step.stepId}`}
            type="text"
            value={rejectReason}
            onChange={(e) => setRejectReason(e.target.value)}
            placeholder="Reason for rejection"
            className="w-full rounded-[6px] bg-[var(--bg-surface)] border border-[var(--border)] px-3 py-2 font-sans text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring focus:border-[var(--border-strong)]"
            autoFocus
          />
          <div className="flex justify-end gap-2 pt-1">
            <LinearButton type="button" variant="secondary" size="sm" onClick={() => setRejecting(false)}>
              Cancel
            </LinearButton>
            <LinearButton type="button" variant="danger" size="sm" onClick={handleReject} disabled={isSubmitting}>
              <X className="size-3.5" />
              <span>Confirm rejection</span>
            </LinearButton>
          </div>
        </div>
      )}

      {isIrreversible && !rejecting && (
        <div className="mt-4 rounded-[8px] border border-[var(--accent-red)] bg-[var(--tint-red)] p-3 space-y-2">
          <div className="flex items-center gap-2 text-[var(--accent-red)] text-[12px] font-medium">
            <AlertTriangle className="size-4 shrink-0" />
            <span>Permanent action — cannot be restored from quarantine</span>
          </div>
          <p className="text-[12px] text-[var(--text-muted)] leading-relaxed font-sans">
            Package cache cleanup cannot be rolled back. Type <strong className="text-[var(--accent-red)] font-mono">CONFIRM</strong> to authorize.
          </p>
          <input
            type="text"
            value={typedConfirmation}
            onChange={(e) => setTypedConfirmation(e.target.value)}
            placeholder="Type CONFIRM"
            className="w-full rounded-[6px] bg-[var(--bg-surface)] border border-[var(--accent-red)] px-3 py-2 font-mono text-[12px] text-[var(--text)] placeholder:text-[var(--text-faint)] focus-ring"
          />
        </div>
      )}

      {!rejecting && (
        <div className="mt-5 flex items-center justify-between border-t border-[var(--border)] pt-4">
          <div className="flex items-center gap-2 text-[12px] text-[var(--text-muted)] font-sans">
            <span className="hidden sm:inline">Shortcuts</span>
            <kbd className="keycap gap-1">
              <span>Enter</span>
              <CornerDownLeft className="size-3.5" />
            </kbd>
            <span>Approve</span>
            <kbd className="keycap">Esc</kbd>
            <span>Reject</span>
          </div>

          <div className="flex items-center gap-3">
            <LinearButton
              type="button"
              variant="secondary"
              size="md"
              onClick={() => setRejecting(true)}
              disabled={isSubmitting || isExecuting}
            >
              <X className="size-3.5" />
              <span>Reject or modify</span>
            </LinearButton>

            <LinearButton
              type="button"
              variant="primary"
              size="md"
              onClick={handleApprove}
              disabled={!canApprove}
            >
              {isSubmitting || isExecuting ? (
                <>
                  <Loader2 className="size-3.5 animate-spin" />
                  <span>Dispatching</span>
                </>
              ) : (
                <>
                  <Check className="size-3.5" />
                  <span>Approve action</span>
                </>
              )}
            </LinearButton>
          </div>
        </div>
      )}
    </div>
  );
};
