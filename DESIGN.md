# DESIGN.md — Starship Control Design System

**Version:** 2.2 · **Date:** 2026-11-26 · **Status:** FINAL & LOCKED (No further UI style changes)
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

## 2. Color Tokens (`app/src/styles/linear-theme.css`)

The shared token file (`linear-theme.css`) is byte-identical across the companion apps; the per-app primary accent lives in `app-accent.css` (Starship Control = **Violet**). Both are mapped into Tailwind by `@theme inline` in `app/src/tailwind.css`. No other source file may contain a hex, `rgb()`, `hsl()`, Tailwind palette class or arbitrary color value. Charts read the same variables at runtime through `echarts-linear-theme.ts`.

| Token | Dark | Light | Semantic Role |
|---|---|---|---|
| `--bg-app` | `#1C1C1C` | `#F8F9FA` | Window, title bar, breadcrumb bar, main area |
| `--bg-sidebar` | `#1C1C1C` | `#F3F4F6` | Sidebar and left rail |
| `--bg-surface` | `#1C1C1C` | `#FFFFFF` | Cards, panels, dialogs, HUD bar |
| `--bg-surface-2` | `#292929` | `#F3F4F6` | Inputs, tags, chips, nested boxes |
| `--bg-active` | `#292929` | `#E5E7EB` | Active nav item, selected range tab |
| `--bg-hover` | `#232323` | `#EEF0F2` | Hovered nav and title bar buttons |
| `--bg-track` / `--bg-bar-idle` | `#333333` | `#E5E7EB` | Switch off, progress track, idle chart bars |
| `--border` | `#2E2E2E` | `#E5E7EB` | 1px hairlines and dividers |
| `--border-strong` | `#363636` | `#D1D5DB` | Card hover, dialog and tooltip borders |
| `--chart-grid` | `#2E2E2E` | `#D1D5DB` | Dotted chart gridlines |
| `--text` | `#FFFFFF` | `#111827` | Headings, numbers, active tab |
| `--text-secondary` | `#CBCBCB` | `#111827` | Body text, labels, table rows |
| `--text-muted` | `#B4B4B4` | `#5B6370` | Units, timestamps, table headers |
| `--text-faint` | `#939393` | `#818998` | Hints, expand icons, idle dots |

### Per-App Primary Accent (`app-accent.css` — Violet for Starship Control)

| Token | Dark | Light | Semantic Role |
|---|---|---|---|
| `--primary` | `#8B3DFF` | `#6D28D9` | Primary button fill, switch on, slider/fader, in-progress fill, selected border |
| `--primary-hover` | `#7A36E0` | `#6023BF` | Primary button hover |
| `--primary-fg` | `#FFFFFF` | `#FFFFFF` | Primary button text |
| `--primary-text` | `#B28CFF` | `#6D28D9` | Links, focus ring, selected tab text/underline, active nav icon, Assigned chip text |
| `--primary-tint` | `#2C213C` | `#F5F0FC` | Assigned / info chip fill |

### Shared Series Palette (Both Themes)

| Token | Dark | Light | Semantic Role |
|---|---|---|---|
| `--blue` | `#346AFE` | `#2563EB` | Series and clip color |
| `--sky` / `--sky-text` | `#2FADFF` / `#2FADFF` | `#0284C7` / `#0369A1` | Series 2, waveform / links, focus ring, active dot, success & positive delta (replaces green) |
| `--amber` | `#F6940F` | `#D97706` | Series 3, warning |
| `--pink` | `#FC669B` | `#DB2777` | Series 4 |
| `--purple` | `#6D59F9` | `#6D28D9` | Series 5, local GGUF |
| `--orange` | `#FB6801` | `#EA580C` | Series 6 |
| `--red` / `--red-text` | `#FC4232` / `#FF6B5E` | `#DC2626` / `#B91C1C` | Error, critical / negative delta |
| `--gray` | `#4F4F4F` | `#868E9C` | Neutral labelled bars only |

Chart series order for Starship Control is `--primary, --sky, --amber, --pink, --blue`; colors within 25 color-distance of the primary (e.g. purple) are skipped. Chip fills are solid (`--chip-sky`, `--chip-red`). Primary buttons use `--primary` fill with `--primary-fg` label in both themes. The only allowed shadow is `--shadow-card`; dark mode keeps it by explicit decision. Scrim and chart fades are the only translucent definitions. Green and teal are removed entirely — positive, healthy, online, done and up use `--sky`.

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
- **Content:** `StatusPill`, live tokens per second metric (`tabular-nums`), and sky/red sparkline with gradient fade under line.
