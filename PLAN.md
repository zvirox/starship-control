# PLAN.md — Starship Control Implementation Roadmap

**Version:** 1.4 · **Date:** 2026-11-27
**Authority chain:** `PRD.md` (scope) → `ARCHITECTURE.md` (structure) → `DESIGN.md` (locked UI law) → `DATABASE.md` (schema) → `API.md` (contracts) → **this file (Phases 0–8)** → [**`GAP-CLOSURE-PLAN.md`**](GAP-CLOSURE-PLAN.md) (Phases 9–16) → [**`PHASE-17-FIX-PLAN.md`**](PHASE-17-FIX-PLAN.md) (Phases 17A–17F)
**Working rule:** Every phase ends with a **runnable application** (vertical slices). **UI Visual Spec Status:** The Linear solid design system (`DESIGN.md` v2.1, implemented on `main`) is **FINAL & LOCKED** — no further UI style planning or visual redesigns remain. The rulebook for executing any task is [`AGENTS.md`](AGENTS.md).
**Release policy:** No release timeline. Publishing a release (`v*` tag + packages) is optional and at the maintainer's sole discretion — no phase or checklist item sets a date or version; the maintainer decides when and which release to push.

---

## Phase 0 — Workspace Foundation & Multi-Window Shell

> **Goal:** Complete Cargo workspace scaffolding, SQLite WAL migration runner, and a themed Tauri 2 shell opening both the Cockpit and Floating HUD windows with working design tokens.

- [x] **0.1 Scaffold Cargo Workspace & Crate Skeletons**
  - Setup root `Cargo.toml` workspace containing `app`, `crates/starship-core`, `crates/starship-infer`, `crates/starship-mcp`, `crates/starship-plugin`, and `crates/starship-db`.
  - Configure workspace lints (`deny(unsafe_code)`, `warn(clippy::pedantic)`, Rust 2024 edition).
  - *Verify:* `cargo check --workspace` compiles cleanly.
- [x] **0.2 Tauri 2 Multi-Window Scaffolding**
  - Initialize Tauri 2 with React 19, TypeScript, and Vite inside `app/`.
  - Configure `tauri.conf.json` with two solid opaque windows: `main` (Cockpit, `1280×800`, min `1024×640`, `decorations: false`, `transparent: false`) and `hud` (Floating HUD, `420×64`, borderless, `transparent: false`, always-on-top, hidden by default).
  - *Verify:* Multi-window Tauri configuration compiles and both surfaces render in sync.
- [x] **0.3 Tailwind CSS v4 `@theme` & Locked Linear Token Bridging**
  - Bundle self-hosted `InterVariable.woff2` and `JetBrainsMono-Variable.woff2` via `@font-face` (`app/public/fonts/`).
  - Author `tailwind.css` using locked `DESIGN.md` v2.1 Linear solid surface tokens (`#050505`/`#111213`/`#1A1B1E` dark, `#F3F4F6`/`#FFFFFF`/`#F4F5F7` light, Violet per-app primary accent in `app-accent.css` plus the shared blue token (`--blue`: `#2563EB`/`#346AFE`), zero blur), enforced by `npm run check:linear`.
  - *Verify:* `npm run check:linear` and `DesignVerification.tsx` validate zero blur/translucent regressions across dark and light modes.
- [x] **0.4 SQLite Engine & Migration Runner (`starship-db`)**
  - Setup `starship-db` with `rusqlite` bundled, enabling WAL mode, `PRAGMA foreign_keys = ON`, busy timeout 5000ms, and `0600` permissions at `$XDG_DATA_HOME/starship-control/library.db`.
  - Implement `PRAGMA user_version` runner with migrations `0001_initial.sql`–`0006_mission_health_summary.sql`.
  - *Verify:* Fresh boot creates `library.db` with correct tables; integration tests (`migration_test.rs`) validate CRUD, cascades, and hash chains.
- [ ] **0.5 Typed IPC Skeleton & Error Format** *(Partial — Logging Pending)*
  - [x] Implement standard `CoreError` envelope (`kind`, `code`, `retryable`, `userMessage`, `details`) in Rust and TypeScript (`app/src/types/ipc.ts`), plus `ipc_hello` and `--self-test`.
  - [ ] Setup rotating structured logging on disk (`tracing-appender`) at `$XDG_STATE_HOME/starship-control/` (currently `tracing_subscriber::fmt()` logs to stdout only).
  - *Verify:* Frontend receives typed errors matching `API.md §1`; file log appender still pending.

**Definition of Done (Phase 0):** ✅ Multi-window Tauri 2 shell launches, renders design tokens (measured on KDE Plasma 6 Wayland; X11 and other compositors unverified — see `docs/COMPAT.md`), passes CI gates (clippy `-D warnings`, `tsc --noEmit`), and seeds the SQLite WAL database.

---

## Phase 1 — Dual Inference Engine (Local GGUF & Cloud APIs)

> **Goal:** Real-time token streaming in Cockpit from both local GGUF models (`llama-cpp-2`) and cloud streaming endpoints (`reqwest` SSE).

- [x] **1.1 Unified `InferenceEngine` Trait (`starship-infer`)**
  - Define `InferenceEngine` async trait returning a pinned `Stream<Item = Result<ChatChunk, CoreError>>`.
  - Implement unified `ChatRequest`, `Message`, and `ToolDefinition` abstractions.
  - *Verify:* Mock engine unit test streams token chunks deterministically.
- [x] **1.2 Cloud SSE Client Implementation**
  - Implemented OpenAI-compatible SSE client (`OpenAiClient`, supporting OpenAI & OpenRouter), Anthropic Claude SSE client (`AnthropicClient`), and Google Gemini streaming client (`GeminiClient`).
  - Wired `SecretVault` (`enc_v1:<hex>` encryption at rest + scrubbing) in `provider_configs` SQLite storage (`starship-db/src/models.rs`, verified in `docs/evidence/10.7.txt`). *(Optional Linux Secret Service daemon integration deferred).*
  - *Verify:* Cloud SSE dispatchers compile and secret-vault roundtrip & canary-scrubbing tests pass.
- [ ] **1.3 Local GGUF Engine Integration (`llama-cpp-2`)** *(Partial — Native Bindings Pending)*
  - [x] Hardware detection (`HardwareCapabilities`), VRAM capacity probes, live `LocalInferenceGuard` headroom checks, and contextual/GBNF tasksheet response generation in `LocalGgufRunner`.
  - [ ] Wire native `llama-cpp-2` C++ GGUF bindings into `starship-infer/Cargo.toml` and `runner.rs` (currently `LocalGgufRunner` synthesizes responses without executing real GGUF weights).
  - *Verify:* Guard & runner tests pass; real on-device weight sampling remains pending.
- [x] **1.4 Streaming Token & Thought UI**
  - Author Cockpit chat message canvas with real-time token rendering, code syntax highlighting, and expandable `<thought>` blocks.
  - Emit `evt://token/stream` and `evt://thought/stream` from Rust core to webview.
  - *Verify:* Visual verification of streamed responses in Cockpit with latency timer (`mono-sm`).

**Definition of Done (Phase 1):** ✅ Cloud providers stream markdown and reasoning traces in Cockpit; local GGUF is a hardware/VRAM-guarded facade until 1.3 wires native `llama-cpp-2` weights.

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

**Definition of Done (Phase 2):** ✅ User can search Hugging Face in Cockpit, download a GGUF model with progress indicators and SHA-256 validation, and register it for selection; real offline weight sampling depends on 1.3.

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

- [x] **4.1 Async MCP JSON-RPC 2.0 Client (`starship-mcp`)**
  - Implement MCP client protocol supporting `initialize`, `tools/list`, `tools/call`, and `tools/cancel`.
  - Implement handshake negotiation (`starship/hello`) exchanging version and risk-tier metadata.
  - *Verify:* Automated unit test executes mock tool calls with round-trip latency < 1ms.
- [x] **4.2 Hybrid Execution Router (GUI Socket vs. Stdio Spawner)**
  - Implement router: if active socket exists, dispatch over Unix socket; if absent and headless-compatible, spawn `<binaryPath> --mcp` and communicate over child stdin/stdout.
  - Implement process supervisor handling child process crashes, timeouts, and cleanup.
  - *Verify:* Headless command executes tool via stdio without opening a desktop GUI window.
- [x] **4.3 Tool Definition Aggregator & Context Injector**
  - Dynamically query tools from all connected sub-apps and format them into standard LLM tool schemas.
  - Cache tool schemas in memory, invalidating when an app disconnects or updates.
  - *Verify:* LLM receives updated function-calling definitions matching currently available apps.

**Definition of Done (Phase 4):** ✅ Starship Control dynamically discovers available tools from running or installed apps and executes tool calls over Unix sockets or stdio.

---

## Phase 5 — Task Sheet Planner & Tiered Safety Arbiter

> **Goal:** Natural language decomposition into `tasksheet/1` DAG plans, with automatic execution for Safe tools and approval gates for Mutating tools.

- [x] **5.1 Task Sheet Planner (`starship-core`)**
  - Implement planning prompt template generating structured `tasksheet/1` JSON.
  - Parse plans into versioned database records (`plans` and `plan_steps` tables).
  - Build dependency DAG resolving step execution order.
  - *Verify:* "Record screen and clean audio" decomposes into distinct sequential steps for SSP and ALP.
- [x] **5.2 Tiered Safety Policy Engine**
  - Categorize tool calls into Tier 1 (Safe/Read-only), Tier 2 (Mutating), and Tier 3 (Destructive).
  - Tier 1 steps execute immediately and stream results back to the agent.
  - Tier 2/3 steps pause execution state machine, generate a diff summary, and wait for a signed UI token.
  - *Verify:* An unapproved Tier 2 mutation is blocked at the Rust core level, returning `safety.permission_denied`.
- [x] **5.3 Cockpit Action Approval Card & Diff Viewer**
  - Render approval cards with pulsating Rose border (`signal.gate`), impact summaries, and diff inspector.
  - Implement Approve and Reject actions with keyboard bindings (`Enter` / `Esc`).
  - *Verify:* Clicking Approve signs token, resumes execution, and records signed audit log entry in SQLite.

**Definition of Done (Phase 5):** ✅ High-stakes actions cannot execute without explicit human consent; safe inspection queries run autonomously without interruptions.

---

## Phase 6 — Floating Companion HUD & Global Shortcut

> **Goal:** Compact, solid always-on-top Linear command bar toggleable via `Super + Shift + Space` over active desktop applications.

- [x] **6.1 Floating HUD Geometry & Window Behavior**
  - Configure Tauri HUD window: borderless (`decorations: false`), solid opaque (`transparent: false`), `alwaysOnTop: true`, `skipTaskbar: true`, synced with Cockpit theme.
  - Implement resting rectangular bar geometry (`420×64`, `6px` radius) with drag handle, logo, close control, and corner snapping.
  - *Verify:* HUD window compiles and renders cleanly without compositor transparency bugs on Linux Light/Dark themes.
- [ ] **6.2 Wayland Global Shortcut Portal (`ashpd`)** *(Partial — Native X11 Fallback Pending)*
  - [x] Register `Super + Shift + Space` via `ashpd` `GlobalShortcuts` portal on Wayland with graceful fallback notice (`app/src-tauri/src/shortcut.rs`).
  - [ ] Native X11 global hotkey fallback (`XGrabKey`) not yet implemented (X11 relies on Cockpit/in-app HUD toggle).
  - *Verify:* Wayland portal session binding compiles and activates `cmd_toggle_hud`; X11 global hook remains pending.
- [x] **6.3 HUD Prompting & Streamlined Approval Expansion**
  - Build HUD input bar with active model chip, text prompt field, health dot indicator, and send button.
  - Implement dynamic vertical expansion (`cmd_resize_hud`) when streaming output or Tier 2/3 approval gates trigger.
  - *Verify:* User can submit prompts and approve/reject steps in place on the Floating HUD.

**Definition of Done (Phase 6):** ✅ On Wayland, the user can summon the Floating HUD over EEP or ALP via the XDG global shortcut (X11 uses the in-app toggle until 6.2), prompt the agent, and sign action approvals in place.

---

## Phase 7 — Cross-App Pipeline & End-to-End Workflows

> **Goal:** Validate complete multi-app creative pipelines across ScreenStudio Pro, Audio Labs Pro, and Easy Edits Pro.

- [x] **7.1 CET Telemetry & Cut Assembly Pipeline**
  - Implement orchestration pipeline: read `.cet.jsonl` sidecar from SSP recording $\to$ detect silences and clicks $\to$ assemble edit markers $\to$ dispatch cut commands to EEP.
  - *Verify:* Full automated screencast edit applied to EEP timeline via Starship Control.
- [x] **7.2 Audio Enhancement & Voice Synthesis Pipeline**
  - Route extracted audio track to ALP for noise removal and voice mastering; synthesize intro narration via ALP TTS and insert into EEP timeline.
  - *Verify:* EEP project updates with enhanced audio and aligned voiceover track.
- [x] **7.3 Systems Health & Diagnostic Pipeline (NWP + ATP)**
  - Query network activity from NWP and disk storage metrics from ATP; generate cleanup task sheet upon approval.
  - *Verify:* Unused package caches cleaned upon single-click approval.

**Definition of Done (Phase 7):** ✅ Template/no-LLM pipelines execute end-to-end against the mock harness with complete audit logs; real-app steps are MOCK-ONLY until the companion apps expose MCP (16.3).

---

## Phase 8 — Distribution & Packaging

> **Goal:** Distribution packaging for Arch Linux, Debian/Ubuntu, and Fedora/RHEL, plus GitHub Actions CI/CD release automation.

- [x] **8.1 Packaging Metadata & App Icons**
  - Setup desktop entry file `packaging/starship-control.desktop` and hicolor PNG icons (`32px`–`512px`).
  - Author unified `packaging/nfpm.yaml` + maintainer scripts (`postinstall.sh`, `preremove.sh`) targeting `.deb`, `.rpm`, and Arch `.pkg.tar.zst` (replacing legacy `packaging/aur/` & `.deb/.rpm` manual scripts).
- [x] **8.2 GitHub Actions CI/CD Pipeline**
  - Author `.github/workflows/ci.yml` (rustfmt, clippy `-D warnings`, workspace tests, `tsc --noEmit`, `npm run build`, `--self-test` smoke check, coverage baseline, audit).
  - Author `.github/workflows/release.yml` calling reusable Linux `nfpm` release workflow (`zvirox/release-workflows`) on `v*` tags.
  - **Locked Packaging Policy:** `.deb`, `.rpm`, and `.pkg.tar.zst` (`nfpm`) plus `SHA256SUMS` (+ `.sig`) are the final, locked release formats; legacy AppImage, Flatpak, tarballs, AUR, and VM package test requirements are retired. Package install/uninstall is verified via CI clean containers (`GAP-CLOSURE-PLAN.md §16.6`).

**Definition of Done (Phase 8):** ✅ Builds clean on CI and produces the locked `.deb`, `.rpm`, `.pkg.tar.zst` packages with `SHA256SUMS`/`.sig`; clean-container install verification is 16.6 (pending). Publishing a release is optional and at the maintainer's discretion.

---

## Phase 9–16 — Gap Filling, Missing Features & System Hardening

> The roadmap continues in detail in [**`GAP-CLOSURE-PLAN.md`**](GAP-CLOSURE-PLAN.md).  
> Governed by `PRD.md` → `ARCHITECTURE.md` → `DESIGN.md` → `DATABASE.md` → `API.md` → `PLAN.md` → **`GAP-CLOSURE-PLAN.md`**.

### Summary of Phases 9–16:
- [x] **Phase 9 — Truth, CI & Foundations**: Ground-truth audit (`docs/evidence/AUDIT.md`), CI matrix (`.github/workflows/ci.yml`), SQLite honesty notice, workspace warnings & `deny(unsafe_code)` audit (`docs/evidence/9.0, 9.1, 9.3.txt`). Evidence files `9.1`–`9.5` refreshed 2026-11-27; container smoke proof tracked in `16.6`.
- [x] **Phase 10 — Hardened Control Plane**: Enforced safety arbiter (`tiers.toml`), HMAC-SHA256 argument-bound approval tokens, `SO_PEERCRED` + binary path checks, `SecretVault` encryption at rest, tamper-evident SHA-256 audit chain with `starship-control audit verify` (`docs/evidence/10.1–10.8.txt`).
- [x] **Phase 11 — Executor Engine, Checkpoints & Fallback Plan**: Execution state machine, jittered retry, timeouts, dynamic fallback planner (`fallbacks.toml`), cross-app checkpoint primitives, lease supervisor, and event-sourced run log (`docs/evidence/11.1–11.11.txt`).
- [x] **Phase 12 — Planner Reliability & Tool Routing**: Two-stage lexical tool router, strict `tasksheet/1` schema + GBNF validator with repair loop, prompt injection defense (`<untrusted_context>`), dry-run preview & `xtask` planner eval (`docs/evidence/12.1–12.7.txt`).
- [x] **Phase 13 — Progress Tracking**: Mission progress state, EWMA/p90 ETA learner (`tool_stats`), crash reconstruction, and Cockpit Timeline / HUD progress views (`docs/evidence/13.1–13.8.txt`).
- [x] **Phase 14 — System Health & Overload Monitor + Dashboard**: `starship-health` samplers (CPU, RAM, swap, PSI, disk, thermal, GPU), 15m ring buffer + SQLite rollups (`0005`–`0006`), hysteresis detector, overload governor, and Cockpit Systems view finalized in locked Linear UI (`docs/evidence/14.1–14.11.txt`).
- [x] **Phase 15 — Resource-Aware Scheduling (Admission Control)**: Tool resource profiles (`resources.toml`), admission controller, local-inference live headroom guard, optional `systemd-run` containment, and fair mission queueing (`docs/evidence/15.1–15.5.txt`).
- [ ] **Phase 16 — Test Harness, Chaos & Release Tooling** *(Complete except `16.6` — `16.1`–`16.5`, `16.7`, `16.8` done)*: `starship-mock-app` crate (`16.1`, `docs/evidence/16.1.txt`), chaos fault-injection suite (`16.2`, `docs/evidence/16.2.txt`), real-app E2E runs (`16.3`, `docs/evidence/16.3.txt`), security mutation CI suite (`16.4`, `docs/evidence/16.4.txt`), compositor `COMPAT.md` matrix (`16.5`, `docs/evidence/16.5.txt`), locked-format CI container packaging & signing verification (`16.6`), `THREAT_MODEL.md` (`16.7`, `docs/evidence/16.7.txt`), and performance budgets (`16.8`, `docs/evidence/16.8.txt`).

## Phase 17 — No Fakes, Real Engines & Diagnostics

> Authoritative breakdown: [**`PHASE-17-FIX-PLAN.md`**](PHASE-17-FIX-PLAN.md) (17A–17F, audited at `8be383e`), including its Manual Test Checklist T1–T12 (§8) and DoD (§9).

- [ ] **Phase 17 — Truth in every build**: remove simulated tool/app/model data (17A), real local + cloud chat (17B), cloud model discovery & picker (17C), real app discovery (17D), wired mission runner (17E), diagnostics/engine console (17F). Release timing remains the maintainer's call, not a milestone.

*Refer to [**`GAP-CLOSURE-PLAN.md`**](GAP-CLOSURE-PLAN.md) for Phases 9–16 and [**`PHASE-17-FIX-PLAN.md`**](PHASE-17-FIX-PLAN.md) for Phase 17 task criteria and non-negotiable verification commands.*

