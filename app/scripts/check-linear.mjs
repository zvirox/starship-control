#!/usr/bin/env node
/**
 * Linear palette gate. Colors live only in src/styles/linear-theme.css.
 * No blur, translucency, glow, extra shadows or retired palette values.
 */
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "src");
const TOKEN_PATH = "src/styles/linear-theme.css";
const TOKEN_FILES = new Set([TOKEN_PATH]);

const RETIRED = [
  "#050505", "#06B6D4", "#08080A", "#0891B2", "#09090B", "#0F1115", "#10B981",
  "#111213", "#121215", "#17171C", "#1A1B1E", "#1F2125", "#1FB5A5", "#202026",
  "#202128", "#22C55E", "#22D3EE", "#23232A", "#2C2E33", "#33333D", "#3355FF",
  "#38B6F0", "#5A5B62", "#6B7280", "#71717A", "#7C3AED", "#7C5CE6", "#8A8B91",
  "#8E8E93", "#9CA3AF", "#A1A1AA", "#A855F7", "#C8C8D0", "#E11D48", "#E2E2E7",
  "#E8E8EC", "#EAB308", "#EAF0FF", "#EC4899", "#F2F2F4", "#F43F5E", "#F4F4F5",
  "#F4F5F7", "#F59E0B", "#F5F5F6", "#F7F7F8", "#F97316",
];

function findFiles(dir, exts, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) findFiles(full, exts, acc);
    else if (exts.some((ext) => entry.endsWith(ext))) acc.push(full);
  }
  return acc;
}

const RULES = [
  {
    name: "blur / backdrop-filter",
    regex: /\bbackdrop-blur\b|(?:-webkit-)?(?:backdrop-filter|filter)\s*:\s*blur\(/i,
    skipTokenFiles: false,
  },
  {
    name: "translucency outside the token file",
    regex: /\b(?:rgba|hsla)\s*\(|\bbg-(?:white|black)\/|\b(?:bg|border|surface)-[a-z0-9-]+\/(?:\d+|\[)/i,
    skipTokenFiles: true,
  },
  {
    name: "glow / drop-shadow / glass",
    regex: /\bdrop-shadow\b|glass-pane|glass-float|glass-modal/i,
    skipTokenFiles: false,
  },
  {
    name: "box-shadow other than var(--shadow-card)",
    regex: /box-shadow\s*:(?!\s*var\(--shadow-card\))|(?:shadow-(?:2xs|xs|sm|md|lg|xl|2xl|inner)|shadow-\[)/i,
    skipTokenFiles: true,
  },
  {
    name: "literal color outside the token file",
    regex:
      /(?:bg|text|border|ring|fill|stroke|from|to|via|accent|outline)-\[#[0-9a-fA-F]{3,8}\]|['"]#[0-9a-fA-F]{3,8}['"]|#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/i,
    skipTokenFiles: true,
  },
  {
    name: "Tailwind default palette class",
    regex:
      /\b(?:bg|text|border|ring|fill|stroke|from|to|via|shadow|divide|accent|caret|outline|decoration)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)-\d/i,
    skipTokenFiles: true,
  },
];

const files = findFiles(UI_SRC, [".tsx", ".ts", ".css", ".mjs", ".js"]);
const violations = [];

for (const file of files) {
  const relPath = relative(APP_ROOT, file).replace(/\\/g, "/");
  const isTokenFile = TOKEN_FILES.has(relPath);
  const lines = readFileSync(file, "utf8").split(/\r?\n/);

  lines.forEach((line, idx) => {
    const trimmed = line.trim();
    if (
      trimmed.startsWith("//") ||
      trimmed.startsWith("*") ||
      trimmed.startsWith("{/*") ||
      trimmed.startsWith("/*") ||
      trimmed.includes("{/*")
    ) return;

    for (const rule of RULES) {
      if (rule.skipTokenFiles && isTokenFile) continue;
      if (rule.regex.test(trimmed)) {
        violations.push({ file: relPath, line: idx + 1, rule: rule.name, snippet: trimmed.slice(0, 120) });
      }
    }
    for (const retired of RETIRED) {
      if (trimmed.toLowerCase().includes(retired.toLowerCase())) {
        violations.push({ file: relPath, line: idx + 1, rule: `retired value ${retired}`, snippet: trimmed.slice(0, 120) });
      }
    }
  });
}

// Token sync with the ATP reference copy.
const token = readFileSync(join(APP_ROOT, TOKEN_PATH));
const actualHash = createHash("sha256").update(token).digest("hex");
const expectedHash = readFileSync(join(APP_ROOT, "scripts/linear-theme.sha256"), "utf8")
  .split(/\s+/)[0];
if (actualHash !== expectedHash) {
  violations.push({
    file: TOKEN_PATH,
    line: 0,
    rule: "token sync",
    snippet: `sha256 ${actualHash} does not match ATP reference ${expectedHash}`,
  });
}

// Contrast checks from the migration plan (WCAG ratios).
function parseTheme(css, selector) {
  const start = css.indexOf(`${selector} {`);
  if (start < 0) return {};
  const end = css.indexOf("\n}", start);
  const block = css.slice(start, end < 0 ? css.length : end);
  const vars = {};
  for (const match of block.matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/gi)) {
    vars[match[1]] = match[2].trim();
  }
  return vars;
}
function toRgb(value) {
  const hex = /^#([0-9a-f]{6})$/i.exec(value.trim());
  if (!hex) return null;
  return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16));
}
function luminance(rgb) {
  const [r, g, b] = rgb.map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a, b) {
  const [l1, l2] = [luminance(toRgb(a)), luminance(toRgb(b))].sort((x, y) => y - x);
  return (l1 + 0.05) / (l2 + 0.05);
}

const css = readFileSync(join(APP_ROOT, TOKEN_PATH), "utf8");
const themes = {
  light: parseTheme(css, ':root, [data-theme="light"]'),
  dark: parseTheme(css, '[data-theme="dark"]'),
};
const checks = [
  ["text", "bg-surface", 4.5],
  ["text-secondary", "bg-surface", 4.5],
  ["text-muted", "bg-surface", 4.5],
  ["text-muted", "bg-surface-2", 4.5],
  ["text-muted", "bg-active", 4.5],
  ["blue-text", "bg-surface", 4.5],
  ["green-text", "chip-green", 4.5],
  ["red-text", "chip-red", 4.5],
  ["btn-primary-text", "btn-primary-bg", 4.5],
];
for (const [themeName, vars] of Object.entries(themes)) {
  for (const [fg, bg, min] of checks) {
    if (!vars[fg] || !vars[bg]) {
      violations.push({ file: TOKEN_PATH, line: 0, rule: "contrast", snippet: `${themeName}: missing ${fg}/${bg}` });
      continue;
    }
    const ratio = contrast(vars[fg], vars[bg]);
    if (ratio < min) {
      violations.push({
        file: TOKEN_PATH,
        line: 0,
        rule: "contrast",
        snippet: `${themeName}: ${fg} on ${bg} = ${ratio.toFixed(2)} (< ${min})`,
      });
    }
  }
  // Graphics need 3:1 against the card, excluding the intentionally labelled gray.
  for (const series of ["blue", "sky", "green", "amber", "teal", "pink", "purple", "orange"]) {
    const ratio = contrast(vars[series], vars["bg-surface"]);
    if (ratio < 3) {
      violations.push({
        file: TOKEN_PATH,
        line: 0,
        rule: "contrast",
        snippet: `${themeName}: series ${series} on bg-surface = ${ratio.toFixed(2)} (< 3.0)`,
      });
    }
  }
}

if (violations.length > 0) {
  console.error(`❌ check:linear FAILED (${violations.length} violation(s)):`);
  for (const v of violations) {
    console.error(`  - ${v.file}:${v.line} [${v.rule}] -> ${v.snippet}`);
  }
  process.exit(1);
}
console.log("✅ check:linear PASSED: shared Linear tokens, no raw colors, no blur or extra shadows.");
