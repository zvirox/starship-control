# ARCHITECTURE.md — Starship Control

**Version:** 1.0 · **Date:** 2026-11-17  
**Authority chain:** `PRD.md` (scope, requirements) → `ARCHITECTURE.md` (system structure, contracts) → `DESIGN.md` (UI law)  
**Rule:** Defines technical structure, crate layout, protocols, process boundaries, and lifecycle contracts. No product code is written as part of this document.

---

## 1. Architecture Principles

1. **Local-First & Sovereign by Default:** Starship Control operates fully on-device without phoning home. Local GGUF models run directly against the hardware (Vulkan/ROCm/CPU) via native Rust bindings. Cloud APIs are strictly optional, user-configured, and directly addressed with the user's private keys.
2. **Two Surfaces, One Core:** The system runs a single core process hosting the agent orchestration runtime, SQLite database, and MCP client manager. It drives two distinct UI surfaces via Tauri 2:
   - **The Cockpit:** Primary window for strategic mission planning, model downloading, app topology inspection, and detailed audit review.
   - **The Floating HUD:** Always-on-top, draggable, translucent companion strip that sits over other applications for quick prompts, live progress, and instant approvals.
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
| **App Shell** | **Tauri 2** | Multi-window support (Cockpit + transparent floating HUD), system tray, global shortcut portal, tiny memory footprint, native Linux Wayland integration. |
| **Frontend Framework** | **React 19 + TypeScript + Vite** | High-performance reactive UI, strict end-to-end typing matching the Rust IPC layer, fast Vite HMR loop. |
| **Styling & Design** | **Tailwind CSS v4** | Pure CSS `@theme` tokens matching DESIGN.md v1.0 (shared across EEP and SSP). |
| **UI Components** | **Radix UI Primitives + Lucide Icons** | Unstyled, fully accessible keyboard-navigable primitives customized to frosted-glass tokens. |
| **Animation & Motion** | **framer-motion** | Physics-based spring animations for HUD expansion, card transitions, and progress indicators. |
| **Async Runtime (Core)** | **Rust 2024 + Tokio** | High-throughput asynchronous runtime managing multiple concurrent MCP transports, streaming SSE connections, and child processes. |
| **Local LLM Engine** | **`llama-cpp-2` / GGUF engine** | Native, high-performance C++ bindings for GGUF quantization execution with Vulkan, ROCm, and AVX2 hardware acceleration. |
| **Cloud Model Client** | **`reqwest` (HTTP/2, rustls)** | Streaming Server-Sent Events (SSE) client for OpenAI, Anthropic, Gemini, and OpenRouter endpoints. |
| **Protocol / Tooling** | **Model Context Protocol (MCP)** | Official protocol for discovering tools, schemas, resources, and executing actions across sub-apps. |
| **IPC Transport (Sub-Apps)** | **Linux UNIX Domain Sockets (`SO_PEERCRED`)** | High-speed local IPC verified by kernel UID checks, replacing TCP/CORS vulnerabilities. |
| **Desktop Portals** | **`ashpd`** | Async client for `xdg-desktop-portal` (GlobalShortcuts for HUD toggle, system notifications). |
| **Database** | **SQLite via `rusqlite` (bundled)** | Zero-admin, ACID-compliant local database with WAL mode and `PRAGMA foreign_keys = ON`. |
| **Logging & Diagnostics** | **`tracing` + `tracing-appender`** | Structured diagnostics with daily log rotation at `$XDG_STATE_HOME/starship-control/`. |

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
├── Cargo.toml                       # Workspace root manifest
├── ARCHITECTURE.md                  # This document
├── PRD.md                           # Product requirements source of truth
├── DESIGN.md                        # Visual design tokens & window layout
├── DATABASE.md                      # SQLite schema, indexes & queries
├── API.md                           # Tauri IPC & MCP bridge contracts
├── PLAN.md                          # Implementation roadmap
├── AGENTS.md                        # Agent invariants & coding rules
├── README.md
├── LICENSE
├── packaging/                       # Linux distro packaging (.deb, .rpm, PKGBUILD)
├── app/                             # Tauri 2 Desktop Application
│   ├── Cargo.toml
│   ├── tauri.conf.json              # Multi-window config (cockpit + hud)
│   ├── capabilities/                # Tauri least-privilege capability manifests
│   ├── src-tauri/
│   │   ├── src/
│   │   │   ├── main.rs              # Application entrypoint
│   │   │   ├── ipc/                 # Tauri commands & event emitters
│   │   │   ├── window/              # Multi-window managers (cockpit, hud, tray)
│   │   │   └── shortcuts.rs         # Global portal shortcut registration
│   ├── src/                         # Frontend (React 19 + TypeScript + Vite)
│   │   ├── App.tsx                  # Window routing (Cockpit vs HUD view)
│   │   ├── components/
│   │   │   ├── cockpit/             # Cockpit panels, model manager, topology drawer
│   │   │   ├── hud/                 # Floating HUD toolbar, approval cards, waveform
│   │   │   ├── common/              # Glass panels, buttons, inputs, modals
│   │   │   └── markdown/            # Streaming markdown & diff visualizer
│   │   ├── lib/                     # Typed IPC client, stores (Zustand), events
│   │   └── tailwind.css             # Tailwind v4 @theme tokens
├── crates/
│   ├── starship-core/               # Agent orchestration & execution engine
│   │   ├── src/
│   │   │   ├── planner/             # Task Sheet decomposition & DAG builder
│   │   │   ├── executor/            # Step execution loop & safety arbiter
│   │   │   ├── context/             # History assembler, token budgeter, system prompts
│   │   │   └── error.rs             # Unified error definitions
│   ├── starship-infer/              # Model inference facade (Local + Cloud)
│   │   ├── src/
│   │   │   ├── engine.rs            # InferenceEngine trait (stream_chat)
│   │   │   ├── local/               # llama-cpp-2 runner, thread allocator, VRAM probe
│   │   │   ├── cloud/               # OpenAI, Anthropic, Gemini, OpenRouter SSE clients
│   │   │   ├── hf/                  # Hugging Face API client & chunked file downloader
│   │   │   └── hash.rs              # Streaming SHA-256 verifier
│   ├── starship-mcp/                # MCP client runtime & transport pool
│   │   ├── src/
│   │   │   ├── client.rs            # Async MCP JSON-RPC 2.0 client
│   │   │   ├── transport/           # Unix domain socket & stdio pipe implementations
│   │   │   ├── discovery.rs         # XDG manifest watcher & socket scanner
│   │   │   ├── security.rs          # SO_PEERCRED validator & handshake verifier
│   │   │   └── registry.rs          # Live active tools & capabilities registry
│   ├── starship-plugin/             # Shared SDK crate (imported by EEP, SSP, etc.)
│   │   ├── src/
│   │   │   ├── lib.rs               # StarshipBridge server initialization
│   │   │   ├── manifest.rs          # Auto-generates $XDG_DATA_HOME plugin JSON
│   │   │   ├── listener.rs          # Unix domain socket server with SO_PEERCRED
│   │   │   └── protocol.rs          # Shared message frames & lifecycle events
│   └── starship-db/                 # SQLite storage layer
│       ├── src/
│       │   ├── lib.rs               # Connection pool & PRAGMA configuration
│       │   ├── migrations/          # Embedded SQL migration scripts
│       │   ├── missions.rs          # Mission & message store
│       │   ├── plans.rs             # Task sheet & step store
│       │   ├── models.rs            # Model registry & provider configuration store
│       │   └── audit.rs             # Immutable audit log recorder
```

---

## 5. Multi-Window & Shell Architecture

Starship Control runs a single binary managing two distinct Tauri windows and a system tray icon:

### 5.1 The Cockpit Window (`cockpit`)
* **Role:** The strategic headquarters.
* **Geometry:** 1200×800 default, 800×600 minimum, resizable.
* **Chrome:** Custom Client-Side Decoration (CSD) header bar with glass styling, model selector pill, connection status indicator, and navigation tabs (Mission, Topology, Models, Settings, Audit).
* **Visibility:** Standard window lifecycle. When closed, it hides rather than terminates the process, keeping the background agent daemon and HUD responsive.

### 5.2 The Floating HUD Window (`hud`)
* **Role:** The ambient, context-aware companion.
* **Geometry:** 520×76 fixed resting state, expanding to 520×240 when approval cards or multi-step execution plans are active.
* **Attributes:**
  - `decorations: false`
  - `transparent: true`
  - `alwaysOnTop: true`
  - `skipTaskbar: true`
* **Behavior:**
  - Snaps to top-center, bottom-center, or custom user-dragged coordinates.
  - Toggled globally via `Super + Shift + Space` (managed through `ashpd` GlobalShortcuts portal under Wayland).
  - Automatically expands to reveal action approval buttons when a Tier 2/3 action is proposed.

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
│   (llama-cpp-2 Rust Wrapper)  │ │      (reqwest SSE HTTP Client)       │
├───────────────────────────────┤ ├──────────────────────────────────────┤
│ • GGUF format (Q4_K_M, etc.)  │ │ • OpenAI API format                  │
│ • Vulkan / ROCm / CPU backend │ │ • Anthropic Claude format            │
│ • VRAM allocation detection   │ │ • Google Gemini format               │
│ • Context caching             │ │ • OpenRouter / Local OpenAI (Ollama) │
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
