#!/usr/bin/env node
/**
 * Linear palette gate. Colors live only in src/styles/linear-theme.css and
 * src/styles/app-accent.css. No blur, translucency, glow, extra shadows,
 * retired palette values, foreign primary accents or green hues.
 */
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "src");
const TOKEN_PATH = "src/styles/linear-theme.css";
const ACCENT_PATH = "src/styles/app-accent.css";
const TOKEN_FILES = new Set([TOKEN_PATH, ACCENT_PATH]);

// Retired values (Plan §4). None may appear anywhere in source.
const RETIRED = [
  "#050505", "#059669", "#08080A", "#0891B2", "#09090B", "#0ACC8A", "#0D9488",
  "#0F1115", "#10B981", "#111213", "#121215", "#15803D", "#17171C", "#1A1B1E",
  "#1A9942", "#1B2531", "#1C261E", "#1F2125", "#1FB5A5", "#202026", "#202128",
  "#22C55E", "#23232A", "#2C2E33", "#33333D", "#3355FF", "#3483FF", "#38B6F0",
  "#38C84F", "#5A5B62", "#6B7280", "#71717A", "#7C3AED", "#7C5CE6", "#831843",
  "#8A8B91", "#8E8E93", "#9CA3AF", "#A1A1AA", "#A855F7", "#C8C8D0", "#DBEAFE",
  "#DCFCE7", "#E2E2E7", "#E8E8EC", "#EAB308", "#EAF0FF", "#EC4899", "#F2F2F4",
  "#F43F5E", "#F4F4F5", "#F4F5F7", "#F59E0B", "#F5F5F6", "#F7F7F8",
];

// Other apps' primary accents (Plan §7.8). Only Starship Control's Violet may appear.
const FOREIGN_PRIMARY = [
  "#06B6D4", "#0C667F", "#0E7490", "#193236", "#1F273C", "#2157CF", "#22D3EE",
  "#24BFD9", "#2E5DE0", "#331D36", "#351B22", "#3B281B", "#6B9BFF", "#8C1032",
  "#8F199A", "#A21CAF", "#A921BA", "#AB390B", "#B7153E", "#C026D3", "#C2410C",
  "#D01846", "#E879F9", "#EEF5F7", "#F0F4FE", "#F8EEF1", "#F8EFF9", "#F97316",
  "#FA8432", "#FB7185", "#FB923C", "#FBF2EE",
];

function findFiles(dir, exts, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) findFiles(full, exts, acc);
    else if (exts.some((ext) => entry.endsWith(ext))) acc.push(full);
  }
  return acc;
}

/** Returns the hue in degrees (0-360) and saturation (0-1) for a #rrggbb color. */
function hexToHueSat(hex) {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const r = parseInt(m[1].slice(0, 2), 16) / 255;
  const g = parseInt(m[1].slice(2, 4), 16) / 255;
  const b = parseInt(m[1].slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  const l = (max + min) / 2;
  if (delta === 0) return { h: 0, s: 0 };
  const s = delta / (1 - Math.abs(2 * l - 1));
  let h;
  if (max === r) h = 60 * (((g - b) / delta) % 6);
  else if (max === g) h = 60 * ((b - r) / delta + 2);
  else h = 60 * ((r - g) / delta + 4);
  if (h < 0) h += 360;
  return { h, s };
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
    for (const foreign of FOREIGN_PRIMARY) {
      if (trimmed.toLowerCase().includes(foreign.toLowerCase())) {
        violations.push({ file: relPath, line: idx + 1, rule: `foreign primary ${foreign}`, snippet: trimmed.slice(0, 120) });
      }
    }
    // Green hue (75-170 deg) at meaningful saturation is forbidden anywhere.
    for (const hex of trimmed.match(/#[0-9a-fA-F]{6}\b/g) ?? []) {
      const hs = hexToHueSat(hex);
      if (hs && hs.s >= 0.15 && hs.h >= 75 && hs.h <= 170) {
        violations.push({ file: relPath, line: idx + 1, rule: `green hue ${hex} (${hs.h.toFixed(0)}deg)`, snippet: trimmed.slice(0, 120) });
      }
    }
  });
}

// Token sync with the ATP reference copy (linear-theme.css) and the Plan §3 accent block.
function checkSync(tokenPath, shaPath, label) {
  const actualHash = createHash("sha256").update(readFileSync(join(APP_ROOT, tokenPath))).digest("hex");
  const expectedHash = readFileSync(join(APP_ROOT, shaPath), "utf8").split(/\s+/)[0];
  if (actualHash !== expectedHash) {
    violations.push({
      file: tokenPath,
      line: 0,
      rule: `${label} sync`,
      snippet: `sha256 ${actualHash} does not match reference ${expectedHash}`,
    });
  }
}
checkSync(TOKEN_PATH, "scripts/linear-theme.sha256", "token");
checkSync(ACCENT_PATH, "scripts/app-accent.sha256", "accent");

// Contrast checks from the migration plan (WCAG ratios), Plan §8.
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

const themeCss = readFileSync(join(APP_ROOT, TOKEN_PATH), "utf8");
const accentCss = readFileSync(join(APP_ROOT, ACCENT_PATH), "utf8");
const themes = {
  light: { ...parseTheme(themeCss, ':root, [data-theme="light"]'), ...parseTheme(accentCss, ':root, [data-theme="light"]') },
  dark: { ...parseTheme(themeCss, '[data-theme="dark"]'), ...parseTheme(accentCss, '[data-theme="dark"]') },
};
const checks = [
  ["text", "bg-surface", 4.5],
  ["text-secondary", "bg-surface", 4.5],
  ["text-muted", "bg-surface", 4.5],
  ["text-muted", "bg-surface-2", 4.5],
  ["text-muted", "bg-active", 4.5],
  ["text-faint", "bg-surface", 3.0],
  ["text-faint", "bg-surface-2", 3.0],
  ["sky-text", "bg-surface", 4.5],
  ["sky-text", "chip-sky", 4.5],
  ["red-text", "chip-red", 4.5],
  ["primary-fg", "primary", 4.5],
  ["primary-fg", "primary-hover", 4.5],
  ["primary-text", "bg-surface", 4.5],
  ["primary-text", "bg-surface-2", 4.5],
  ["primary-text", "primary-tint", 4.5],
];
// Graphics need 3:1 against the card (gray is intentionally labelled and excluded).
const graphicSeries = ["primary", "blue", "sky", "amber", "pink", "purple", "orange", "red"];
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
  for (const series of graphicSeries) {
    if (!vars[series] || !vars["bg-surface"]) continue;
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
console.log("✅ check:linear PASSED: shared Linear tokens, Violet accent, no raw colors, no green, no blur or extra shadows.");