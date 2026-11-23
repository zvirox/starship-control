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
import { GlassPanel } from "../common/GlassPanel";
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
      className="relative my-4 overflow-hidden rounded-2xl border border-signal-gate/50 bg-surface-solid p-5 transition-colors duration-150"
    >
      {/* Gate Banner */}
      <div className="mb-4 flex items-center justify-between border-b border-signal-gate/30 pb-3">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-signal-gate-bg text-signal-gate border border-signal-gate/40">
            <ShieldAlert className="h-4 w-4" />
          </div>
          <div>
            <h3
              id={`approval-title-${step.stepId}`}
              className="font-sans text-sm font-medium text-text-primary flex items-center gap-2"
            >
              <span>Approval required</span>
              <span className="text-text-disabled">·</span>
              <span className="text-text-secondary">{appDisplayName}</span>
            </h3>
            {planTitle && (
              <p className="font-sans text-xs text-text-secondary">Mission plan: {planTitle}</p>
            )}
          </div>
        </div>

        {/* Risk Tier & Irreversible Chips */}
        <div className="flex items-center gap-2">
          {isIrreversible && (
            <span className="flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-mono border border-signal-gate/40 bg-signal-gate-bg text-signal-gate">
              <AlertTriangle className="h-3.5 w-3.5" />
              <span>Irreversible</span>
            </span>
          )}
          <span
            className={`flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] font-mono border ${
              isTier3
                ? "bg-signal-gate-bg text-signal-gate border-signal-gate/40"
                : "bg-signal-thinking-bg text-signal-thinking border-signal-thinking/40"
            }`}
          >
            <AlertTriangle className="h-3.5 w-3.5" />
            <span>{isTier3 ? "Tier 3 · destructive" : "Tier 2 · mutating"}</span>
          </span>
        </div>
      </div>

      {/* Impact & Action Description */}
      <div className="space-y-3 font-sans text-xs leading-relaxed text-text-primary">
        <div className="flex items-start gap-2">
          <span className="font-medium text-text-secondary min-w-[70px]">Action:</span>
          <span className="font-medium text-text-primary">
            {step.diffSummary || `Execute tool: ${step.toolName}`}
          </span>
        </div>

        <div className="flex items-start gap-2">
          <span className="font-medium text-text-secondary min-w-[70px]">Tool:</span>
          <span className="font-mono text-[11px] text-text-primary bg-white/[0.05] px-2 py-1 rounded-lg border border-border-subtle">
            {step.toolName}
          </span>
          <span className="text-text-disabled ml-1">(Step #{step.stepIndex + 1})</span>
        </div>

        <div className="flex items-center gap-3 pt-2 text-[11px] text-text-secondary border-t border-border-subtle">
          <span className="flex items-center gap-1">
            <KeyRound className="h-3.5 w-3.5 text-signal-success" />
            <span>Signed token:</span>
            <code className="font-mono text-signal-success bg-signal-success-bg px-2 py-1 rounded-lg text-[11px]">
              {step.signatureToken ? `${step.signatureToken.slice(0, 16)}...` : "None"}
            </code>
          </span>
          <span>·</span>
          <span>Checkpoint: <strong className="font-medium text-text-primary">#bk-01 (Auto-save)</strong></span>
        </div>
      </div>

      {/* Collapsible Diff / Parameter Viewer */}
      <div className="mt-4">
        <button
          type="button"
          onClick={() => setShowDiff((prev) => !prev)}
          className="flex items-center gap-2 font-mono text-[11px] text-text-secondary hover:text-text-primary transition-colors py-1 focus-visible:outline-none focus-visible:text-accent"
        >
          {showDiff ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
          <span className="flex items-center gap-1">
            <FileCode className="h-3.5 w-3.5" />
            {showDiff ? "Hide parameters and diff" : "View parameters and diff"}
          </span>
        </button>

        {showDiff && (
          <GlassPanel
            recipe="pane"
            className="mt-2 p-3 font-mono text-[11px] text-text-secondary bg-black/40 border border-border-subtle rounded-xl max-h-48 overflow-y-auto"
          >
            <div className="text-text-disabled mb-1">// Parameters for {step.toolName}</div>
            <pre className="whitespace-pre-wrap text-text-primary">
              {JSON.stringify(step.arguments, null, 2)}
            </pre>
          </GlassPanel>
        )}
      </div>

      {/* Rejection Reason Sub-form */}
      {rejecting && (
        <div className="mt-4 p-3 rounded-xl border border-signal-gate/30 bg-signal-gate-bg space-y-2">
          <label htmlFor={`reject-reason-${step.stepId}`} className="block text-[11px] font-medium text-signal-gate">
            Rejection reason or guidance for planner
          </label>
          <input
            id={`reject-reason-${step.stepId}`}
            type="text"
            value={rejectReason}
            onChange={(e) => setRejectReason(e.target.value)}
            placeholder="Reason for rejection"
            className="w-full rounded-xl bg-black/40 border border-border-subtle px-3 py-2 font-sans text-xs text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-accent transition-colors"
            autoFocus
          />
          <div className="flex justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={() => setRejecting(false)}
              className="px-3 py-2 rounded-lg text-xs text-text-secondary hover:bg-white/[0.05] transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleReject}
              disabled={isSubmitting}
              className="flex items-center gap-1 px-3 py-2 rounded-lg border border-border-subtle bg-raised text-xs font-medium text-text-primary hover:bg-white/[0.05] transition-colors"
            >
              <X className="h-3.5 w-3.5" />
              Confirm rejection
            </button>
          </div>
        </div>
      )}

      {/* Irreversible Confirmation Requirement (GAP-CLOSURE-PLAN.md §11.8) */}
      {isIrreversible && !rejecting && (
        <div className="mt-4 rounded-xl border border-signal-gate/40 bg-signal-gate-bg p-3 space-y-2">
          <div className="flex items-center gap-2 text-signal-gate text-xs font-medium">
            <AlertTriangle className="h-4 w-4 shrink-0" />
            <span>Permanent action — cannot be restored from quarantine</span>
          </div>
          <p className="text-[11px] text-text-secondary leading-relaxed font-sans">
            Package cache cleanup cannot be rolled back. Type <strong className="text-signal-gate font-mono">CONFIRM</strong> to authorize.
          </p>
          <input
            type="text"
            value={typedConfirmation}
            onChange={(e) => setTypedConfirmation(e.target.value)}
            placeholder="Type CONFIRM"
            className="w-full rounded-xl bg-black/40 border border-signal-gate/40 px-3 py-2 font-mono text-xs text-text-primary placeholder:text-text-disabled focus:outline-none focus:border-accent transition-colors"
          />
        </div>
      )}

      {/* Action Buttons */}
      {!rejecting && (
        <div className="mt-5 flex items-center justify-between border-t border-border-subtle pt-4">
          <div className="flex items-center gap-2 text-[11px] text-text-secondary font-sans">
            <span className="hidden sm:inline">Shortcuts</span>
            <kbd className="keycap gap-1">
              <span>Enter</span>
              <CornerDownLeft className="h-3.5 w-3.5" />
            </kbd>
            <span className="text-text-disabled">Approve</span>
            <kbd className="keycap">Esc</kbd>
            <span className="text-text-disabled">Reject</span>
          </div>

          <div className="flex items-center gap-3">
            {/* Reject Button */}
            <button
              type="button"
              onClick={() => setRejecting(true)}
              disabled={isSubmitting || isExecuting}
              className="flex items-center gap-2 rounded-xl border border-border-subtle bg-raised px-4 py-2 font-sans text-xs font-medium text-text-secondary hover:bg-white/[0.05] hover:text-text-primary transition-colors duration-150 cursor-pointer"
            >
              <X className="h-3.5 w-3.5" />
              <span>Reject or modify</span>
            </button>

            {/* Approve Button */}
            <button
              type="button"
              onClick={handleApprove}
              disabled={!canApprove}
              className={`flex items-center gap-2 rounded-xl px-5 py-2 font-sans text-xs font-medium transition-colors duration-150 ${
                canApprove
                  ? "bg-accent text-accent-text hover:bg-accent/90 active:scale-[0.98] cursor-pointer"
                  : "bg-raised text-text-disabled cursor-not-allowed border border-border-subtle"
              }`}
            >
              {isSubmitting || isExecuting ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>Dispatching</span>
                </>
              ) : (
                <>
                  <Check className="h-3.5 w-3.5" />
                  <span>Approve action</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
