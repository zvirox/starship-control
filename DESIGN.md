# DESIGN.md — Starship Control Design System

**Version:** 2.1 · **Date:** 2026-11-25 · **Status:** FINAL & LOCKED (No further UI style changes)
**Authority:** `PRD.md` (scope, functional requirements) → `ARCHITECTURE.md` (multi-window, crates, IPC) → **this file (UI & visual law)**  
**Rule:** Every interface built in Starship Control must follow this document strictly. Do not invent custom tokens, raw hex in components, blur, translucency, or dark-theme shadows outside this specification.

---

## 1. Visual Principles

- **Linear Solid Minimalism:** Flat, solid, opaque surfaces with 1px borders. Dark and light themes only (`data-theme="dark|light"`, storage key `ui-theme`, default follows OS preference, applied before first paint).
- **Strictly Forbidden:** `backdrop-filter`, blur, frosted glass, translucent surface fills (`rgba()`, `bg-white/10`), vibrancy/mica/acrylic, glows, inset highlights, shadows in dark mode, gradients on surfaces. A gradient fade is permitted **only** under chart lines and sparklines.
- **Color Discipline:** Color lives in charts and small elements (precomputed solid icon chips, delta pills, progress bars, status dots, tags), never in large surface fills.
- **Depth:** Zero shadows in dark mode. Light cards use 1px border plus at most `0 1px 2px rgb(16 24 40 / 5%)`.
- **Motion:** `150ms ease-out`, zero blur transitions, reduced-motion compliant.

---

## 2. Color Tokens (`app/src/tailwind.css` & `echarts-linear-theme.ts`)

| Token | Dark (`[data-theme="dark"]`) | Light (`[data-theme="light"]`) | Semantic Role |
|---|---|---|---|
| `--bg-app` | `#050505` | `#F3F4F6` | Window base and sidebar |
| `--bg-surface` | `#111213` | `#FFFFFF` | Primary cards, panels, HUD surface |
| `--bg-surface-2` | `#1A1B1E` | `#F4F5F7` | Active nav, inputs, chips, nested cards |
| `--bg-bar-idle` | `#202128` | `#EAF0FF` | Idle chart bars and progress tracks |
| `--border` | `#1F2125` | `#E5E7EB` | 1px card boundaries, dividers, table hairlines |
| `--border-strong` | `#2C2E33` | `#D1D5DB` | Hover card borders, active inputs, dialog borders |
| `--text` | `#F5F5F6` | `#0F1115` | Primary headings and metric values |
| `--text-muted` | `#8A8B91` | `#6B7280` | Secondary labels, inactive nav, axis labels |
| `--text-faint` | `#5A5B62` | `#9CA3AF` | Placeholders, disabled captions |

### Shared Accent Palette (Both Themes)

| Accent | Hex | Semantic Role |
|---|---|---|
| Blue | `#3355FF` | Primary button, main chart series, focus ring |
| Sky | `#38B6F0` | Second series, active stack bar |
| Green | `#22C55E` | Positive delta, healthy status, sparkline up |
| Red | `#F43F5E` | Negative delta, error, approval gate, sparkline down |
| Amber | `#EAB308` | Warning, third series |
| Teal | `#1FB5A5` | Fourth series |
| Pink | `#EC4899` | Fifth series |
| Purple | `#7C5CE6` | Sixth series, local GGUF inference |
| Orange | `#F97316` | Seventh series |
| Close Hover | `#E5484D` | Window close button hover background |

Icon chips and delta pills use solid precomputed theme tints (`--tint-blue`, `--tint-sky`, `--tint-green`, `--tint-red`, `--tint-amber`, `--tint-teal`, `--tint-pink`, `--tint-purple`, `--tint-orange`), never translucent overlays.

---

## 3. Typography & Spacing

- **UI Typeface:** Self-hosted `Inter` (`/fonts/InterVariable.woff2`). Sizes: `12 / 13 / 14 / 16 / 20 / 28 px`.
- **Stat Numbers:** `28 px` semibold with tabular figures (`tabular-nums`); units at `12 px` muted.
- **Table Headers:** `12 px` uppercase with subtle tracking.
- **Monospace:** Self-hosted `JetBrains Mono`, used only in log, schema, and code views.
- **Spacing Grid:** `4 px` grid. Card padding `20 px`, grid gap `16 px`. Collapsible sidebar `240 px` (collapsed `56 px`), nav items `36 px` tall.
- **Radii:** Cards `8 px`, buttons/inputs `6 px`, chips/tags/tooltips `4 px`. No pill shapes. Round (`999px` / `rounded-full`) applies **only** to status dots, avatars, and donut arc ends.

---

## 4. Window & Layout Specifications

### 4.1 Cockpit Window (`1280×800`, min `1024×640`)
- **Window Config:** 16:10 wide rectangle, `decorations: false`, `transparent: false`, `resizable: true`, `shadow: true`, square window corners.
- **Title Bar (`40 px`):**
  - Left: `20 px` logo, `"Starship Control"` name, `11 px` (`4 px` radius, `1 px` border) version tag read from `tauri.conf.json`.
  - Center: empty `data-tauri-drag-region`; double-click toggles maximize.
  - Right: `ThemeToggle` (sun/moon), thin vertical divider, minimize, maximize/restore, close (`#E5484D` hover). Toggle exists only in the title bar.
- **Shell:** Collapsible `240 px` → `56 px` `Sidebar` with workspace switcher + `Breadcrumb` header bar + `Command Center` dashboard (`StatCard`s, ECharts `Hub throughput` line chart with `RangeTabs`, `Inference latency` bar chart, `Model states` donut + `DonutLegend`, `ProgressRow` resource list, `DataTable` task & approval queue) and subviews (`Chat`, `Timeline`, `Missions`, `Workflows`, `Connected Apps`, `Tools`, `Models`, `Systems`, `Activity`).

### 4.2 Floating HUD Window (`420×64`)
- **Window Config:** `420×64` rectangular solid bar (`decorations: false`, `transparent: false`, `shadow: true`, `alwaysOnTop: true`), `6 px` inner surface radius on `--bg-surface` with `1 px` `--border`. Zero blur, zero dark-theme shadow.
- **Controls:** Compact drag handle (`data-tauri-drag-region`), `20 px` logo, and close button (`#E5484D` hover) only. No theme toggle — synchronizes with Cockpit via Tauri event (`evt://theme/changed`).
- **Content:** `StatusPill`, live tokens per second metric (`tabular-nums`), and green/red sparkline with gradient fade under line.
