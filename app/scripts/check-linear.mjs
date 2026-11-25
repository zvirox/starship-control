#!/usr/bin/env node
/**
 * Linear-Style Design System Gate (Zero Frosted Glass, Zero Raw Hex, Zero Dark Shadows).
 * Enforces Plan Section 1 & Section 5 verification rules across app/src.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "src");

const TOKEN_FILES = new Set([
  "src/tailwind.css",
  "src/components/linear/echarts-linear-theme.ts",
]);

function findFiles(dir, exts, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      findFiles(full, exts, acc);
    } else if (exts.some((ext) => entry.endsWith(ext))) {
      acc.push(full);
    }
  }
  return acc;
}

const RULES = [
  {
    name: "blur / backdrop-filter",
    regex: /\bbackdrop-blur\b|(?:-webkit-)?backdrop-filter\s*:/i,
    skipTokenFiles: false,
  },
  {
    name: "translucency (rgba/hsla or translucent bg/surface utilities)",
    regex:
      /\b(?:rgba|hsla)\s*\(|\bbg-(?:white|black)\/|\b(?:bg|border|surface)-[a-z0-9-]+\/(?:\d+|\[)/i,
    skipTokenFiles: false,
  },
  {
    name: "glow / drop-shadow",
    regex: /\bdrop-shadow\b|glass-pane|glass-float|glass-modal/i,
    skipTokenFiles: false,
  },
  {
    name: "shadows in components (dark mode must have zero shadows)",
    regex:
      /\bshadow-(?:2xs|xs|sm|md|lg|xl|2xl|inner)\b|\bshadow-\[/i,
    skipTokenFiles: true,
  },
  {
    name: "raw hex color outside token files",
    regex:
      /(?:bg|text|border|ring|fill|stroke|from|to|via|accent|outline)-\[#[0-9a-fA-F]{3,8}\]|(?:color|background|backgroundColor|borderColor|fill|stroke|stopColor)\s*:\s*['"]#[0-9a-fA-F]{3,8}['"]|['"]#[0-9a-fA-F]{6,8}['"]/,
    skipTokenFiles: true,
  },
];

const files = findFiles(UI_SRC, [".tsx", ".ts", ".css"]);
const violations = [];

for (const file of files) {
  const relPath = relative(APP_ROOT, file).replace(/\\/g, "/");
  const isTokenFile = TOKEN_FILES.has(relPath);
  const lines = readFileSync(file, "utf8").split(/\r?\n/);

  lines.forEach((line, idx) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("//") || trimmed.startsWith("*")) return;

    for (const rule of RULES) {
      if (rule.skipTokenFiles && isTokenFile) continue;
      const match = line.match(rule.regex);
      if (match) {
        violations.push({
          file: relPath,
          line: idx + 1,
          rule: rule.name,
          snippet: trimmed.slice(0, 120),
        });
      }
    }
  });
}

if (violations.length > 0) {
  console.error(
    `❌ check:linear gate FAILED (${violations.length} violation(s)):`
  );
  for (const v of violations) {
    console.error(`  - ${v.file}:${v.line} [${v.rule}] -> ${v.snippet}`);
  }
  process.exit(1);
} else {
  console.log(
    "✅ check:linear gate PASSED: Zero blur, translucency, glow, dark shadows, or off-token hex colors."
  );
  process.exit(0);
}
