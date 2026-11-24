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
    pane: "glass-pane rounded-2xl",
    float: "glass-float rounded-2xl",
    modal: "glass-modal rounded-2xl",
    solid: "bg-surface-solid border border-border-subtle rounded-2xl",
  }[recipe];

  return (
    <div className={`relative transition-colors duration-150 ${recipeClass} ${className}`} {...props}>
      {children}
    </div>
  );
};
