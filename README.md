<p align="center">
  <h1 align="center">Starship Control</h1>
</p>

<p align="center">
  <strong>The Local-First AI Agent Cockpit & Multi-App Orchestrator for Linux</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/starship-control/releases"><img src="https://img.shields.io/badge/Release-v0.0.1--planning-blue.svg" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <a href="#4-tech-stack"><img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" /></a>
  <a href="#4-tech-stack"><img src="https://img.shields.io/badge/Rust-1.92+-informational.svg" alt="Rust" /></a>
  <a href="#4-tech-stack"><img src="https://img.shields.io/badge/Shell-Tauri_2_·_Frosted_Glass-indigo.svg" alt="Tauri 2" /></a>
  <a href="#4-tech-stack"><img src="https://img.shields.io/badge/Protocol-MCP_JSON--RPC_2.0-brightgreen.svg" alt="MCP" /></a>
</p>

<p align="center">
  <a href="PRD.md">Product Requirements (PRD)</a> ·
  <a href="ARCHITECTURE.md">System Architecture</a> ·
  <a href="DESIGN.md">Design System</a> ·
  <a href="DATABASE.md">Database Schema</a> ·
  <a href="API.md">IPC & Protocols</a> ·
  <a href="PLAN.md">Roadmap & Plan</a> ·
  <a href="AGENTS.md">Contributor Rules</a>
</p>

---

## 1. Overview & Vision

**Starship Control** is the mission command center and central AI agent orchestrator for the **Mother Ship** Linux native software suite. It acts as the intelligent bridge connecting autonomous AI models (both local GGUF models running directly on device hardware and frontier cloud APIs) with specialized high-performance desktop applications.

On Linux today, specialized creative tools and systems monitors each expose deep, capable agentic tool surfaces via the **Model Context Protocol (MCP)**. However, these tools operate in isolated silos. Starship Control unites them: users state their high-level intent, and Starship Control plans, decomposes, coordinates, and verifies multi-app execution pipelines across the entire ecosystem.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        STARSHIP CONTROL TOPOLOGY                       │
└────────────────────────────────────────────────────────────────────────┘

 [ USER SURFACES ]
   ┌────────────────────────────────┐    ┌───────────────────────────────┐
   │         COCKPIT WINDOW         │    │         FLOATING HUD          │
   │  1200×800 Strategic Workspace  │    │  520×76 Companion Glass Bar   │
   │  Mission Stream · Model Mgr    │    │  Global Shortcut: Super+Shift+Space│
   │  App Topology · Audit Trail    │    │  Dynamic Approval Expansion   │
   └───────────────┬────────────────┘    └───────────────┬───────────────┘
                   │                                     │
                   └──────────────────┬──────────────────┘
                                      │ Tauri IPC (`invoke` / `emit`)
                                      ▼
 [ STARSHIP CONTROL CORE (Rust 2024 / Tokio Runtime) ]
   ┌─────────────────────────────────────────────────────────────────────┐
   │                        STARSHIP ORCHESTRATOR                        │
   │   Task Sheet Planner (`tasksheet/1`) · Tiered Safety Arbiter        │
   │   Context Synthesizer · In-Memory Lease & Lock Supervisor           │
   └──────────────┬───────────────────┬───────────────────┬──────────────┘
                  │                   │                   │
                  ▼                   ▼                   ▼
   ┌──────────────────────┐ ┌──────────────────┐ ┌──────────────────────┐
   │    STARSHIP-INFER    │ │   STARSHIP-MCP   │ │     STARSHIP-DB      │
   │  Dual Engine Facade  │ │ Client Manager   │ │ SQLite Engine (WAL)  │
   ├──────────────────────┤ ├──────────────────┤ ├──────────────────────┤
   │ • Local: llama-cpp-2 │ │ • Unix Sockets   │ │ • missions & messages│
   │   (Vulkan/ROCm/CPU)  │ │   (SO_PEERCRED)  │ │ • plans & step DAG   │
   │ • Cloud: reqwest SSE │ │ • Stdio Spawner  │ │ • app_registry       │
   │   (OpenAI, Claude,   │ │ • Tool Discovery │ │ • models & providers │
   │    Gemini, OpenRouter│ │ • JSON-RPC 2.0   │ │ • audit_log (0600)   │
   │ • HF Chunked Downld  │ │                  │ │                      │
   │ • SHA-256 Verifier   │ │                  │ │                      │
   └──────────────────────┘ └─────────┬────────┘ └──────────────────────┘
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │ Unix Sockets (Live GUI) │ Stdio Pipes (CLI MCP)   │
            ▼                         ▼                         ▼
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │ EASY EDITS PRO  │       │SCREENSTUDIO PRO │       │ AUDIO LABS PRO  │
   │ (Video Editor)  │       │(Screen Recorder)│       │ (Voice Studio)  │
   │ starship-plugin │       │ starship-plugin │       │ starship-plugin │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
            │                         │                         │
            ▼                         ▼                         ▼
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │  NET WATCH PRO  │       │AGENTIC TUNEUP PRO       │  FUTURE APPS    │
   │(Network Monitor)│       │ (System Cleaner)│       │(Plugin Bridge)  │
   │ starship-plugin │       │ starship-plugin │       │ starship-plugin │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
```

---

## 2. The Orchestrated App Ecosystem

Starship Control coordinates the following primary applications and telemetry formats via the shared **`starship-plugin` SDK**:

| Sub-Application | Domain & Role | Tech Stack | Primary Capabilities Orchestrated |
|---|---|---|---|
| [**Easy Edits Pro (EEP)**](https://github.com/zvirox/easy-edits-pro) | NLE Video Editor | Rust, Tauri 2, React, GStreamer | 490+ editing verbs, multi-track timeline cuts, scene-referred color grading, audio track alignment, rendering. |
| [**ScreenStudio Pro (SSP)**](https://github.com/zvirox/screen-studio-pro) | Screen Recorder & HUD | Rust, Tauri 2, React 19, PipeWire | Wayland monitor/window/region capture, PipeWire audio, webcam PiP, hardware encoding, CET event logging. |
| [**Audio Labs Pro (ALP)**](https://github.com/zvirox/audio-labs-pro) | Audio Studio & Voice Lab | Rust 1.92+, iced 0.14, ONNX | Multitrack audio recording, voice cleanup/denoising, local Kokoro-82M TTS, Moonshine STT dictation, voice cloning. |
| [**Net Watch Pro (NWP)**](https://github.com/zvirox/net-watch-pro) | Network & Data Tracker | Rust, Tauri 2, Svelte 5, Aya eBPF | In-kernel eBPF per-process byte accounting, connection auditing, data usage tracking, network leak detection. |
| [**Agentic Tuneup Pro (ATP)**](https://github.com/zvirox/agentic-tuneup-pro) | System Cleaner & Optimizer | Rust, Tauri 2, React 19 | Package manager cache pruning (pacman/apt/dnf/flatpak), dev artifact cleanup, orphan package removals. |
| [**Capture Event Track (CET)**](https://github.com/zvirox/capture-event-track) | Telemetry Specification | JSON Lines (`.cet.jsonl`) | Recorder-neutral telemetry describing mouse clicks, cursor motion, and window focus used to automate cuts in EEP. |

---

## 3. Core Architectural Highlights

### 3.1 Dual-Surface Interface
- **The Cockpit (1200×800):** Primary mission control window. Features a 3-column workspace with a searchable session drawer, central streaming dialogue with interactive Task Sheet step cards, live App Topology drawer, and dedicated Model Control settings.
- **The Floating HUD (520×76 resting, 520×240 expanded):** Translucent, always-on-top glass capsule toggleable via `Super + Shift + Space` (Wayland global shortcut portal via `ashpd`). Stays pinned above active applications for ambient prompting and automatically expands with spring physics when presenting human approval action cards.

### 3.2 Dual Engine Model Control from Day One
- **Local LLMs:** In-app Hugging Face browser and chunked resumable downloader. Executes GGUF models directly on workstation hardware (Vulkan/ROCm/CPU) via native `llama-cpp-2` bindings with streaming SHA-256 integrity validation and VRAM capacity probes.
- **Cloud APIs:** Universal gateway supporting custom Base URLs, API keys, and model IDs for OpenAI, Anthropic Claude, Google Gemini, OpenRouter, and local OpenAI-compatible daemons (Ollama/vLLM).

### 3.3 Hybrid Control Plane
- **Live GUI Mode (Unix Domain Sockets):** When a target app is open, Starship Control communicates over Unix domain sockets at `$XDG_RUNTIME_DIR/starship/bridges/<app-id>.sock`. Linux kernel `SO_PEERCRED` checks verify the calling UID matches the host user, enabling zero-copy real-time timeline updates.
- **Headless Background Mode (stdio MCP):** When an app is closed, Starship Control spawns its headless CLI binary (e.g. `easy-edits-pro --mcp`, `alp-mcp`) as a background child process, completing batch tasks without opening windows.

### 3.4 Tiered Safety & Human-in-the-Loop Gates
Safety is enforced in the Rust core, not left to LLM prompt adherence:
- **Tier 1 (Safe / Autonomous):** Read-only queries (reading timelines, checking network counters, inspecting media tags) execute automatically.
- **Tier 2 (Mutating) & Tier 3 (Destructive):** Pauses the agent state machine, generates an interactive diff summary, and requires an explicit user signature token via `[Approve]` or `[Reject]` actions on the Cockpit or Floating HUD.

---

## 4. Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| **App Shell** | **Tauri 2** | Multi-window support (Cockpit + transparent Floating HUD), system tray, Wayland global shortcuts, tiny binary size. |
| **Frontend Framework** | **React 19 + TypeScript + Vite** | Fast HMR dev loop, strict end-to-end typing mirroring Rust IPC payloads. |
| **Styling & Design** | **Tailwind CSS v4** | Pure CSS `@theme` tokens matching DESIGN.md v1.0 (shared across EEP and SSP). |
| **UI Components** | **Radix UI Primitives + Lucide Icons** | Accessible keyboard-operable primitives customized to frosted-glass tokens. |
| **UI Behaviors & Agent Bridge** | **`@ui-sdk/react` + `@ui-sdk/core`** | 50+ headless interaction primitives (popups, roving focus, selection, createAgent) from `https://ui-sdk.dev/`. |
| **Animations** | **framer-motion** | Physics-based spring animations for HUD expansion and approval card slide-ins. |
| **Core Runtime** | **Rust 2024 + Tokio** | High-throughput asynchronous runtime managing concurrent MCP streams, child processes, and SSE. |
| **Local LLM Engine** | **`llama-cpp-2`** | High-performance C++ bindings for GGUF execution with Vulkan, ROCm, and AVX2 acceleration. |
| **Cloud Model Client**| **`reqwest` (HTTP/2 SSE)** | Streaming client for OpenAI, Anthropic, Gemini, and OpenRouter endpoints. |
| **Protocol** | **Model Context Protocol (MCP)** | JSON-RPC 2.0 protocol for dynamic tool discovery, execution, and cancellation. |
| **IPC Transport** | **Linux Unix Domain Sockets** | Peer-credential (`SO_PEERCRED`) authenticated local IPC. |
| **Database** | **SQLite 3 (`rusqlite`)** | WAL mode, `PRAGMA foreign_keys = ON;`, `0600` permissions at `$XDG_DATA_HOME/starship-control/library.db`. |

---

## 5. Source-of-Truth Hierarchy

All development in this repository strictly adheres to the following specification documents (lower-numbered files win every conflict):

| Priority | Document | Governs |
|---|---|---|
| **1** | [**`PRD.md`**](PRD.md) | Scope, MVP vs. Later, user personas, functional requirements, and safety policies. |
| **2** | [**`ARCHITECTURE.md`**](ARCHITECTURE.md) | Crate boundaries, system topology, IPC transports, and lifecycle contracts. |
| **3** | [**`DESIGN.md`**](DESIGN.md) | Visual law: frosted-glass recipes, Electric Cyan tokens, typography, window layouts. |
| **4** | [**`DATABASE.md`**](DATABASE.md) | SQLite schema, tables (`missions`, `plans`, `models`, `audit_log`), migrations, and retention. |
| **5** | [**`API.md`**](API.md) | Typed Tauri commands, push events, and `starship-plugin` MCP wire protocol. |
| **6** | [**`PLAN.md`**](PLAN.md) | Implementation roadmap, vertical slices, and verifiable acceptance criteria. |
| **—** | [**`AGENTS.md`**](AGENTS.md) | Developer & AI Contributor Rules: coding rules, architecture invariants, and verification gates. |

---

## 6. License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
