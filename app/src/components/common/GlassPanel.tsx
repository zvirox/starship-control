import React from "react";
import { LinearCard } from "../linear/LinearCard";

export interface GlassPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  recipe?: "pane" | "float" | "modal" | "solid";
}

export const GlassPanel: React.FC<GlassPanelProps> = ({
  recipe = "pane",
  className = "",
  children,
  ...props
}) => {
  const levelMap: Record<string, 1 | 2 | 3> = {
    pane: 1,
    float: 1,
    modal: 1,
    solid: 2,
  };

  return (
    <LinearCard
      level={levelMap[recipe] ?? 1}
      className={`relative rounded-2xl ${className}`}
      {...props}
    >
      {children}
    </LinearCard>
  );
};
