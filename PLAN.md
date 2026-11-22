# PLAN.md — Starship Control Implementation Roadmap

**Version:** 1.0 · **Date:** 2026-11-17  
**Authority chain:** `PRD.md` (scope) → `ARCHITECTURE.md` (structure) → `DESIGN.md` (UI law) → `DATABASE.md` (schema) → `API.md` (contracts) → **this file (order of work)**  
**Working rule:** Every phase ends with a **runnable application** (vertical slices — `cargo tauri dev` produces real, testable behavior at each phase boundary, never "backend first, UI later"). Tasks are sized $\le$ 1–2 hours. The rulebook for executing any task is [`AGENTS.md`](AGENTS.md).

---

## Phase 0 — Workspace Foundation & Multi-Window Shell

> **Goal:** Complete Cargo workspace scaffolding, SQLite WAL migration runner, and a themed Tauri 2 shell opening both the Cockpit and Floating HUD windows with working design tokens.

- [x] **0.1 Scaffold Cargo Workspace & Crate Skeletons**
  - Setup root `Cargo.toml` workspace containing `app`, `crates/starship-core`, `crates/starship-infer`, `crates/starship-mcp`, `crates/starship-plugin`, and `crates/starship-db`.
  - Configure workspace lints (`deny(unsafe_code)`, `warn(clippy::pedantic)`, Rust 2024 edition).
  - *Verify:* `cargo check --workspace` compiles cleanly.
- [x] **0.2 Tauri 2 Multi-Window Scaffolding**
  - Initialize Tauri 2 with React 19, TypeScript, and Vite inside `app/`.
  - Configure `tauri.conf.json` with two windows: `main` (Cockpit, 1200×800) and `hud` (Floating HUD, 520×76, borderless, transparent, always-on-top, hidden by default).
  - *Verify:* `cargo tauri dev` launches the main glass window; HUD window can be programmatically displayed.
- [x] **0.3 Tailwind CSS v4 `@theme` & Token Bridging**
  - Import self-hosted fonts (`FamiljenGrotesk`, `JetBrainsMono`) via `@font-face`.
  - Author `tailwind.css` using DESIGN.md tokens (dark/light palettes, glass utility recipes `@utility pane/float/modal`, specular overlays).
  - *Verify:* Swatch verification page renders obsidian canvas, Electric Cyan accents, and frosted glass recipes.
- [x] **0.4 SQLite Engine & Migration Runner (`starship-db`)**
  - Setup `starship-db` with `rusqlite` bundled, enabling WAL mode, `PRAGMA foreign_keys = ON`, busy timeout 5000ms, and `0600` permissions at `$XDG_DATA_HOME/starship-control/library.db`.
  - Implement `PRAGMA user_version` runner with initial schema `0001_initial.sql` (creating `missions`, `messages`, `plans`, `steps`, `models`, `app_registry`, `audit_log`).
  - *Verify:* Fresh boot creates `library.db` with correct tables; integration test validates CRUD and foreign key cascades.
- [x] **0.5 Typed IPC Skeleton & Error Format**
  - Implement standard `CoreError` envelope (`kind`, `code`, `retryable`, `userMessage`, `details`) in Rust and TypeScript.
  - Implement initial `ipc.hello` handshake command verifying protocol parity.
  - Setup rotating structured logging via `tracing-subscriber` at `$XDG_STATE_HOME/starship-control/`.
  - *Verify:* Frontend receives typed errors matching `API.md §1` without panics.

**Definition of Done (Phase 0):** ✅ Multi-window Tauri 2 shell launches, renders design tokens on Wayland/X11, passes CI gates (clippy `-D warnings`, `tsc --noEmit`), and seeds the SQLite WAL database.

---

## Phase 1 — Dual Inference Engine (Local GGUF & Cloud APIs)

> **Goal:** Real-time token streaming in Cockpit from both local GGUF models (`llama-cpp-2`) and cloud streaming endpoints (`reqwest` SSE).

- [x] **1.1 Unified `InferenceEngine` Trait (`starship-infer`)**
  - Define `InferenceEngine` async trait returning a pinned `Stream<Item = Result<ChatChunk, CoreError>>`.
  - Implement unified `ChatRequest`, `Message`, and `ToolDefinition` abstractions.
  - *Verify:* Mock engine unit test streams token chunks deterministically.
- [x] **1.2 Cloud SSE Client Implementation**
  - Implement OpenAI-compatible streaming client (supporting OpenAI, OpenRouter, and Ollama).
  - Implement Anthropic Claude SSE streaming client (`messages` API).
  - Implement Google Gemini streaming client.
  - Wire encrypted provider configuration storage in `provider_configs` table.
  - *Verify:* Live prompt against configured cloud provider streams tokens smoothly into Cockpit chat.
- [x] **1.3 Local GGUF Engine Integration (`llama-cpp-2`)**
  - Author native `llama-cpp-2` wrapper with hardware detection (Vulkan, ROCm, CPU threads).
  - Implement context caching, token sampling, and streaming token generator.
  - Implement RAM/VRAM pre-allocation probe preventing out-of-memory crashes.
  - *Verify:* Loading local GGUF file executes inference locally on device with token generation logged.
- [x] **1.4 Streaming Token & Thought UI**
  - Author Cockpit chat message canvas with real-time token rendering, code syntax highlighting, and expandable `<thought>` blocks.
  - Emit `evt://token/stream` and `evt://thought/stream` from Rust core to webview.
  - *Verify:* Visual verification of streamed responses in Cockpit with latency timer (`mono-sm`).

**Definition of Done (Phase 1):** ✅ User can switch between a local GGUF model and cloud providers in Cockpit, issue a prompt, and observe streaming markdown and reasoning traces.

---

## Phase 2 — Model Manager & Hugging Face Downloader

> **Goal:** In-app discovery, download, and validation of GGUF models directly from Hugging Face.

- [x] **2.1 Hugging Face Model Search & Metadata Parser**
  - Integrate Hugging Face Hub API to search and list GGUF repositories (filtering by quantizations: `Q4_K_M`, `Q8_0`).
  - Compute model parameter sizes and estimated VRAM requirements.
  - *Verify:* Query returns available GGUF files with exact byte counts.
- [x] **2.2 Resumable Chunked File Downloader**
  - Implement multi-part chunked downloader in `starship-infer` with `Range` header support for paused/interrupted downloads.
  - Stream progress telemetry at 4 Hz (`evt://download/progress`: downloaded bytes, speed MB/s, ETA).
  - *Verify:* Force-killing download and restarting resumes without redownloading existing bytes.
- [x] **2.3 Streaming SHA-256 Checksum Verification**
  - Pipe downloaded chunks through streaming SHA-256 hasher; reject corrupted downloads before registering.
  - Register verified files into SQLite `models` table.
  - *Verify:* Checksum mismatch immediately purges corrupt file and emits typed `infer.checksum_mismatch`.
- [x] **2.4 Model Control UI & Hardware Compatibility Gauge**
  - Build Cockpit **Model Control** panel: Hugging Face search input, model cards with quant badges, download progress bars, and VRAM fit badges (`Fits in VRAM`, `Partial Offload`, `CPU Only`).
  - *Verify:* One-click download from Hugging Face registers the model and makes it immediately selectable.

**Definition of Done (Phase 2):** ✅ User can search Hugging Face in Cockpit, download a GGUF model with progress indicators and SHA-256 validation, and immediately use it for offline inference.

---

## Phase 3 — App Discovery & `starship-plugin` SDK

> **Goal:** Zero-configuration discovery of EEP, SSP, ALP, NWP, and ATP via XDG manifests and Unix domain sockets.

- [x] **3.1 Shared `starship-plugin` Crate Authoring**
  - Author reusable Rust crate `crates/starship-plugin` with zero third-party dependencies beyond standard library and `serde`.
  - Implement manifest generator writing to `$XDG_DATA_HOME/starship/plugins/<app-id>.json` on app boot.
  - Implement Unix domain socket listener at `$XDG_RUNTIME_DIR/starship/bridges/<app-id>.sock`.
  - *Verify:* Standalone test binary importing `starship-plugin` registers manifest and opens socket cleanly.
- [x] **3.2 XDG Manifest Watcher & Registry Scanner**
  - Implement file-watcher in `starship-mcp` monitoring `$XDG_DATA_HOME/starship/plugins/`.
  - Populate and update `app_registry` SQLite table upon manifest additions or removals.
  - *Verify:* Dropping a mock manifest file triggers an immediate `evt://topology/app_state` event in UI.
- [x] **3.3 Unix Socket Client & Linux `SO_PEERCRED` Validation**
  - Implement asynchronous Unix domain socket client connecting to active bridge sockets.
  - Verify caller UID using `libc::getsockopt(..., SO_PEERCRED, ...)` to reject cross-user access attempts.
  - *Verify:* Sockets owned by matching UID succeed; mismatched UID or permissions fail with `mcp.so_peercred_failed`.
- [x] **3.4 Cockpit App Topology Drawer**
  - Build Right Topology Drawer in Cockpit displaying tiles for `eep`, `ssp`, `alp`, `nwp`, `atp`.
  - Show connection badge (`● Live GUI`, `◌ Headless CLI`, `✕ Offline`), latency ping, and active capabilities.
  - *Verify:* Launching a mock app updates its tile from Offline to Live GUI in real time.

**Definition of Done (Phase 3):** ✅ Sub-apps importing `starship-plugin` are automatically discovered upon installation and display live status in Cockpit's App Topology drawer.

---

## Phase 4 — MCP Client & Hybrid Control Plane

> **Goal:** Bidirectional MCP JSON-RPC 2.0 communication over live GUI Unix sockets and headless CLI stdio pipes.

- [ ] **4.1 Async MCP JSON-RPC 2.0 Client (`starship-mcp`)**
  - Implement MCP client protocol supporting `initialize`, `tools/list`, `tools/call`, and `tools/cancel`.
  - Implement handshake negotiation (`starship/hello`) exchanging version and risk-tier metadata.
  - *Verify:* Automated unit test executes mock tool calls with round-trip latency < 1ms.
- [ ] **4.2 Hybrid Execution Router (GUI Socket vs. Stdio Spawner)**
  - Implement router: if active socket exists, dispatch over Unix socket; if absent and headless-compatible, spawn `<binaryPath> --mcp` and communicate over child stdin/stdout.
  - Implement process supervisor handling child process crashes, timeouts, and cleanup.
  - *Verify:* Headless command executes tool via stdio without opening a desktop GUI window.
- [ ] **4.3 Tool Definition Aggregator & Context Injector**
  - Dynamically query tools from all connected sub-apps and format them into standard LLM tool schemas.
  - Cache tool schemas in memory, invalidating when an app disconnects or updates.
  - *Verify:* LLM receives updated function-calling definitions matching currently available apps.

**Definition of Done (Phase 4):** ✅ Starship Control dynamically discovers available tools from running or installed apps and executes tool calls over Unix sockets or stdio.

---

## Phase 5 — Task Sheet Planner & Tiered Safety Arbiter

> **Goal:** Natural language decomposition into `tasksheet/1` DAG plans, with automatic execution for Safe tools and approval gates for Mutating tools.

- [ ] **5.1 Task Sheet Planner (`starship-core`)**
  - Implement planning prompt template generating structured `tasksheet/1` JSON.
  - Parse plans into versioned database records (`plans` and `plan_steps` tables).
  - Build dependency DAG resolving step execution order.
  - *Verify:* "Record screen and clean audio" decomposes into distinct sequential steps for SSP and ALP.
- [ ] **5.2 Tiered Safety Policy Engine**
  - Categorize tool calls into Tier 1 (Safe/Read-only), Tier 2 (Mutating), and Tier 3 (Destructive).
  - Tier 1 steps execute immediately and stream results back to the agent.
  - Tier 2/3 steps pause execution state machine, generate a diff summary, and wait for a signed UI token.
  - *Verify:* An unapproved Tier 2 mutation is blocked at the Rust core level, returning `safety.permission_denied`.
- [ ] **5.3 Cockpit Action Approval Card & Diff Viewer**
  - Render approval cards with pulsating Rose border (`signal.gate`), impact summaries, and diff inspector.
  - Implement Approve and Reject actions with keyboard bindings (`Enter` / `Esc`).
  - *Verify:* Clicking Approve signs token, resumes execution, and records signed audit log entry in SQLite.

**Definition of Done (Phase 5):** ✅ High-stakes actions cannot execute without explicit human consent; safe inspection queries run autonomously without interruptions.

---

## Phase 6 — Floating Companion HUD & Global Shortcut

> **Goal:** Ambient, always-on-top glass capsule toggleable via `Super + Shift + Space` over active desktop applications.

- [ ] **6.1 Floating HUD Geometry & Window Behavior**
  - Configure Tauri HUD window: borderless, transparent, `alwaysOnTop: true`, `skipTaskbar: true`.
  - Implement resting capsule geometry (520×76) and drag handle with screen corner snapping.
  - *Verify:* HUD floats smoothly over full-screen apps without losing focus or stealing compositor inputs.
- [ ] **6.2 Wayland Global Shortcut Portal (`ashpd`)**
  - Register `Super + Shift + Space` via `ashpd` GlobalShortcuts portal on Wayland (with X11 keybinding fallback).
  - Handle portal permission dialog gracefully with educational first-run notice.
  - *Verify:* Pressing shortcut toggles HUD visibility anywhere on desktop.
- [ ] **6.3 HUD Prompting & Streamlined Approval Expansion**
  - Build HUD input bar with active model chip, text prompt field, and send button.
  - Implement spring-physics expansion (520×76 $\to$ 520×240) when a Tier 2/3 approval gate triggers.
  - *Verify:* User working in EEP can approve an edit directly on the floating HUD without switching windows.

**Definition of Done (Phase 6):** ✅ User can summon the Floating HUD over EEP or ALP via global shortcut, prompt the agent, and sign action approvals in place.

---

## Phase 7 — Cross-App Pipeline & End-to-End Workflows

> **Goal:** Validate complete multi-app creative pipelines across ScreenStudio Pro, Audio Labs Pro, and Easy Edits Pro.

- [ ] **7.1 CET Telemetry & Cut Assembly Pipeline**
  - Implement orchestration pipeline: read `.cet.jsonl` sidecar from SSP recording $\to$ detect silences and clicks $\to$ assemble edit markers $\to$ dispatch cut commands to EEP.
  - *Verify:* Full automated screencast edit applied to EEP timeline via Starship Control.
- [ ] **7.2 Audio Enhancement & Voice Synthesis Pipeline**
  - Route extracted audio track to ALP for noise removal and voice mastering; synthesize intro narration via ALP TTS and insert into EEP timeline.
  - *Verify:* EEP project updates with enhanced audio and aligned voiceover track.
- [ ] **7.3 Systems Health & Diagnostic Pipeline (NWP + ATP)**
  - Query network activity from NWP and disk storage metrics from ATP; generate cleanup task sheet upon approval.
  - *Verify:* Unused package caches cleaned upon single-click approval.

**Definition of Done (Phase 7):** ✅ End-to-end multi-app workflow executes reliably with complete audit logs and verification proof.

---

## Phase 8 — Distribution & Packaging

> **Goal:** Distribution packaging for Arch Linux, Debian/Ubuntu, and Fedora/RHEL, plus GitHub Actions CI/CD release automation.

- [ ] **8.1 Packaging Metadata & App Icons**
  - Setup desktop entry file `starship-control.desktop` and hicolor vector icons.
  - Author `packaging/aur/PKGBUILD`, `packaging/deb/`, and `packaging/rpm/` specs.
- [ ] **8.2 GitHub Actions CI/CD Pipeline**
  - Author `.github/workflows/ci.yml` (formatting, clippy `-D warnings`, unit tests, typecheck).
  - Author `.github/workflows/release.yml` (multi-arch Linux builds $\to$ GitHub Releases artifacts).
  - *Verify:* Release pipeline generates installable `.deb`, `.rpm`, and `.tar.gz` packages.

**Definition of Done (Phase 8):** ✅ Starship Control builds clean on CI and installs seamlessly across major Linux distributions.
