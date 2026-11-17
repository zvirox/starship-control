<p align="center">
  <h1 align="center">Starship Control</h1>
</p>

<p align="center">
  <strong>The AI Agent Cockpit & Multi-App Orchestrator for Linux</strong>
</p>

<p align="center">
  <a href="https://github.com/zvirox/starship-control/releases"><img src="https://img.shields.io/badge/Release-v0.0.1--planning-blue.svg" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License" /></a>
  <a href="#tech-stack"><img src="https://img.shields.io/badge/Platform-Linux_(Wayland_&_X11)-orange.svg" alt="Platform" /></a>
  <a href="#tech-stack"><img src="https://img.shields.io/badge/Rust-1.92+-informational.svg" alt="Rust" /></a>
  <a href="#tech-stack"><img src="https://img.shields.io/badge/Shell-Tauri_2_·_Frosted_Glass-indigo.svg" alt="Tauri 2" /></a>
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

## 1. Overview

**Starship Control** is the mission control center and autonomous AI agent orchestrator for your Linux desktop software suite. It coordinates specialized desktop applications—including **Easy Edits Pro (EEP)**, **ScreenStudio Pro (SSP)**, **Audio Labs Pro (ALP)**, **Net Watch Pro (NWP)**, and **Agentic Tuneup Pro (ATP)**—through the Model Context Protocol (MCP) and Linux-native IPC.

Whether running fully local GGUF models downloaded from Hugging Face or leveraging cloud frontier models (OpenAI, Anthropic, Gemini, OpenRouter), Starship Control translates high-level creative and system intent into multi-stage, coordinated desktop actions.

```
┌────────────────────────────────────────────────────────────────────────┐
│                        STARSHIP CONTROL TOPOLOGY                       │
└────────────────────────────────────────────────────────────────────────┘

 [ USER SURFACES ]
   ┌────────────────────────────────┐    ┌───────────────────────────────┐
   │         COCKPIT WINDOW         │    │         FLOATING HUD          │
   │  1200×800 Mission Headquarters │    │  Always-on-top Glass Capsule  │
   └───────────────┬────────────────┘    └───────────────┬───────────────┘
                   │                                     │
                   └──────────────────┬──────────────────┘
                                      ▼
 [ STARSHIP CONTROL CORE (Rust / Tokio Runtime) ]
   ┌─────────────────────────────────────────────────────────────────────┐
   │    Task Planner · Tiered Safety Arbiter · MCP Client Manager        │
   │    Local GGUF Engine (llama-cpp-2) · Cloud SSE Gateway (reqwest)    │
   │    SQLite WAL Storage (library.db) · Manifest Watcher & Discovery   │
   └──────────────────────────────────┬──────────────────────────────────┘
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
   │(Network Monitor)│       │(System Cleaner) │       │(Plugin Bridge)  │
   │ starship-plugin │       │ starship-plugin │       │ starship-plugin │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
```

---

## 2. Key Features

- **Dual-Surface Interface:**
  - **The Cockpit:** Full 1200×800 management window for strategic planning, model downloading, topology inspection, and audit logs.
  - **The Floating HUD:** Always-on-top frosted-glass capsule (`Super + Shift + Space`) that sits over active apps like EEP or ALP for instant voice/text prompts and one-click approvals.
- **Dual Engine (Local + Cloud) from Day One:**
  - **Local Models:** In-app Hugging Face downloader with resumable chunked downloads, streaming SHA-256 verification, and hardware-accelerated local execution (Vulkan/ROCm/CPU) via `llama-cpp-2`.
  - **Cloud APIs:** Universal provider settings for custom Base URLs, API keys, and model IDs (compatible with OpenAI, Anthropic, Gemini, OpenRouter, and Ollama).
- **Hybrid Control Plane:**
  - Dispatches commands over Linux **Unix Domain Sockets** (`SO_PEERCRED` authenticated) when target apps are open for real-time visual UI manipulation.
  - Spawns headless CLI binaries with stdio MCP when apps are closed for background batch execution.
- **Tiered Safety Policy:**
  - **Tier 1 (Safe):** Read-only inspections execute autonomously.
  - **Tier 2 (Mutating) & Tier 3 (Destructive):** Pauses execution and displays an interactive preview diff card with `[Approve]` / `[Reject]` buttons.
- **Shared Rust SDK (`starship-plugin`):** Zero-overhead crate embedded in sub-apps for auto-discovery via `$XDG_DATA_HOME/starship/plugins/*.json` manifests.

---

## 3. Tech Stack

| Layer | Choice |
|---|---|
| **App Shell** | Tauri 2 (Multi-window: Cockpit + Floating HUD) |
| **Frontend** | React 19 + TypeScript + Vite |
| **Styling** | Tailwind CSS v4 (`@theme` tokens matching EEP & SSP) |
| **Components & Icons** | Radix UI primitives + Lucide Icons |
| **Animations** | `framer-motion` (Spring physics) |
| **Core Runtime** | Rust 2024 edition + Tokio async |
| **Local LLM Engine** | `llama-cpp-2` (GGUF runner with Vulkan/ROCm/CPU) |
| **Cloud Model Client**| `reqwest` (HTTP/2 SSE client) |
| **Protocol** | Model Context Protocol (MCP) JSON-RPC 2.0 |
| **IPC Transport** | Linux Unix Domain Sockets (`SO_PEERCRED`) |
| **Database** | SQLite 3 (`rusqlite`) in WAL mode (`0600` permissions) |
| **Desktop Portals** | `ashpd` (Global shortcuts & notifications) |

---

## 4. Source of Truth Documents

All architectural, functional, and design laws are formalized in these specifications:

1. [**`PRD.md`**](PRD.md) — Product Requirements Document: Vision, user personas, functional requirements, and safety policies.
2. [**`ARCHITECTURE.md`**](ARCHITECTURE.md) — Technical Blueprint: Crate breakdown, system topology, process boundaries, and protocols.
3. [**`DESIGN.md`**](DESIGN.md) — Design System: Frosted glass tokens, typography, Cockpit layout, Floating HUD mechanics.
4. [**`DATABASE.md`**](DATABASE.md) — Data Architecture: SQLite schemas (`missions`, `plans`, `steps`, `models`, `audit_log`), indexes, migrations.
5. [**`API.md`**](API.md) — IPC & Protocol Contracts: Typed Tauri commands, push events, and `starship-plugin` MCP wire format.
6. [**`PLAN.md`**](PLAN.md) — Implementation Roadmap: Vertical slices, phase-by-phase deliverables, and verification gates.
7. [**`AGENTS.md`**](AGENTS.md) — Developer & AI Contributor Rules: Coding rules, architecture invariants, and verification laws.

---

## 5. License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
