# DESIGN.md — Starship Control Design System

**Version:** 1.0 · **Date:** 2026-11-17  
**Authority:** `PRD.md` (scope, functional requirements) → `ARCHITECTURE.md` (multi-window, crates, IPC) → **this file (UI & visual law)**  
**Rule:** Every interface built in Starship Control must follow this document strictly. Do not invent custom tokens, colors, or fonts outside it. To change a token or recipe, update this specification first.

---

## 1. Brand Feel & Visual Principles

**Feel:** Mission control on glass. Translucent, calm, and ambient while resting; laser-focused, precise, and authoritative when an agent plans and executes. Designed natively for modern Linux rices (KDE Plasma 6, Hyprland, niri, Sway, MangoWC) using frosted-glass compositing and high-contrast typography.

### Principle 1 — Dual-Surface Identity
- **The Cockpit (Strategic Canvas):** An immersive 1200×800 workspace for mission planning, deep multi-step workflow graphs, model downloading, and audit inspection. Solid surfaces dominate reading zones; glass frames the outer shell.
- **The Floating HUD (Ambient Companion):** A lightweight, always-on-top glass capsule (520×76 resting) that hovers seamlessly over active apps like EEP or ALP. It expands dynamically (520×240) only when presenting live agent reasoning or requiring human approval.

### Principle 2 — Glass with a Contrast Floor
Translucency must never compromise readability. Every frosted-glass layer pairs tinted surface fill with hardware backdrop blur (`blur(24px)` to `blur(32px)`), a 1px specular inner border, and a dark scrim floor (`bg.canvas` at 55% opacity). If a compositor lacks blur protocols (e.g. pure X11 without Compton/Picom), surfaces gracefully step down to opaque solid tokens without layout shift.

### Principle 3 — Signal-Driven Color Economy
The UI is built on quiet obsidian neutrals with **Electric Cyan (`#38BDF8`)** as the primary focus accent (meaning: *action available / active agent execution*). Color is never used as arbitrary decoration:
- **Amber (`#F59E0B`)**: Agent Thinking / Planning in progress.
- **Emerald (`#10B981`)**: Step Verified / Sub-app Connected.
- **Rose (`#F43F5E`)**: Human Approval Gate Required (Tier 2/3) / Tool Error.
- **Violet (`#A855F7`)**: Local Hugging Face GGUF Execution.

### Principle 4 — Motion is Operational Feedback
Every motion communicates state. Window expansions, card slide-ins, and approval reveals use spring physics (`framer-motion`, 180–240ms). No looping animations outside the active agent reasoning pulse. Fully suppressed when `prefers-reduced-motion` is detected.

---

## 2. Color Tokens

### 2.1 Dark Mode (Default & Primary — Graphite)

| Token | Hex / Value | Semantic Role |
|---|---|---|
| `bg.canvas` | `#121214` | Neutral Graphite black base canvas beneath all glass surfaces |
| `bg.raised` | `#18181b` | Opaque graphite layer for header, sidebars, settings cards, audit tables |
| `surface.glass` | `rgba(255, 255, 255, 0.05)` | Standard glass fill (Cockpit sidebars & panels) |
| `surface.glass-strong`| `rgba(24, 24, 27, 0.90)` | High-density glass (Floating HUD, Modals, Toasts) |
| `surface.solid` | `#202024` | Resting cards, task sheet step blocks, dialogs, inputs |
| `border.subtle` | `rgba(255, 255, 255, 0.12)` | Dividers, separators, inactive card edges |
| `border.glass` | `rgba(255, 255, 255, 0.18)` | 1px inner specular boundary on glass layers |
| `border.focus` | `#38BDF8` | Universal keyboard focus indicator |
| `text.primary` | `#FAFAFA` | High-contrast headings, primary user prompts, active labels (16.5:1 contrast) |
| `text.secondary` | `#D4D4D8` | Assistant thoughts, step explanations, metadata, readable labels (11.8:1 contrast) |
| `text.disabled` | `#A1A1AA` | Crisp placeholders, timestamps, subtle captions (6.5:1 contrast, WCAG AA compliant) |
| `accent` | `#38BDF8` | Primary actions, Send button, active model selection |
| `accent.hover` | `#7DD3FC` | Interactive hover state for primary accent |
| `accent.active` | `#0284C7` | Pressed state for primary controls |
| `accent.muted` | `rgba(56, 189, 248, 0.16)` | Selected table rows, active tab indicators |
| `accent.text` | `#031726` | High-contrast text rendered over solid `accent` |
| `signal.thinking` | `#F59E0B` | Agent actively planning / LLM streaming tokens |
| `signal.thinking-bg` | `rgba(245, 158, 11, 0.15)`| Ambient wash during model generation |
| `signal.success` | `#10B981` | Tool verified, sub-app online, mission completed |
| `signal.success-bg` | `rgba(16, 185, 129, 0.15)`| Success badge background |
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

### 4.3 Glass Surface Recipes

| Recipe | Base Fill | Backdrop Filter | Border | Shadow Elevation | Applied To |
|---|---|---|---|---|---|
| **Pane** | `surface.glass` | `blur(24px) saturate(160%)` | 1px `border.glass` | `0 4px 16px rgba(0,0,0,0.25)` | Cockpit sidebars, topology drawer |
| **Float** | `surface.glass-strong` | `blur(32px) saturate(190%)` | 1px `border.glass` | `0 8px 32px -4px rgba(0,0,0,0.50)` | Floating HUD, approval action cards |
| **Modal** | `surface.glass-strong` | `blur(36px)` | 1px `border.subtle` | `0 16px 48px -8px rgba(0,0,0,0.65)` | Model download modal, diff reviewers |

*Surface law:* Depth comes from surface steps and quiet hairlines. No colored glows, neon borders, specular gradient overlays or radial color washes. The frosted fill and blur are the effect; the scrim floor keeps text readable.

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
