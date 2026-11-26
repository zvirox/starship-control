import React from "react";

export interface LinearButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
}

export const LinearButton: React.FC<LinearButtonProps> = ({
  variant = "secondary",
  size = "md",
  className = "",
  type = "button",
  children,
  ...props
}) => {
  const sizeStyles = {
    sm: "h-7 px-2.5 text-xs",
    md: "h-8 px-3 text-[13px]",
    lg: "h-10 px-4 text-sm",
  }[size];

  const variantStyles = {
    primary:
      "bg-[var(--btn-primary-bg)] text-[var(--btn-primary-text)] font-medium border border-[var(--btn-primary-bg)] hover:opacity-90 active:opacity-80",
    secondary:
      "bg-[var(--bg-surface-2)] hover:bg-[var(--bg-bar-idle)] text-[var(--text)] border border-[var(--border)] hover:border-[var(--border-strong)] font-medium",
    ghost:
      "bg-transparent hover:bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:text-[var(--text)] border border-transparent",
    danger:
      "bg-[var(--chip-red)] text-[var(--red-text)] border border-[var(--red)] font-medium hover:opacity-90 active:opacity-80",
  }[variant];

  return (
    <button
      type={type}
      className={`inline-flex items-center justify-center gap-1.5 rounded-[6px] font-sans transition-colors duration-150 ease-out focus-ring disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${sizeStyles} ${variantStyles} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
