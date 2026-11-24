# DESIGN.md — Starship Control Design System

**Version:** 2.0 · **Date:** 2026-11-24  
**Authority:** `PRD.md` (scope, functional requirements) → `ARCHITECTURE.md` (multi-window, crates, IPC) → **this file (UI & visual law)**  
**Rule:** Every interface built in Starship Control must follow this document strictly. Do not invent custom tokens, colors, or fonts outside it. To change a token or recipe, update this specification first.

---

## 1. Brand Feel & Visual Principles

**Feel:** Mission control engineered with Linear-Style screenshot precision. Pure solid matte surfaces, crisp 1px hairlines, flat border-only card boundaries, and zero-glass blur. Designed natively for modern Linux desktop environments with dark elevated surface minimalism and high-contrast metrics.

### Principle 1 — Dual-Surface Identity
- **The Cockpit (Strategic Canvas):** An immersive 1200×800 workspace for mission planning, deep multi-step workflow graphs, model downloading, and audit inspection. Solid elevated surface cards (`#111213` / `#1A1B1E`) dominate reading zones over the dark base canvas (`#050505`).
- **The Floating HUD (Ambient Companion):** A lightweight, always-on-top solid matte capsule (520×76 resting) that hovers seamlessly over active apps like EEP or ALP. It expands dynamically (520×240) only when presenting live agent reasoning or requiring human approval.

### Principle 2 — Pure Solid Surfaces (Zero Glass Blur, Flat in Dark Mode)
Do not use `backdrop-filter: blur()`. Panels are 100% opaque matte graphite/charcoal (Dark: `#111213`) or crisp alabaster/paper (Light: `#FFFFFF`).
- Dark mode surfaces are flat, border-only: no shadow, no glow, no blur.
- Light mode cards use 1px border plus at most `0 1px 2px rgb(16 24 40 / 5%)`.
- Structural boundaries use a crisp 1px border (`#1F2125` default, `#2C2E33` strong). Never use thick borders or specular gradients.

### Principle 3 — Screenshot Color Economy
The UI is built on quiet obsidian neutrals with **Linear Blue (`#3355FF`)** as the primary focus accent (meaning: *action available / active agent execution*). Color is strictly controlled:
- **Primary Accent (`#3355FF`)**: Primary actions, Send button, active model selection.
- **Sky (`#38B6F0`)**: Secondary series, checkout/queue metrics.
- **Green (`#22C55E`)**: Positive delta, step verified, sub-app online.
- **Red (`#F43F5E`)**: Negative delta, human approval gate (Tier 2/3), tool error.
- **Amber (`#EAB308`)**: Agent thinking / delivery processing.
- **Teal (`#1FB5A5`)**: Delivery done / verified cache.
- **Pink (`#EC4899`)**: Returned / reverted action.
- **Purple (`#7C5CE6`)**: Local GGUF model execution.
- **Orange (`#F97316`)**: Progress tracking / warning telemetry.

---

## 2. Color Tokens

### 2.1 Dark Mode (Default & Primary — Screenshot Palette)

| Token | Hex / Value | Semantic Role |
|---|---|---|
| `--bg-app` | `#050505` | Window, sidebar, and canvas base |
| `--bg-surface` | `#111213` | Primary cards, panels, and modal canvas |
| `--bg-surface-2` | `#1A1B1E` | Active nav, secondary cards, inputs, chips |
| `--bg-bar-idle` | `#202128` | Idle/unselected chart bars and progress tracks |
| `--border` | `#1F2125` | Hairline card boundaries, dividers, table borders |
| `--border-strong` | `#2C2E33` | Hovered card boundaries, active inputs, modal borders |
| `--text` | `#F5F5F6` | High-contrast headings, primary text |
| `--text-muted` | `#8A8B91` | Secondary descriptions, timestamps, inactive icons |
| `--text-faint` | `#5A5B62` | Placeholders, disabled states, faint captions |
| `accent.primary` | `#3355FF` | Linear Blue primary action buttons, focused controls |
| `accent.sky` | `#38B6F0` | Secondary telemetry metric series |
| `accent.green` | `#22C55E` | Positive delta, verified status |
| `accent.red` | `#F43F5E` | Approval required, danger, negative delta |

### 2.2 Light Mode (Screenshot Palette)

| Token | Hex / Value | Semantic Role |
|---|---|---|
| `--bg-app` | `#F3F4F6` | Window background, canvas |
| `--bg-surface` | `#FFFFFF` | Solid white cards, panels |
| `--bg-surface-2` | `#F4F5F7` | Active nav, inputs, chips |
| `--bg-bar-idle` | `#EAF0FF` | Light idle bars |
| `--border` | `#E5E7EB` | Hairline borders |
| `--border-strong` | `#D1D5DB` | Strong boundaries |
| `--text` | `#0F1115` | High-contrast dark text |
| `--text-muted` | `#6B7280` | Muted secondary labels |
| `--text-faint` | `#9CA3AF` | Faint text, disabled |
| `signal.gate` | `#F43F5E` | Human Approval Gate required (Tier 2/3 action) |
| `signal.gate-bg` | `rgba(244, 63, 94, 0.16)` | Approval card highlight & pulsating rim |
| `signal.local-model` | `#A855F7` | Indicator that local GGUF engine is running |

### 2.2 Light Mode (Clean Graphite Light)

| Token | Hex / Value | Semantic Role |
|---|---|---|
| `bg.canvas` | `#F4F4F6` | Soft clean light desktop canvas |
| `bg.raised` | `#FFFFFF` | Pure white top bar and sidebars |
| `surface.glass` | `rgba(255, 255, 255, 0.88)` | Translucent light panels |
| `surface.glass-strong`| `rgba(255, 255, 255, 0.98)` | High-density light glass |
| `surface.solid` | `#FFFFFF` | Solid cards, inputs, dropdowns |
| `border.subtle` | `#E4E4E7` | Zinc-200 dividers and card borders |
| `border.glass` | `#D4D4D8` | Specular inner border |
| `text.primary` | `#09090B` | Zinc-950 high contrast text (17.5:1 contrast) |
| `text.secondary` | `#3F3F46` | Zinc-700 clear secondary text (9.2:1 contrast) |
| `text.disabled` | `#71717A` | Zinc-500 crisp meta labels (4.8:1 contrast, WCAG AA compliant) |
| `accent` | `#0284C7` | High-contrast sky blue for primary controls |
| `accent.text` | `#FFFFFF` | Pure white text on accent |
| `signal.thinking` | `#D97706` | Amber signal |
| `signal.success` | `#059669` | Emerald signal |
| `signal.gate` | `#E11D48` | Rose approval signal |

### 2.3 PC Health & Telemetry Rich Color Palette

| Metric | Accent Color | Hex | Visual Representation |
|---|---|---|---|
| **CPU Load** | Sky Blue | `#38BDF8` | Telemetry SVG area gradient, progress bar, header trace |
| **RAM Used** | Violet / Purple | `#A855F7` | Telemetry SVG memory gradient, progress bar, footprint |
| **GPU Compute** | Cyan / Teal | `#2DD4BF` | Compute utilization badge and track |
| **VRAM Used** | Amber | `#FBBF24` | Memory allocation track |
| **Disk Free** | Emerald | `#34D399` | Free storage badge and capacity meter |
| **Thermal Core**| Rose | `#FB7185` | Temperature margin gauge |

---

## 3. Typography Specification

To maintain complete visual uniformity across the Linux software suite, Starship Control adopts the exact font pairings established in ScreenStudio Pro and Easy Edits Pro:

| Face | Format & Delivery | Semantic Usage |
|---|---|---|
| **Familjen Grotesk** | Variable `.woff2` (400–700), self-hosted | Headings, UI labels, buttons, mission prompts, agent explanations. |
| **JetBrains Mono** | Variable `.woff2` (400–600), self-hosted | Token metrics, latency timers, socket paths, model IDs, parameters, diffs. |

### 3.1 Type Scale

| Token | Size | Line Height | Weight | Font Family | Typical Usage |
|---|---|---|---|---|---|
| `display.lg` | 1.75rem (28px) | 1.20 | 500 | Familjen Grotesk | Cockpit hero titles ("Starship Control") |
| `display.md` | 1.375rem (22px)| 1.25 | 500 | Familjen Grotesk | View headers ("Model Control", "Topology") |
| `heading` | 1.125rem (18px)| 1.35 | 500 | Familjen Grotesk | Card headers, mission session titles |
| `body-md` | 0.9375rem (15px)| 1.50 | 400 | Familjen Grotesk | Chat body, agent explanations, descriptions |
| `body-sm` | 0.8125rem (13px)| 1.45 | 400 | Familjen Grotesk | Meta labels, secondary status notes |
| `label` | 0.75rem (12px) | 1.30 | 500 | Familjen Grotesk | Sentence-case meta labels and chips (`Tier 2`, `GGUF`) |
| `mono-md` | 0.9375rem (15px)| 1.30 | 500 | JetBrains Mono | Memory gauges, token metrics, timers |
| `mono-sm` | 0.8125rem (13px)| 1.35 | 400 | JetBrains Mono | Socket paths, JSON diff values, hashes |

*Type law:* Hierarchy comes from size and color, not weight. Weights are 400 and 500; 600 is reserved for rare emphasis. No 700–900 headings, no gradient text, no negative tracking below 16px, no italic emphasis, and no uppercase text with wide tracking. All labels, badges and table headers use sentence case. The smallest text in any dense panel is 11px.

---

## 4. Spacing, Radii & Glass Recipes

### 4.1 Spacing Scale (4px Base Grid)
`space.1 = 4px · space.2 = 8px · space.3 = 12px · space.4 = 16px · space.5 = 20px · space.6 = 24px · space.8 = 32px · space.10 = 40px`

### 4.2 Corner Radii
| Token | Value | Applied Elements |
|---|---|---|
| `r.xs` | 6px | Inline badges, status chips, code snippets |
| `r.sm` | 10px | Buttons, inputs, dropdown selectors |
| `r.md` | 14px | Task Sheet step cards, tool output boxes |
| `r.lg` | 20px | Floating HUD capsule, dialog modals, Cockpit panels |
| `r.full` | 999px | The HUD capsule and genuinely circular status dots only |

*Radius law:* Controls use `r.sm`, cards `r.md`, panels `r.lg`. Nested radius equals the outer radius minus the padding between them, never below 4px. Status chips, counts and badges use `r.xs` or `r.sm` with quiet text, not full pills.

### 4.3 Linear Elevated Surface Recipes

| Recipe | Base Fill | Backdrop Filter | Border | Shadow Elevation | Applied To |
|---|---|---|---|---|---|
| **Surface 1 (Base Panel)** | `#121215` (Dark) / `#ffffff` (Light) | None (0 blur) | 1px `#23232a` (Dark) / `#e2e2e7` (Light) | `shadow-linear-dark` / `shadow-linear-light` | Cockpit primary panels, outer shell, main cards |
| **Surface 2 (Nested / Input)** | `#17171c` (Dark) / `#f2f2f4` (Light) | None (0 blur) | 1px `#23232a` (Dark) / `#e2e2e7` (Light) | Micro 1px inset highlight | Inputs, table headers, nested step blocks |
| **Surface 3 (Hover / Active)** | `#202026` (Dark) / `#e8e8ec` (Light) | None (0 blur) | 1px `#33333d` (Dark) / `#c8c8d0` (Light) | Micro edge definition | Interactive hover, selected list items |
| **Float (Capsule & HUD)** | `#121215` (Dark) / `#ffffff` (Light) | None (0 blur) | 1px `#33333d` (Dark) / `#c8c8d0` (Light) | `shadow-linear-dark` (deep ambient drop) | Floating HUD capsule, approval action overlays |

*Surface law:* 100% solid matte surfaces with 1px hairlines and dual-shadow elevation. No `backdrop-filter: blur()`, no translucent glass tints, and no specular gradient overlays. Inset top highlight catching light on beveled edges provides tactile depth.

---

## 5. Multi-Window Layout Specifications

### 5.1 The Cockpit Window Layout (1200×800)

```
┌────────────────────────────────────────────────────────────────────────┐
│ [★ Starship Control]   [Active: Qwen2.5-Coder-7B (Local)]  [-][□][×]   │
├────────────────────────────────────────────────────────────────────────┤
│ (≡) Missions  ·  (⊞) Topology  ·  (⚡) Model Control  ·  (☵) Audit Log  │
├───────────────┬────────────────────────────────────────┬───────────────┤
│ MISSIONS (280)│ MISSION EXECUTION CANVAS (Center)      │ TOPOLOGY (300)│
│               │                                        │               │
│ + New Mission │ [User Prompt]                          │ CONNECTED     │
│               │ "Process screencast with EEP and ALP"  │ ● eep (Live)  │
│ Active:       │                                        │ ● ssp (Live)  │
│ • Tutorial Ep4│ ┌────────────────────────────────────┐ │ ● alp (Ready) │
│ • Clean cache │ │ ⚡ Starship Agent (Thinking 1.2s)    │ │               │
│ • Net audit   │ │ Decomposed into 3 tasks...         │ │ OFFLINE       │
│               │ ├────────────────────────────────────┤ │ ○ atp (CLI)   │
│ Past (24h):   │ │ [✓] 1. Read CET sidecar from SSP   │ │ ○ nwp (eBPF)  │
│ • Voice dub   │ │ [▶] 2. Clean vocal track in ALP    │ ├───────────────┤
│ • Render cuts │ │ [!] 3. Apply 14 cuts in EEP (Gate) │ │ RESOURCE GAUGE│
│               │ │     [Reject]  [Approve & Apply]    │ │ VRAM: 5.4/12GB│
│               │ └────────────────────────────────────┘ │ CPU: 12%      │
│               ├────────────────────────────────────────┤ Tokens: 1,420 │
│               │ [ Enter instruction or task sheet... ] │ Cost: $0.00   │
└───────────────┴────────────────────────────────────────┴───────────────┘
```

1. **Custom Titlebar (52px):** Contains Starship brand glyph, active mission name, model selection indicator (with Local GGUF / Cloud pill), and Wayland-compatible window actions.
2. **Navigation Strip (44px):** Instant switching between **Missions**, **App Topology**, **Model Control**, and **Audit Log**.
3. **Left Mission Drawer (280px):** Searchable list of past and active missions with state chips (`Idle`, `Running`, `Needs Review`).
4. **Center Canvas (Flexible):** Streaming agent conversation, markdown rendering, tool call execution boxes, and interactive approval cards.
5. **Right Topology Drawer (300px):** Real-time connection matrix showing connected sub-apps (`eep`, `ssp`, `alp`, `nwp`, `atp`) with latency ping, socket status, and local VRAM/RAM gauges.

---

### 5.2 The Floating Companion HUD Layout

The Floating HUD is designed to stay pinned above full-screen editors like EEP:

#### A. Resting State (520×76 px)
```
┌────────────────────────────────────────────────────────────────────────┐
│ [★] [Local: Qwen2.5-7B ▾]  Ask Starship or give instruction...   [🎤][▶]│
└────────────────────────────────────────────────────────────────────────┘
```
- **Starship Emblem:** Pulsates subtly during background tasks (`signal.thinking`).
- **Model Switcher Chip:** Quick dropdown to switch between downloaded GGUF models or active cloud endpoints.
- **Input Pill:** Glass-inset text field with auto-resizing height and placeholder tips.
- **Action Icons:** Microphone toggle (voice transcription via ALP engine) and Submit button (`accent`).

#### B. Expanded Approval State (520×240 px)
Triggered automatically when the agent proposes a Tier 2 (Mutating) or Tier 3 (Destructive) operation:

```
┌────────────────────────────────────────────────────────────────────────┐
│ [!] HUMAN APPROVAL REQUIRED — Easy Edits Pro                  [Tier 2] │
├────────────────────────────────────────────────────────────────────────┤
│ Action: Apply 18 ripple cuts and remove 01:24 dead time via CET.       │
│ Project: ~/Videos/Tutorial-Ep4.eeproj (Checkpoint created: #bk-04)     │
│ Impact: 18 timeline edits · 0 audio clips deleted                      │
├────────────────────────────────────────────────────────────────────────┤
│  [View Diff Details ▾]                 [Reject / Modify]   [✓ Approve] │
└────────────────────────────────────────────────────────────────────────┘
```
- **Pulsating Rose Border:** Clear visual distinction that execution is paused awaiting human consent.
- **Impact Summary:** Explicitly states files touched, checkpoints created, and reversibility.
- **Approval Actions:** Keyboard-accessible (`Enter` to Approve, `Escape` to Reject).

---

## 6. Component Library Specifications

### 6.1 Action Approval Card
- **Resting:** `surface.solid`, border 1px `border.glass`, shadow `elev.float`.
- **Active Gate State:** Border switches to `signal.gate` (`#F43F5E`) with an ambient outer glow (`rgba(244, 63, 94, 0.25)`).
- **Buttons:**
  - **Approve Button:** Primary accent (`accent` fill, `accent.text` label, `Enter` shortcut).
  - **Reject Button:** Secondary ghost (`border.subtle`, hover `error.muted`, `Esc` shortcut).

### 6.2 Model Manager Card (Hugging Face Downloader)
- Shows model name, parameter size, quantization tier (`Q4_K_M`, `Q8_0`), download progress bar with speed (`MB/s`) and ETA, and SHA-256 verification badge.
- Hardware Compatibility Gauge: Inspects system VRAM and tags models as:
  - `● Fits in VRAM (Fastest)`
  - `◐ Partial Offload (Moderate)`
  - `○ CPU Only (Slow)`

### 6.3 App Topology Tile
- Displays sub-app icon, display name, connection mode badge:
  - `[● Live GUI]` (Connected via Unix Domain Socket)
  - `[◌ Headless CLI]` (Available via stdio MCP)
  - `[✕ Offline]` (Not running; provides a "Wake Up" click action)

---

## 7. Motion & Animation Tokens

Implemented with `framer-motion` physics-based springs:

| Token | Parameters | Applied To |
|---|---|---|
| `motion.fast` | `duration: 0.15s, ease: "easeOut"` | Button hover fills, input focus rings |
| `motion.spring`| `type: "spring", stiffness: 320, damping: 26` | HUD expansion from 76px to 240px, modal popups |
| `motion.slide` | `type: "spring", stiffness: 280, damping: 24` | Approval card slide-in, toast appearance |
| `motion.pulse` | `duration: 1.6s, repeat: Infinity, ease: "easeInOut"` | Thinking emblem breath (opacity `0.45` to `1.0`) |

*Motion law:* One easing curve (`cubic-bezier(0.22, 1, 0.36, 1)`) and a few short durations. Loop-free feedback only: hover and press change color, surface, border or opacity; menus fade and rise a few pixels. Never animate width, height, margins or large blurs. `animate-pulse` and `animate-ping` are reserved for the active agent stream and the critical-health state; no `transition-all`, bouncy springs or `hover:scale` on cards.

*Accessibility:* When `@media (prefers-reduced-motion: reduce)` is active, all layout springs snap instantly (`duration: 0s`), and opacity transitions are limited to `≤100ms`.
