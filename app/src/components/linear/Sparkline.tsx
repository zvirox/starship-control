import React, { useId } from "react";

export interface SparklineProps {
  data: number[];
  positive?: boolean;
  width?: number;
  height?: number;
  className?: string;
}

export const Sparkline: React.FC<SparklineProps> = ({
  data,
  positive = true,
  width = 84,
  height = 28,
  className = "",
}) => {
  const gradId = useId();
  const points = data.length >= 2 ? data : [data[0] ?? 0, data[0] ?? 0];
  const min = Math.min(...points);
  const max = Math.max(...points);
  const span = max - min || 1;
  const pad = 2;

  const coords = points.map((v, i) => {
    const x = pad + (i / (points.length - 1)) * (width - pad * 2);
    const y = height - pad - ((v - min) / span) * (height - pad * 2);
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const linePoints = coords.join(" ");
  const areaPoints = `${pad},${height} ${linePoints} ${width - pad},${height}`;
  const strokeVar = positive ? "var(--accent-green)" : "var(--accent-red)";

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={`shrink-0 overflow-visible ${className}`}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={strokeVar} stopOpacity="0.24" />
          <stop offset="100%" stopColor={strokeVar} stopOpacity="0" />
        </linearGradient>
      </defs>
      <polygon points={areaPoints} fill={`url(#${gradId})`} />
      <polyline
        points={linePoints}
        fill="none"
        stroke={strokeVar}
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
};
