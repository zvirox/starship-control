/**
 * Neutralizes executable content before it reaches the webview.
 * Shared by the React renderer and the Node security test suite.
 *
 * @param {string} content
 * @returns {string}
 */
export function sanitizeMarkdownText(content) {
  return content
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, "[script stripped]")
    // CHECK:javascript-scheme — removing this line breaks the mutation test
    .replace(/javascript\s*:/gi, "blocked-scheme:")
    .replace(/data\s*:\s*text\/html/gi, "blocked-scheme:");
}
