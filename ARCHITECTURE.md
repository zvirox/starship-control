# ARCHITECTURE.md — Starship Control

**Version:** 2.0 · **Date:** 2026-11-25  
**Authority chain:** `PRD.md` (scope, requirements) → `ARCHITECTURE.md` (system structure, contracts) → `DESIGN.md` (locked UI law)  
**Rule:** Defines technical structure, crate layout, protocols, process boundaries, and lifecycle contracts. No product code is written as part of this document.

---

## 1. Architecture Principles

1. **Local-First & Sovereign by Default:** Starship Control operates fully on-device without phoning home. Local GGUF models run through a managed `llama-server` sidecar (OpenAI-compatible HTTP over loopback), with hardware/VRAM probing and a live headroom guard. Cloud APIs are strictly optional, user-configured, and directly addressed with the user's private keys.
2. **Two Surfaces, One Core:** The system runs a single core process hosting the agent orchestration runtime, SQLite database, and MCP client manager. It drives two distinct UI surfaces via Tauri 2:
   - **The Cockpit:** Primary window (`1280×800`, min `1024×640`, solid Linear UI) for strategic mission planning, timelines, workflows, model downloading, app topology inspection, live systems health, and audit review.
   - **The Floating HUD:** Always-on-top, draggable, solid matte companion bar (`420×64`, `transparent: false`, synced dark/light theme) that sits over other applications for quick prompts, live progress, and instant approvals.
3. **Sub-Apps as Autonomous MCP Agents:** Every sub-application (Easy Edits Pro, ScreenStudio Pro, Audio Labs Pro, Net Watch Pro, Agentic Tuneup Pro) is treated as an autonomous agent exposing tool surfaces through the Model Context Protocol (MCP). Starship Control does not duplicate their media engines or low-level logic—it orchestrates them.
4. **Shared Rust SDK (`starship-plugin`):** A zero-dependency, shared Rust crate imported by each sub-app handles manifest registration, socket creation with Linux `SO_PEERCRED` authentication, and handshake negotiation.
5. **Hybrid Control Plane (Live GUI vs. Headless CLI):**
   - If the target app is actively open on the desktop, Starship Control connects to its live GUI socket to provide instant visual feedback on canvas/timeline.
   - If the app is closed, Starship Control spawns its headless CLI binary in stdio MCP mode to complete background batch tasks without screen clutter.
6. **Tiered Safety Enforced in Native Code:** Safety is never left to LLM prompt compliance. The Rust orchestrator categorizes tool calls into three risk tiers, intercepting mutating and high-stakes operations before tool dispatch and requiring signed UI approval tokens.
7. **SQLite is the Single Source of Persisted Truth:** All mission sessions, message streams, task sheet plans, execution traces, registered models, and audit logs are recorded in a single local SQLite database in WAL mode with strict `0600` permissions.

---

## 2. Tech Stack Specification

| Layer | Technology | Rationale |
|---|---|---|
| **App Shell** | **Tauri 2** | Multi-window support (Cockpit + solid opaque Floating HUD), desktop notifications, Wayland global shortcut portal, tiny memory footprint. |
| **Frontend Framework** | **React 19 + TypeScript + Vite** | High-performance reactive UI, strict end-to-end typing matching the Rust IPC layer, fast Vite HMR loop. |
| **Styling & Design** | **Tailwind CSS v4** | Pure CSS `@theme` tokens matching **locked** `DESIGN.md` v2.1 (Linear solid surfaces, `Inter` + `JetBrains Mono` self-hosted fonts, zero blur, dark & light modes, enforced via `npm run check:linear`). |
| **UI Components & Charts** | **Custom Linear Primitives + Lucide Icons + Apache ECharts 6** | Accessible keyboard-first primitives (`TitleBar`, `Sidebar`, `Breadcrumb`, `DataTable`, `StatCard`, `StatusPill`, `LinearDialog`, `LinearTelemetryChart`) with precomputed ECharts 6 dark/light themes. |
| **UI Behaviors & Agent Bridge** | **`@ui-sdk/react` + `@ui-sdk/core` + Zustand 5** | Headless behavior primitives and synchronized cross-window theme/app state. |
| **Animation & Motion** | **framer-motion / motion** | Snappy `150ms ease-out` UI transitions compliant with `prefers-reduced-motion` and zero blur transitions. |
| **Async Runtime (Core)** | **Rust 2024 + Tokio** | High-throughput asynchronous runtime managing concurrent MCP transports, streaming SSE connections, system health monitoring, and child processes. |
| **Local LLM Engine** | **`starship-infer/local/` runner + `llama-server` sidecar** | Hardware/VRAM probe, `LocalInferenceGuard`, managed runtime lifecycle, real GGUF streaming and GBNF passthrough (in-process `llama-cpp-2` bindings remain an optional later path). |
| **Cloud Model Client** | **`reqwest` (HTTP/2, rustls)** | Streaming Server-Sent Events (SSE) client for OpenAI, Anthropic, Gemini, and OpenRouter endpoints. |
| **Protocol / Tooling** | **Model Context Protocol (MCP)** | Official protocol for discovering tools, schemas, resources, and executing actions across sub-apps. |
| **IPC Transport (Sub-Apps)** | **Linux UNIX Domain Sockets (`SO_PEERCRED`)** | High-speed local IPC verified by kernel UID/binary/manifest checks, replacing TCP/CORS vulnerabilities. |
| **Desktop Portals** | **`ashpd`** | Async client for `xdg-desktop-portal` (Wayland `GlobalShortcuts` for HUD toggle). |
| **Database** | **SQLite via `rusqlite` (bundled)** | Zero-admin, ACID-compliant local database with WAL mode, `PRAGMA foreign_keys = ON`, and 6 embedded migrations (`0001`–`0006`). |
| **Logging & Diagnostics** | **`tracing` + `tracing-subscriber`** | Structured env-filtered diagnostics (disk rotation appender at `$XDG_STATE_HOME/starship-control/` pending). |

---

## 3. System Topology

```
┌────────────────────────────────────────────────────────────────────────┐
│                        STARSHIP CONTROL TOPOLOGY                       │
└────────────────────────────────────────────────────────────────────────┘

 [ USER INTERFACE SURFACES (Tauri 2 Webview) ]
   ┌────────────────────────────────┐    ┌───────────────────────────────┐
   │         COCKPIT WINDOW         │    │         FLOATING HUD          │
   │  Mission Stream · Model Mgr    │    │  Quick Prompt · Live Progress │
   │  Topology Drawer · Audit Logs  │    │  Approve / Reject Action Card │
   └───────────────┬────────────────┘    └───────────────┬───────────────┘
                   │                                     │
                   └──────────────────┬──────────────────┘
                                      │ Tauri IPC (`invoke` / `emit`)
                                      ▼
 [ STARSHIP CONTROL CORE (Rust / Tokio Runtime) ]
   ┌─────────────────────────────────────────────────────────────────────┐
   │                        STARSHIP ORCHESTRATOR                        │
   │   Task Planner · Context Synthesizer · Tiered Safety Arbiter        │
   └──────────────┬───────────────────┬───────────────────┬──────────────┘
                  │                   │                   │
                  ▼                   ▼                   ▼
   ┌──────────────────────┐ ┌──────────────────┐ ┌──────────────────────┐
   │    STARSHIP-INFER    │ │   STARSHIP-MCP   │ │     STARSHIP-DB      │
   │  Inference Switch    │ │ Client Manager   │ │ SQLite Engine (WAL)  │
   ├──────────────────────┤ ├──────────────────┤ ├──────────────────────┤
   │ • Local: llama-cpp-2 │ │ • Unix Sockets   │ │ • missions & messages│
   │ • Cloud: reqwest SSE │ │ • Stdio Spawner  │ │ • task sheets & steps│
   │ • HF Chunked Downld  │ │ • Tool Registry  │ │ • app manifests      │
   │ • SHA-256 Verifier   │ │ • SO_PEERCRED    │ │ • models & audit log │
   └──────────────────────┘ └─────────┬────────┘ └──────────────────────┘
                                      │
            ┌─────────────────────────┼─────────────────────────┐
            │ Unix Domain Sockets     │ Stdio Child Streams     │
            ▼                         ▼                         ▼
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │ EASY EDITS PRO  │       │SCREENSTUDIO PRO │       │ AUDIO LABS PRO  │
   │  (Live GUI NLE) │       │ (Live Recorder) │       │ (Voice Studio)  │
   │ starship-plugin │       │ starship-plugin │       │ starship-plugin │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
            │                         │                         │
            ▼                         ▼                         ▼
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │  NET WATCH PRO  │       │AGENTIC TUNEUP PRO       │  OTHER APPS     │
   │ (eBPF Monitor)  │       │(Package Cleaner)│       │  (Future Apps)  │
   │ starship-plugin │       │ starship-plugin │       │ starship-plugin │
   └─────────────────┘       └─────────────────┘       └─────────────────┘
```

---

## 4. Crate & Workspace Structure

Starship Control is structured as a modular Cargo workspace, isolating the agent logic, inference engines, and client protocols:

```
starship-control/
├── Cargo.toml                       # Workspace root manifest (8 members)
├── .github/workflows/               # ci.yml & nfpm release.yml
├── ARCHITECTURE.md                  # This document
├── PRD.md                           # Product requirements source of truth
├── DESIGN.md                        # Visual design tokens & window layout (LOCKED)
├── DATABASE.md                      # SQLite schema, indexes & migrations
├── API.md                           # Tauri IPC & MCP bridge contracts
├── PLAN.md                          # Implementation roadmap (Phases 0–8 summary)
├── GAP-CLOSURE-PLAN.md              # Hardening & reliability roadmap (Phases 9–16)
├── AGENTS.md                        # Agent invariants & coding rules
├── schemas/tasksheet-1.json         # Strict JSON Schema for tasksheet/1 plans
├── packaging/                       # nfpm.yaml (.deb, .rpm, .pkg.tar.zst), desktop & icons
├── app/                             # Tauri 2 Desktop Application
│   ├── package.json
│   ├── scripts/check-linear.mjs     # Anti-regression check enforcing DESIGN.md law
│   ├── src-tauri/
│   │   ├── Cargo.toml
│   │   ├── tauri.conf.json          # Multi-window config (main 1280×800 + hud 420×64)
│   │   └── src/
│   │       ├── main.rs              # Entrypoint, CLI flags (--self-test, audit verify), health loop
│   │       ├── ipc/commands.rs      # Typed Tauri commands & event emitters
│   │       ├── health_service.rs    # 1 Hz live sampler + governor loop
│   │       ├── notification.rs      # Desktop alert & overload notification dispatch
│   │       └── shortcut.rs          # Wayland GlobalShortcuts portal registration (ashpd)
│   └── src/                         # Frontend (React 19 + TypeScript + Vite)
│       ├── App.tsx                  # Window routing (Cockpit vs HUD view)
│       ├── features/
│       │   ├── cockpit/             # Shell chrome (CockpitView) & workspace views (CockpitDashboard)
│       │   └── hud/                 # Linear solid HUD surface (HudView)
│       ├── components/
│       │   ├── linear/              # Shared Linear UI kit (TitleBar, Sidebar, Breadcrumb, ECharts theme, etc.)
│       │   ├── cockpit/             # Domain views (Chat, Timeline, Missions, Workflows, Topology, Tools, Models, Systems, Audit)
│       │   ├── hud/                 # Legacy HUD barrel re-exports
│       │   └── common/              # LinearTelemetryChart, SanitizedMarkdown, DesignVerification
│       ├── stores/themeStore.ts     # Cross-window dark/light theme sync via localStorage
│       ├── types/ipc.ts             # Exact TypeScript mirrors of Rust IPC contracts
│       └── tailwind.css             # Tailwind v4 @theme Linear tokens + self-hosted fonts
└── crates/
    ├── starship-core/               # Agent orchestration, safety, admission & recovery engine
    │   └── src/
    │       ├── planner/             # TaskSheet/1 DAG builder, GBNF grammar, validator, repair loop, preview
    │       ├── executor/            # State machine, retry, timeouts, fallback, governor, admission, fair queue, leases
    │       ├── policy/              # Enforced risk tiers (tiers.toml) & resource profiles (resources.toml)
    │       ├── safety/              # SafetyArbiter, HMAC approval tokens, SecretVault/SecretScrubber
    │       ├── context/             # Two-stage tool router & token budgeter
    │       ├── pipeline/            # Screencast, record_and_edit, CET, media, diagnostics pipelines
    │       └── error.rs             # Canonical CoreError taxonomy
    ├── starship-infer/              # Model inference facade (Local + Cloud)
    │   └── src/
    │       ├── engine.rs            # InferenceEngine trait & dispatcher
    │       ├── local/               # GGUF runner facade, hardware/VRAM probe, live headroom guard
    │       ├── cloud/               # OpenAI, Anthropic, Gemini, OpenRouter SSE clients
    │       └── hf/                  # Hugging Face API search & chunked SHA-256 downloader
    ├── starship-mcp/                # MCP client runtime & hybrid transport pool
    │   └── src/
    │       ├── client.rs            # Async MCP JSON-RPC 2.0 client
    │       ├── router.rs            # Hybrid Live GUI socket vs headless stdio child router
    │       ├── registry.rs          # XDG manifest watcher & app scanner
    │       ├── security.rs          # Linux SO_PEERCRED & binary verification
    │       ├── containment.rs       # Optional systemd-run memory/CPU scope containment
    │       └── aggregator.rs        # Dynamic curated tool definition aggregator
    ├── starship-plugin/             # Shared SDK crate (imported by EEP, SSP, ALP, NWP, ATP)
    │   └── src/
    │       ├── lib.rs & server.rs   # StarshipBridge UDS/stdio MCP server & manifest registration
    │       ├── checkpoint.rs        # Sub-app checkpoint protocol primitives
    │       ├── progress.rs          # Progress streaming types
    │       └── resource.rs          # Tool resource profile declarations
    ├── starship-db/                 # SQLite storage layer
    │   └── src/
    │       ├── lib.rs               # WAL connection pool & 0001–0006 migrations
    │       ├── migrations/          # 0001_initial..0006_mission_health_summary
    │       ├── plans.rs & events.rs # Task sheets, steps, event-sourced run log
    │       ├── models.rs & apps.rs  # Models, encrypted provider configs, app registry
    │       ├── health.rs & tool_stats.rs # 1m/1h rollups, alerts, EWMA/p90 ETA learner
    │       └── audit.rs             # Tamper-evident SHA-256 hash-chained audit log
    ├── starship-health/             # Live host/GPU telemetry, 15m ring buffer & hysteresis detector
    └── xtask/                       # Planner benchmark & schema/router evaluation harness
```

---

## 5. Multi-Window & Shell Architecture

Starship Control runs a single binary managing two distinct Tauri windows and a system tray icon:

### 5.1 The Cockpit Window (`main`)
* **Role:** The strategic headquarters.
* **Geometry:** `1280×800` default, `1024×640` minimum, resizable.
* **Chrome:** Solid opaque Linear window (`decorations: false`, `transparent: false`, custom `40px` `TitleBar`, collapsible `240px → 56px` `Sidebar`, `36px` `Breadcrumb`, theme toggle synced with HUD, single active navigation surface across Chat, Timeline, Missions, Workflows, Apps, Tools, Models, Systems, Activity).
* **Visibility:** Primary Tauri `main` window managed cleanly via Tauri IPC and tray state events.

### 5.2 The Floating HUD Window (`hud`)
* **Role:** The ambient, context-aware companion.
* **Geometry:** `420×64` compact solid rectangular command bar (`6px` border radius), dynamically resized via `cmd_resize_hud` when streaming responses or Tier 2/3 approval cards are active.
* **Attributes:**
  - `decorations: false`
  - `transparent: false`
  - `shadow: true`
  - `alwaysOnTop: true`
  - `skipTaskbar: true`
  - `visible: false` (shown on toggle)
* **Behavior:**
  - Drag handle (`data-tauri-drag-region`) + logo + close/toggle button + prompt bar with theme parity (`dark` / `light`).
  - Toggled globally via `Super + Shift + Space` (managed through `ashpd` `GlobalShortcuts` portal on Wayland; in-app/cockpit toggle fallback if portal unavailable).
  - Expands cleanly to show streaming output or action approval buttons (`Approve` / `Reject`) when a Tier 2/3 action is proposed.

### 5.3 System Tray Integration
* **Status Icon:** Reflects agent state (Idle, Thinking, Executing, Waiting Approval, Offline).
* **Context Menu:**
  - Show/Hide Cockpit
  - Toggle Floating HUD
  - Active Model Selector (quick switch between downloaded local models and cloud providers)
  - Abort Current Task (emergency stop)
  - Quit Starship Control

---

## 6. App Discovery & Hybrid Transport Protocol

Starship Control establishes communication with sub-applications using a standardized two-tier discovery and connection model:

### 6.1 Discovery Phase (Manifest-Driven)
When a sub-app (such as EEP) is installed, its `starship-plugin` writes a standardized JSON manifest to:
`$XDG_DATA_HOME/starship/plugins/<app-id>.json`

```json
{
  "schemaVersion": "1.0",
  "appId": "eep",
  "name": "Easy Edits Pro",
  "version": "0.1.2-alpha.7",
  "binaryPath": "/usr/bin/easy-edits-pro",
  "launchArgs": [],
  "mcpCliArgs": ["--mcp"],
  "socketPath": "/run/user/1000/starship/bridges/eep.sock",
  "capabilities": {
    "videoEditing": true,
    "audioMastering": true,
    "timelineControl": true
  }
}
```

Starship Control uses `notify` to watch `$XDG_DATA_HOME/starship/plugins/`, automatically populating its App Topology registry when new applications are installed.

### 6.2 Connection Phase (Hybrid Routing)

```
                       Check for Active Socket:
              /run/user/<UID>/starship/bridges/<app-id>.sock
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
             [ SOCKET EXISTS ]             [ SOCKET NOT FOUND ]
                    │                               │
       Connect over Unix Domain Socket       Task requires GUI?
       Verify Linux SO_PEERCRED (UID)               │
                    │                       ┌───────┴───────┐
                    │                       ▼               ▼
                    │                    [ YES ]          [ NO ]
                    │                       │               │
                    │               Launch App GUI     Spawn CLI:
                    │               via .desktop/exec   <bin> --mcp
                    │                       │               │
                    │                       ▼               ▼
                    └──────────────► [ EXECUTE MCP COMMANDS ]
```

1. **Active GUI Path:** If the socket exists, Starship Control connects directly. The kernel checks `SO_PEERCRED` to ensure the socket owner matches the current user. Communication runs over a local, zero-copy bidirectional JSON-RPC 2.0 stream.
2. **Headless CLI Path:** If the socket does not exist and the user’s instruction is a background batch operation (e.g. "Clean this audio track with ALP"), Starship Control spawns `<binaryPath> --mcp` as a child process and communicates over standard input/output pipes.
3. **App Launch Path:** If the socket does not exist and the task requires user visibility (e.g., "Open this project in EEP and show me the edit"), Starship Control launches the GUI application via `gtk-launch` or direct execution, waiting for its bridge socket to initialize before sending commands.

---

## 7. Model Inference & Hugging Face Engine (`starship-infer`)

`starship-infer` acts as a unified abstraction layer over heterogeneous AI backends:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        STARSHIP INFER INTERFACE                        │
│             pub trait InferenceEngine: Send + Sync {                   │
│                 async fn stream_chat(&self, req: ChatRequest)          │
│                     -> Result<Pin<Box<dyn Stream<Item = ChatChunk>>>>; │
│             }                                                          │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
           ┌────────────────────────┴────────────────────────┐
           ▼                                                 ▼
┌───────────────────────────────┐ ┌──────────────────────────────────────┐
│         LOCAL ENGINE          │ │             CLOUD ENGINE             │
│ (llama-server sidecar / HTTP) │ │      (reqwest SSE HTTP Client)       │
├───────────────────────────────┤ ├──────────────────────────────────────┤
│ • GGUF format (Q4_K_M, etc.)  │ │ • OpenAI API format                  │
│ • Vulkan / ROCm / CPU backend │ │ • Anthropic Claude format            │
│ • VRAM allocation detection   │ │ • Google Gemini format               │
│ • Context caching             │ │ • OpenRouter / local llama-server      │
└───────────────────────────────┘ └──────────────────────────────────────┘
```

### 7.1 Hugging Face Model Downloader
* **Location:** Stores weights in `$XDG_DATA_HOME/starship-control/models/`.
* **Download Engine:** Multi-part chunked downloader with `Range` header support to resume interrupted transfers.
* **Integrity Gate:** Computes SHA-256 hash concurrently during download; rejects and removes corrupted weights before registering.
* **Hardware Sizing Probe:** Inspects system RAM (`/proc/meminfo`) and GPU VRAM before loading; warns users if a model exceeds available capacity.

---

## 8. Tiered Safety & Execution Policy

To prevent agent hallucinations from damaging user projects or files, all tool calls pass through the **Safety Arbiter**:

```
                       Tool Call Proposed by LLM
                                   │
                                   ▼
                       Inspect Tool Risk Tier
                                   │
            ┌──────────────────────┼──────────────────────┐
            ▼                      ▼                      ▼
    [ TIER 1: SAFE ]      [ TIER 2: MUTATING ]   [ TIER 3: DESTRUCTIVE ]
    Read-only actions     Modifies project,      Deletes files,
    (status, scopes,      applies effects,       purges caches,
    telemetry, metadata)  creates new assets     replaces takes
            │                      │                      │
            │                      └──────────┬───────────┘
            │                                 │
            │                      Generate Visual Diff &
            │                      Action Card (Cockpit/HUD)
            │                                 │
            │                                 ▼
            │                     Wait for Human Signature:
            │                     [ APPROVE ]  or  [ REJECT ]
            │                                 │
            │                      ┌──────────┴──────────┐
            │                      ▼                     ▼
            │                 [ APPROVED ]          [ REJECTED ]
            │                      │                     │
            ▼                      ▼                     ▼
       Execute Tool           Execute Tool          Abort Step &
       Immediately            Immediately           Notify Agent
```

* **Signed Action Tokens:** When a user clicks **Approve**, the UI signs an ephemeral action token validated by the Rust core before dispatching the MCP call.
* **Undo & Checkpoints:** When targeting EEP or ALP, Starship Control instructs the sub-app to create a versioned checkpoint or project branch before executing mutating operations.

---

## 9. Security & Sandboxing Boundaries

1. **Local Socket Authentication:** All Unix domain sockets are placed under `$XDG_RUNTIME_DIR/starship/bridges/` (permissions `0700`). Connections are validated with `libc::getsockopt(..., SO_PEERCRED, ...)` to guarantee caller UID matches the host user.
2. **API Key Isolation:** Cloud API keys are stored in the SQLite database with optional protection via Linux Secret Service API (`libsecret`). Keys are strictly scrubbed from all terminal outputs, UI logs, and LLM context payloads.
3. **Prompt Injection Defense:** External strings returned from tool calls (file names, window titles, CET telemetry strings, network endpoints) are encapsulated inside designated `<untrusted_context>` blocks in prompt templates. The agent's system prompt strictly prohibits treating external strings as instructions.

---

## 10. File System & XDG Path Specification

Starship Control adheres strictly to the Linux XDG Base Directory specification:

| Directory | Standard Path | Content |
|---|---|---|
| **Configuration** | `$XDG_CONFIG_HOME/starship-control/` (`~/.config/starship-control/`) | `config.toml` (window geometry, active model preference, shortcuts). |
| **Data & Storage** | `$XDG_DATA_HOME/starship-control/` (`~/.local/share/starship-control/`) | `library.db` (SQLite primary store), `models/` (downloaded GGUF weights). |
| **Runtime Bridges** | `$XDG_RUNTIME_DIR/starship/` (`/run/user/<UID>/starship/`) | Active Unix domain bridge sockets (`bridges/*.sock`). |
| **Plugins Manifests**| `$XDG_DATA_HOME/starship/plugins/` (`~/.local/share/starship/plugins/`) | JSON capability manifests dropped by installed sub-apps. |
| **State & Logs** | `$XDG_STATE_HOME/starship-control/` (`~/.local/state/starship-control/`) | Rotating execution logs (`starship-control.log`). |
