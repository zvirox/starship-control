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

  const appDisplayName = APP_NAMES[step.appId.toLowerCase()] || step.appId.toUpperCase();
  const isTier3 = step.riskTier === 3;

  const handleApprove = useCallback(async () => {
    if (isSubmitting || isExecuting || !step.signatureToken) return;
    try {
      setIsSubmitting(true);
      await onApprove(step.stepId, step.signatureToken);
    } catch (err) {
      console.error("Approval execution error:", err);
    } finally {
      setIsSubmitting(false);
    }
  }, [isSubmitting, isExecuting, onApprove, step.signatureToken, step.stepId]);

  const handleReject = useCallback(async () => {
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
      className="relative my-4 overflow-hidden rounded-2xl border-2 border-rose-500/80 bg-[#0E131B]/95 p-5 shadow-[0_0_32px_rgba(244,63,94,0.30)] backdrop-blur-2xl transition-all duration-300 animate-in fade-in zoom-in-95"
    >
      {/* Specular highlight border overlay */}
      <div className="pointer-events-none absolute inset-0 bg-gradient-to-br from-white/10 via-transparent to-rose-500/5" />

      {/* Pulsating Rose Gate Banner */}
      <div className="mb-4 flex items-center justify-between border-b border-rose-500/30 pb-3">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-rose-500/20 text-rose-400 border border-rose-500/40 shadow-[0_0_12px_rgba(244,63,94,0.4)]">
            <ShieldAlert className="h-4 w-4 animate-pulse" />
          </div>
          <div>
            <h3
              id={`approval-title-${step.stepId}`}
              className="font-sans text-sm font-bold tracking-tight text-slate-100 flex items-center gap-2"
            >
              <span>HUMAN APPROVAL REQUIRED</span>
              <span className="text-slate-400">·</span>
              <span className="text-sky-400">{appDisplayName}</span>
            </h3>
            {planTitle && (
              <p className="font-sans text-xs text-slate-400">Mission Plan: {planTitle}</p>
            )}
          </div>
        </div>

        {/* Risk Tier Badge */}
        <div
          className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-[11px] font-bold font-mono tracking-wider uppercase border shadow-sm ${
            isTier3
              ? "bg-rose-500/20 text-rose-300 border-rose-500/40 shadow-rose-500/20"
              : "bg-amber-500/20 text-amber-300 border-amber-500/40 shadow-amber-500/20"
          }`}
        >
          <AlertTriangle className="h-3 w-3" />
          <span>{isTier3 ? "TIER 3 (DESTRUCTIVE)" : "TIER 2 (MUTATING)"}</span>
        </div>
      </div>

      {/* Impact & Action Description */}
      <div className="space-y-3 font-sans text-xs leading-relaxed text-slate-200">
        <div className="flex items-start gap-2">
          <span className="font-semibold text-slate-400 min-w-[70px]">Action:</span>
          <span className="font-medium text-slate-100">
            {step.diffSummary || `Execute tool: ${step.toolName}`}
          </span>
        </div>

        <div className="flex items-start gap-2">
          <span className="font-semibold text-slate-400 min-w-[70px]">Tool:</span>
          <span className="font-mono text-[11px] text-cyan-300 bg-cyan-950/40 px-2 py-0.5 rounded border border-cyan-500/20">
            {step.toolName}
          </span>
          <span className="text-slate-500 ml-1">(Step #{step.stepIndex + 1})</span>
        </div>

        <div className="flex items-center gap-3 pt-1 text-[11px] text-slate-400 border-t border-white/5">
          <span className="flex items-center gap-1">
            <KeyRound className="h-3 w-3 text-emerald-400" />
            <span>Pre-signed Kernel Signature:</span>
            <code className="font-mono text-emerald-300 bg-emerald-950/40 px-1.5 py-0.5 rounded text-[10px]">
              {step.signatureToken ? `${step.signatureToken.slice(0, 16)}...` : "None"}
            </code>
          </span>
          <span>·</span>
          <span>Checkpoint: <strong className="text-slate-200">#bk-01 (Auto-save)</strong></span>
        </div>
      </div>

      {/* Collapsible Diff / Parameter Viewer */}
      <div className="mt-4">
        <button
          type="button"
          onClick={() => setShowDiff((prev) => !prev)}
          className="flex items-center gap-1.5 font-mono text-[11px] text-slate-400 hover:text-slate-200 transition-colors py-1 focus-visible:outline-none focus-visible:text-sky-400"
        >
          {showDiff ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
          <span className="flex items-center gap-1">
            <FileCode className="h-3.5 w-3.5 text-cyan-400" />
            {showDiff ? "Hide Detailed Parameters & Diff" : "View Diff & Tool Parameters"}
          </span>
        </button>

        {showDiff && (
          <GlassPanel
            recipe="pane"
            className="mt-2 p-3 font-mono text-[11px] text-slate-300 bg-black/60 border border-white/10 rounded-xl max-h-48 overflow-y-auto"
          >
            <div className="text-slate-500 mb-1">// Parameters to be passed to {step.toolName}:</div>
            <pre className="whitespace-pre-wrap text-cyan-200">
              {JSON.stringify(step.arguments, null, 2)}
            </pre>
          </GlassPanel>
        )}
      </div>

      {/* Rejection Reason Sub-form */}
      {rejecting && (
        <div className="mt-4 p-3 rounded-xl border border-rose-500/30 bg-rose-500/10 space-y-2">
          <label htmlFor={`reject-reason-${step.stepId}`} className="block text-[11px] font-semibold text-rose-300">
            Rejection Reason / Guidance for Planner:
          </label>
          <input
            id={`reject-reason-${step.stepId}`}
            type="text"
            value={rejectReason}
            onChange={(e) => setRejectReason(e.target.value)}
            placeholder="e.g. Do not modify timeline cuts yet"
            className="w-full rounded-lg bg-black/50 border border-rose-500/40 px-3 py-1.5 font-sans text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-rose-400"
            autoFocus
          />
          <div className="flex justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={() => setRejecting(false)}
              className="px-3 py-1 rounded-lg text-xs text-slate-300 hover:bg-white/10 transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={handleReject}
              disabled={isSubmitting}
              className="flex items-center gap-1 px-3 py-1 rounded-lg text-xs font-semibold bg-rose-600 text-white hover:bg-rose-500 transition-colors"
            >
              <X className="h-3 w-3" />
              Confirm Rejection
            </button>
          </div>
        </div>
      )}

      {/* Action Buttons */}
      {!rejecting && (
        <div className="mt-5 flex items-center justify-between border-t border-white/10 pt-4">
          <div className="flex items-center gap-2 text-[11px] text-slate-400 font-sans">
            <span className="hidden sm:inline">Shortcuts:</span>
            <kbd className="flex items-center gap-1 rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[10px] text-slate-300 border border-slate-700">
              <span>Enter</span>
              <CornerDownLeft className="h-2.5 w-2.5" />
            </kbd>
            <span className="text-slate-500">Approve</span>
            <kbd className="rounded bg-slate-800 px-1.5 py-0.5 font-mono text-[10px] text-slate-300 border border-slate-700">
              Esc
            </kbd>
            <span className="text-slate-500">Reject</span>
          </div>

          <div className="flex items-center gap-3">
            {/* Reject Button */}
            <button
              type="button"
              onClick={() => setRejecting(true)}
              disabled={isSubmitting || isExecuting}
              className="flex items-center gap-1.5 rounded-xl border border-white/15 bg-white/5 px-4 py-2 font-sans text-xs font-medium text-slate-300 hover:border-rose-500/40 hover:bg-rose-500/10 hover:text-rose-300 focus-visible:ring-2 focus-visible:ring-rose-400 transition-all duration-150 cursor-pointer"
            >
              <X className="h-3.5 w-3.5 text-rose-400" />
              <span>Reject / Modify</span>
            </button>

            {/* Approve Button */}
            <button
              type="button"
              onClick={handleApprove}
              disabled={isSubmitting || isExecuting}
              className="flex items-center gap-1.5 rounded-xl bg-gradient-to-r from-sky-400 to-cyan-400 px-5 py-2 font-sans text-xs font-bold text-slate-950 shadow-[0_0_20px_rgba(56,189,248,0.4)] hover:shadow-[0_0_28px_rgba(56,189,248,0.6)] hover:brightness-110 active:scale-98 focus-visible:ring-2 focus-visible:ring-sky-300 transition-all duration-150 cursor-pointer"
            >
              {isSubmitting || isExecuting ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  <span>Dispatching...</span>
                </>
              ) : (
                <>
                  <Check className="h-3.5 w-3.5 stroke-[2.5]" />
                  <span>Approve Action</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
