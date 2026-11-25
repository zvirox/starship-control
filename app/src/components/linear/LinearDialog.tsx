import React, { useEffect } from "react";
import { X } from "lucide-react";
import { LinearCard } from "./LinearCard";

export interface LinearDialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  icon?: React.ReactNode;
  maxWidthClass?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}

export const LinearDialog: React.FC<LinearDialogProps> = ({
  open,
  onClose,
  title,
  icon,
  maxWidthClass = "max-w-lg",
  children,
  footer,
}) => {
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={title}
      className="fixed inset-0 z-50 flex items-center justify-center bg-[var(--bg-app)] p-5 font-sans select-none"
    >
      <LinearCard
        level={3}
        className={`w-full ${maxWidthClass} p-5 space-y-4`}
      >
        <div className="flex items-center justify-between border-b border-[var(--border)] pb-3">
          <div className="flex items-center gap-2">
            {icon}
            <h3 className="text-[14px] font-semibold text-[var(--text)]">
              {title}
            </h3>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close dialog"
            className="size-7 inline-flex items-center justify-center rounded-[6px] text-[var(--text-muted)] hover:bg-[var(--bg-surface-2)] hover:text-[var(--text)] transition-colors duration-150 focus-ring cursor-pointer"
          >
            <X className="size-4" strokeWidth={1.75} />
          </button>
        </div>

        <div>{children}</div>

        {footer && (
          <div className="flex items-center justify-between border-t border-[var(--border)] pt-3">
            {footer}
          </div>
        )}
      </LinearCard>
    </div>
  );
};
