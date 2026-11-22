import React from "react";

export interface GlassPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  recipe?: "pane" | "float" | "modal" | "solid";
  glow?: "cyan" | "rose" | "amber" | "emerald" | "violet" | "none";
  specular?: boolean;
}

export const GlassPanel: React.FC<GlassPanelProps> = ({
  recipe = "pane",
  glow = "none",
  specular = true,
  className = "",
  children,
  ...props
}) => {
  const recipeClass = {
    pane: "glass-pane rounded-2xl",
    float: "glass-float rounded-2xl",
    modal: "glass-modal rounded-3xl",
    solid: "bg-[#141B26] border border-white/10 rounded-2xl",
  }[recipe];

  const glowClass = {
    none: "",
    cyan: "shadow-[0_0_24px_rgba(56,189,248,0.25)] border-cyan-400/40",
    rose: "shadow-[0_0_24px_rgba(244,63,94,0.30)] border-rose-500/50",
    amber: "shadow-[0_0_24px_rgba(245,158,11,0.25)] border-amber-500/40",
    emerald: "shadow-[0_0_24px_rgba(16,185,129,0.25)] border-emerald-500/40",
    violet: "shadow-[0_0_24px_rgba(168,85,247,0.25)] border-purple-500/40",
  }[glow];

  return (
    <div
      className={`relative overflow-hidden transition-all duration-200 ${recipeClass} ${glowClass} ${className}`}
      {...props}
    >
      {specular && recipe !== "pane" && (
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-br from-white/10 via-transparent to-transparent opacity-80" />
      )}
      <div className="relative z-10">{children}</div>
    </div>
  );
};
