# PRD — Starship Control

**Status:** Draft v0.1 · **Owner:** zvirox · **Date:** 2026-11-17  
**Project:** Mother Ship / Starship Control  
**Authority:** Root Product Requirements Document for Starship Control  

---

## 1. Overview & Vision

**Starship Control** is a local-first, native Linux AI agent orchestrator and unified command center for desktop software. It bridges autonomous AI models (both local and cloud) with specialized, high-performance desktop applications through the Model Context Protocol (MCP) and Linux-native IPC.

Today, creative and system utility workflows across Linux are fragmented. Independent applications like **Easy Edits Pro (EEP)** (video editor), **ScreenStudio Pro (SSP)** (screen recorder), **Audio Labs Pro (ALP)** (audio studio & voice cloner), **Net Watch Pro (NWP)** (network monitor), and **Agentic Tuneup Pro (ATP)** (system optimizer) each expose deep, capable agentic tool surfaces. However, there is no centralized, intelligent "mission control" capable of coordinating them into autonomous, multi-stage pipelines.

**Vision:** The intelligent command cockpit for Linux creators and power users. A user provides high-level intent ("Record this demonstration, isolate and clean the vocal track, trim silence and dead time using mouse telemetry, and render a final YouTube video"), and Starship Control plans, coordinates, commands, and verifies the entire workflow across the specialized sub-applications.

**One-line pitch:** *The AI Agent Cockpit for Linux — command, coordinate, and automate your desktop apps from a single, beautiful interface.*

---

## 2. Problem Statement

Modern open-source desktop software is increasingly embracing AI and MCP interfaces, but users face distinct bottlenecks:

1. **Siloed Agent Capabilities:** Each application (EEP, SSP, ALP, etc.) implements or plans an MCP server, but each operates in isolation. A video editor does not know when a screen recording finishes; an audio synthesizer does not know where the video project timeline needs voiceover clips.
2. **Missing Local Agent Cockpit:** Generic coding-agent harnesses (Claude Code, Cursor, generic chatbots) lack native awareness of Linux desktop environments, Wayland compositors, media pipelines, or local UNIX domain socket protocols.
3. **The Local vs. Cloud Dilemma:** Users must either surrender privacy and funds to cloud APIs or struggle through complex terminal setups to run local models. There is no unified desktop manager that natively blends local Hugging Face GGUF models with cloud APIs under a single task-routing layer.
4. **Context Switching & Friction:** When working in an application like EEP or ALP, switching to a full terminal or web chat to command an agent breaks creative flow. Users need an ambient, non-intrusive way to prompt the agent directly over the active workspace.
5. **Lack of Human-in-the-Loop Safety:** Autonomous agents operating across the filesystem and desktop apps risk destructive errors without granular permission controls and transparent preview diffs.

---

## 3. Target Users & Personas

### Persona 1: "The Linux Content Creator" (Primary)
- **Profile:** Produces software demos, devlogs, tutorials, or podcasts on Linux (KDE Plasma, Hyprland, GNOME). Uses SSP for recording, ALP for vocal tuning and TTS narration, and EEP for editing.
- **Pain Points:** Repetitive, manual export/import chains; manually cutting dead air; generating SRT subtitles and aligning voice tracks.
- **Success Criteria:** Issues a single prompt into Starship Control's Floating HUD while looking at EEP: "Process my last recording, enhance audio with ALP, and auto-cut dead frames." The agent executes the plan and presents an interactive diff for one-click approval.

### Persona 2: "The Privacy-Conscious Power User"
- **Profile:** Strict data sovereignty requirements. Refuses to send system telemetry, network logs, or proprietary audio/video takes to third-party cloud servers.
- **Pain Points:** Most modern agentic assistants require proprietary cloud endpoints.
- **Success Criteria:** Downloads a quantized GGUF model directly via Starship Control's Model Manager from Hugging Face, running 100% offline via local hardware acceleration (Vulkan/ROCm/CPU) with complete data confidentiality.

### Persona 3: "The SysAdmin & Power Developer"
- **Profile:** Manages complex Linux workstations; monitors network leaks and data usage with NWP; optimizes packages and cache bloat with ATP.
- **Pain Points:** Must run independent CLI tools, cron scripts, and ad-hoc monitors to diagnose machine state.
- **Success Criteria:** Starship Control acts as an autonomous systems copilot, synthesizing network diagnostics from NWP and disk health from ATP to remediate system bottlenecks upon approval.

---

## 4. Goals & Non-Goals

### Product Goals (MVP)
- **G1. Dual-Surface Interface:**
  - **The Cockpit:** A comprehensive primary desktop window (1200×800) for mission planning, multi-app topology status, workflow visualization, model management, and audit logs.
  - **The Floating HUD:** A lightweight, always-on-top, draggable frosted-glass companion bar (mirroring SSP's design philosophy) with global shortcut invocation for quick prompts and one-click approvals without leaving active apps.
- **G2. Dual Engine (Local + Cloud) from Day One:**
  - Local model runtime supporting direct Hugging Face GGUF downloads with download progress, SHA-256 verification, and hardware-accelerated local execution.
  - Universal Cloud API connectivity supporting custom Base URLs, API keys, and model IDs (compatible with OpenAI, Anthropic, Gemini, OpenRouter, and local OpenAI-compatible endpoints like Ollama).
- **G3. Pre-Installed Plugin / Bridge Architecture (`starship-plugin`):**
  - Standardized, zero-overhead Rust SDK crate imported by sub-apps (EEP, SSP, ALP, NWP, ATP) providing zero-configuration auto-discovery via XDG manifests.
- **G4. Hybrid Control Plane:**
  - Real-time live GUI control via Linux UNIX Domain Sockets (`SO_PEERCRED` authenticated) when target apps are open.
  - Headless background execution via stdio MCP CLI binaries when apps are closed.
- **G5. Tiered Safety & Approval Gates:**
  - Read-only inspections execute autonomously.
  - Mutating actions (deletions, project modifications, take overwrites, cloud token expenditures) pause for explicit human approval via interactive preview diffs on both Cockpit and HUD.
- **G6. Ecosystem Consistency:** Built with Rust, Tauri 2, React 19, TypeScript, and Tailwind CSS v4, matching the visual design tokens, fonts, and frosted-glass aesthetics of EEP and SSP.

### Non-Goals (MVP)
- **N1. Not a Generic Chatbot:** Starship Control is an application and task orchestrator, not a generic conversational assistant. Every action is grounded in tools, task sheets, and application control.
- **N2. Not a Re-implementation of Sub-App Features:** Starship Control never duplicates video rendering, audio mixing, or packet filtering; it delegates directly to EEP, ALP, NWP, SSP, and ATP.
- **N3. No Proprietary SaaS Accounts or Telemetry:** No mandatory external account logins, telemetry, or remote tracking. All settings and mission databases remain strictly local in SQLite (`library.db`).
- **N4. Non-Linux Platforms:** Linux-first and Linux-exclusive for MVP (Wayland primary, X11 fallback). Windows and macOS are deferred to post-1.0 phases.

---

## 5. Ecosystem & App Integration Map

Starship Control coordinates the following primary applications and formats:

| App ID | Name | Role & Domain | Primary Control Channel | Typical Agent Use Cases |
|---|---|---|---|---|
| **`eep`** | Easy Edits Pro | NLE Video Editor | Unix Socket (GUI) / stdio MCP CLI | Timeline editing, split cuts, color grading, asset assembly, rendering. |
| **`ssp`** | ScreenStudio Pro | Screen Recorder & HUD | Unix Socket (GUI) / D-Bus / CLI | Starting/stopping takes, configuring regions/sources, exporting CET telemetry. |
| **`alp`** | Audio Labs Pro | Multitrack Audio & Voice Lab | Unix Socket (GUI) / stdio MCP CLI | Voice cleanup, noise reduction, TTS narration generation, voice cloning. |
| **`nwp`** | Net Watch Pro | Network & Data Tracker | Unix Socket (`netwatchd` root IPC) | Network leak auditing, application data usage telemetry, connection tracing. |
| **`atp`** | Agentic Tuneup Pro | PC Cleaner & Package Manager | Unix Socket / CLI (`atpd`) | Cache purging, orphan package removal, disk space reclamation, health checks. |
| **`cet`** | Capture Event Track | Telemetry Plugin / Specification | `.cet.jsonl` Sidecar Reader | Parsing user clicks, cursor motion, and window focus for automated editing cuts. |

---

## 6. Core Functional Requirements

### 6.1 Mission Planning & Execution Engine
- **Task Decomposition:** Translates natural-language requests into structured, versioned **Task Sheets** (`tasksheet/1` format aligned with EEP).
- **Dependency Graph:** Identifies sequential and parallel execution stages (e.g., SSP capture must finish before CET parsing and ALP audio synthesis can feed EEP assembly).
- **Execution State Machine:** Tracks steps across states: `Pending`, `Running`, `WaitingApproval`, `Completed`, `Failed`, `RolledBack`.

### 6.2 Model Management & Selection ("Model Control")
- **Dual Selection Mode:** Users can globally or per-mission select between **Local LLM** or **Cloud API**.
- **Hugging Face Downloader:**
  - Direct repository search / direct model URI input for GGUF models.
  - Multi-stream chunked downloading with resume support and SHA-256 integrity verification.
  - Disk storage management at `$XDG_DATA_HOME/starship-control/models/`.
  - Configurable GPU offload (GPU layers, context length, thread allocation).
- **Cloud Provider Gateway:**
  - Custom endpoint management: Base URL, API Key, Model ID, Organization/Header parameters.
  - Pre-configured presets for OpenAI, Anthropic, Google Gemini, and OpenRouter.
  - Real-time token consumption tracking and cost estimator.

### 6.3 App Discovery & Plugin Bridge
- **Discovery Service:** Watches `$XDG_DATA_HOME/starship/plugins/*.json` and `$XDG_RUNTIME_DIR/starship/bridges/*.sock` for active application manifests.
- **Automatic App Wake-Up:** Can spawn applications into either active GUI mode or headless CLI daemon mode depending on task requirements.
- **Live State Synchronization:** Subscribes to application status events (e.g., recording active in SSP, render in progress in EEP).

### 6.4 Tiered Safety & Human Approval System
- **Tier 1 (Safe / Autonomous):** Read-only queries (fetching project status, scanning timeline markers, inspecting media metadata, checking network telemetry). No user prompt required.
- **Tier 2 (Mutating / Approval Gate):** Modifying timelines, applying audio effects, deleting temporary assets, spending cloud API tokens over a set budget. Requires single-click approval.
- **Tier 3 (Destructive / High-Stakes):** File system purges (ATP), take file deletions, system package modifications. Requires explicit confirmation modal with full diff and impact warning.
- **Interactive Preview Diffs:** Displays visual summaries of proposed modifications (e.g., "Will remove 14 dead-time segments totaling 01:22 from EEP timeline").

### 6.5 User Experience & Surfaces
- **Cockpit Desktop Window:**
  - Multi-panel frosted glass interface (Familjen Grotesk headers, JetBrains Mono numbers/code).
  - Central mission stream with streaming token markdown, thought inspection, and tool-call cards.
  - Right-hand App Topology Drawer showing live status badges for all discovered applications.
  - Dedicated **Model Control** settings tab for Hugging Face downloads and API keys.
- **Floating Companion HUD:**
  - Always-on-top, draggable, compact glass strip.
  - Hotkey toggle (default: `Super + Shift + Space`).
  - Voice/text input bar with mini audio waveform indicator.
  - Inline action cards for Tier 2/3 approvals (Approve / Reject buttons).

---

## 7. User Journeys

### Journey 1: Automated Screencast to Polished Video
1. User records a 10-minute programming tutorial using **ScreenStudio Pro**.
2. Once recording stops, SSP writes `demo.mkv` and `demo.cet.jsonl`.
3. User presses `Super + Shift + Space` to summon Starship Control HUD over EEP.
4. User types: *"Import my latest recording, remove silences, generate an AI intro voiceover introducing Starship Control with ALP, and zoom in on terminal clicks."*
5. Starship Control reads CET telemetry, sends audio track to ALP for speech enhancement, plans cut points, and sends a Task Sheet to EEP.
6. HUD displays an approval card: *"Plan ready: 18 cuts, 1 vocal enhancement, 4 zoom tracks. [Approve] [Review]"*.
7. User clicks **Approve**; EEP updates its timeline live in front of the user.

### Journey 2: Pure Local & Private System Maintenance
1. Privacy-conscious user opens Starship Cockpit $\to$ Model Control.
2. Selects Hugging Face GGUF model (e.g., `Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf`), clicks Download.
3. Once downloaded, toggles Active Engine to **Local**.
4. Prompts: *"Analyze my network activity over the last 24 hours with NWP and clean up stale package caches with ATP."*
5. Model runs locally; Starship queries NWP read-only, identifies 3 GB of cache in ATP, and presents an approval card to clean the caches.
6. Entire transaction executes without a single byte leaving the workstation.

---

## 8. Technical Assumptions & Distro Compatibility

- **OS Target:** Linux 64-bit (x86_64 and aarch64).
- **Desktop Environments:**
  - Tier 1: Wayland on KDE Plasma 6+, Hyprland, niri, Sway, MangoWC.
  - Tier 2: GNOME Wayland (subject to standard portal/blur constraints), X11 fallback.
- **IPC Transports:** Linux UNIX domain sockets, `SO_PEERCRED`, stdio pipelines, POSIX signals.
- **Storage:** SQLite 3 with Write-Ahead Logging (WAL) and strict 0600 file permissions.

---

## 9. Key Risks & Mitigations

| Risk | Impact | Mitigation Strategy |
|---|---|---|
| **Local LLM Performance Bottlenecks** | Agent responses lag or exhaust host RAM/VRAM. | Strict quantization defaults (Q4_K_M), automatic hardware capability detection (RAM/VRAM probe before loading), and clear model sizing recommendations in the downloader UI. |
| **Cross-App Deadlocks / Contention** | Multiple agents or Starship Control competing for project locks (e.g., EEP project lock). | Implement explicit lease-based locking in `starship-plugin` with timeout and auto-release semantics. |
| **Unintended Destructive Mutations** | Agent misinterprets prompt and deletes user media or project files. | Tiered safety gates are enforced at the Rust core level, not prompt-instructed. Mutating tool calls cannot execute without signed user token confirmation. |
| **API Key Leakage** | Cloud API keys exposed to sub-apps or untrusted logs. | API keys stored in local SQLite database with optional system keyring (Secret Service API / `keyutils`), strictly sanitized from all mission audit logs and prompts. |

---

## 10. Success Metrics

- **Time-to-Orchestration:** Less than 5 seconds to wake up and connect to a running sub-application.
- **One-Click Approval Latency:** Instantaneous (<100ms) transition from user approval click to tool execution dispatch.
- **Zero-Configuration Discovery:** 100% of apps shipping `starship-plugin` automatically detected without manual path or port entry.
- **Resource Footprint:** Starship Control idle memory usage under 120 MB RSS (when running cloud mode or idling local engine).
