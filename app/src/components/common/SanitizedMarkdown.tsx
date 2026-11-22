import React, { useMemo } from "react";

export interface SanitizedMarkdownProps {
  content: string;
  className?: string;
}

/**
 * Renders text/markdown safely while neutralizing XSS payloads (GAP-CLOSURE-PLAN.md §10.4).
 * Strictly forbids:
 * - Raw HTML execution (<script>, <iframe>, <embed>, <object>, inline event handlers)
 * - `javascript:` and `data:text/html` URIs in links
 */
export const SanitizedMarkdown: React.FC<SanitizedMarkdownProps> = ({
  content,
  className = "",
}) => {
  const sanitizedSegments = useMemo(() => {
    // Sanitize any malicious javascript: links or script tags
    const safeText = content
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "[script stripped]")
      .replace(/javascript\s*:/gi, "blocked-scheme:")
      .replace(/data\s*:\s*text\/html/gi, "blocked-scheme:");

    return safeText;
  }, [content]);

  // Render purely as escaped text node in pre-wrapped layout, ensuring absolute safety
  return (
    <div className={`whitespace-pre-wrap leading-relaxed ${className}`}>
      {sanitizedSegments}
    </div>
  );
};
