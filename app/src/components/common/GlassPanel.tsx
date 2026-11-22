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
    pane: "bg-white/[0.03] backdrop-blur-md border border-white/[0.06] rounded-2xl shadow-sm",
    float: "bg-[#12151B]/95 backdrop-blur-xl border border-white/[0.08] rounded-2xl shadow-2xl",
    modal: "bg-[#14171E] border border-white/[0.1] rounded-3xl shadow-2xl",
    solid: "bg-[#12151B] border border-white/[0.06] rounded-2xl",
  }[recipe];

  const glowClass = {
    none: "",
    cyan: "border-sky-500/40 shadow-lg",
    rose: "border-rose-500/40 shadow-lg",
    amber: "border-amber-500/40 shadow-lg",
    emerald: "border-emerald-500/40 shadow-lg",
    violet: "border-purple-500/40 shadow-lg",
  }[glow];

  return (
    <div
      className={`relative overflow-hidden transition-all duration-200 ${recipeClass} ${glowClass} ${className}`}
      {...props}
    >
      {specular && recipe !== "pane" && (
        <div className="pointer-events-none absolute inset-0 bg-gradient-to-br from-white/[0.05] via-transparent to-transparent opacity-60" />
      )}
      <div className="relative z-10">{children}</div>
    </div>
  );
};
