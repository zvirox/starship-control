import React from "react";

export interface GlassPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  recipe?: "pane" | "float" | "modal" | "solid";
}

export const GlassPanel: React.FC<GlassPanelProps> = ({
  recipe = "pane",
  className = "",
  children,
  ...props
}) => {
  const recipeClass = {
    pane: "bg-surface-glass backdrop-blur-2xl border border-border-glass rounded-2xl",
    float: "bg-surface-glass-strong backdrop-blur-3xl border border-border-glass rounded-2xl shadow-[0_8px_32px_-4px_rgba(0,0,0,0.5)]",
    modal: "bg-surface-glass-strong border border-border-subtle rounded-2xl shadow-[0_16px_48px_-8px_rgba(0,0,0,0.65)]",
    solid: "bg-surface-solid border border-border-subtle rounded-2xl",
  }[recipe];

  return (
    <div className={`relative transition-colors duration-150 ${recipeClass} ${className}`} {...props}>
      {children}
    </div>
  );
};
