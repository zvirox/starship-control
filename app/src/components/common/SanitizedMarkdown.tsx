import React, { useMemo } from "react";
import { sanitizeMarkdownText } from "../../lib/sanitize.js";

export interface SanitizedMarkdownProps {
  content: string;
  className?: string;
}

/**
 * Renders text/markdown safely while neutralizing XSS payloads (GAP-CLOSURE-PLAN.md §10.4).
 * Raw HTML is never injected: the result is rendered as an escaped React text node.
 */
export const SanitizedMarkdown: React.FC<SanitizedMarkdownProps> = ({
  content,
  className = "",
}) => {
  const safeText = useMemo(() => sanitizeMarkdownText(content), [content]);

  return (
    <div className={`whitespace-pre-wrap leading-relaxed ${className}`}>
      {safeText}
    </div>
  );
};
