# AUDIT.md — Ground-Truth Codebase & Roadmap Audit

**Original Baseline Audit:** 2026-11-22 (`GAP-CLOSURE-PLAN.md §9.0`)  
**Re-Audit Date:** 2026-11-25 (`feat/linear-ui`, commit `9506d3b` + doc sync)  
**Target:** Starship Control `0.1.0-beta.1` (historical re-audit target)
**Current state:** 2026-11-27 · `0.1.0-beta.5` (commit `8be383e`) · see the live status lines in `PLAN.md` / `GAP-CLOSURE-PLAN.md` for the current phase state.

---

## 0. Current Re-Audit Summary (2026-11-25)

Since the initial `2026-11-22` audit:
- **Closed Former Partials:**
  - **`0.3` Tailwind CSS v4 `@theme` & Tokens → `VERIFIED`:** Self-hosted `InterVariable.woff2` & `JetBrainsMono-Variable.woff2` are bundled in `app/public/fonts/`, the final Linear solid design system (`DESIGN.md` v2.1) is locked across Cockpit (`1280×800`) and HUD (`420×64`, `transparent: false`), and `npm run check:linear` passes cleanly.
  - **`1.2` Cloud SSE & Secret Storage → `VERIFIED`:** OpenAI, Anthropic, Gemini, and OpenRouter SSE clients plus `SecretVault` (`enc_v1:<hex>` encryption at rest & log/prompt scrubbing) are implemented and tested (`docs/evidence/10.7.txt`).
  - **`8.2` GitHub Actions CI/CD → `VERIFIED`:** Workflows now live at `.github/workflows/ci.yml` and `.github/workflows/release.yml`, backed by `packaging/nfpm.yaml` (`.deb`, `.rpm`, `.pkg.tar.zst`).
- **Remaining Honest Partials in `PLAN.md` (Keep `[ ]`):**
  - **`0.5` Rotating Disk Log Appender:** `CoreError`, IPC parity, `--self-test`, and `audit verify` work cleanly, while `main.rs` currently logs to stdout only (`tracing-appender` daily rotation at `$XDG_STATE_HOME/starship-control/` pending).
  - **`1.3` Native `llama-cpp-2` Bindings:** Hardware/VRAM detection, `LocalInferenceGuard`, and `LocalGgufRunner` response/GBNF synthesis exist, while native `llama-cpp-2` GGUF inference bindings are not yet wired into `crates/starship-infer/Cargo.toml`.
  - **`6.2` Native X11 Global Shortcut Fallback:** Wayland `ashpd` `GlobalShortcuts` (`Super+Shift+Space`) and in-app fallback exist (`shortcut.rs`), while native X11 `XGrabKey` fallback is pending.
- **Phases 9–15:** All 52 tasks completed & verified (`docs/evidence/9.0–15.5.txt`), with Systems/Timeline/HUD/Cockpit finalized under the locked Linear UI (`DESIGN.md` v2.1).
- **Phase 16 (`16.1–16.8`):** `16.1`–`16.5`, `16.7` and `16.8` **VERIFIED** (`docs/evidence/16.1.txt`–`16.5.txt`, `16.7.txt`, `16.8.txt`); only `16.6` (CI container packaging & signing sign-off) pending; manual VM packaging tests retired. Threat model at [`THREAT_MODEL.md`](THREAT_MODEL.md). Performance budgets enforced by `cargo run -p xtask --bin perf_budget` and the warn-only `performance-budget` CI job.

---

*(Historical `2026-11-22` Task 9.0 baseline table preserved below for provenance.)*

## 1. Historical Baseline Summary (2026-11-22)

| Category | Count | Percentage |
|---|---|---|
| **VERIFIED** | 21 | 77.8% |
| **PARTIAL** | 6 | 22.2% |
| **MISSING** | 0 | 0.0% |
| **TOTAL** | 27 | 100% |

> **Rule Enforced:** In accordance with `GAP-CLOSURE-PLAN.md §0`, any task in `PLAN.md` that is not fully proven (`VERIFIED`) has been un-ticked (`[ ]`) until dedicated closure tasks (Phases 9–16) complete and record empirical verification.

---

## 2. Item-by-Item Audit (PLAN.md Phases 0–8)

### Phase 0 — Workspace Foundation & Multi-Window Shell

- **0.1 Scaffold Cargo Workspace & Crate Skeletons**  
  **Status:** `VERIFIED`  
  **Command:** `cargo check --workspace`  
  **Notes:** Cargo workspace compiles cleanly with `app/src-tauri` and all 5 crates (`starship-core`, `starship-infer`, `starship-mcp`, `starship-plugin`, `starship-db`). Workspace lints configured.

- **0.2 Tauri 2 Multi-Window Scaffolding**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build && cargo check -p starship-control`  
  **Notes:** `tauri.conf.json` defines `main` (Cockpit, 1200×800) and `hud` (Floating HUD, 520×76, borderless, transparent, alwaysOnTop: true, visible: false). React 19 + TypeScript frontend builds cleanly.

- **0.3 Tailwind CSS v4 `@theme` & Token Bridging**  
  **Status:** `PARTIAL`  
  **Command:** `cd app && npm run build`  
  **Notes:** `app/src/tailwind.css` correctly defines `@theme` design tokens and glass utility recipes (`.glass-pane`, `.glass-float`, `.glass-modal`). However, fonts are linked via Google Fonts CDN in `index.html` rather than bundled self-hosted `@font-face` files. Un-ticked in `PLAN.md`.

- **0.4 SQLite Engine & Migration Runner (`starship-db`)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-db`  
  **Notes:** 5 tests pass (2 unit, 3 integration). File permissions `0600` and `0700` parent directory enforced; initial schema `0001_initial.sql` creates all base tables with WAL mode and foreign keys enabled.

- **0.5 Typed IPC Skeleton & Error Format**  
  **Status:** `PARTIAL`  
  **Command:** `cargo test -p starship-core -- test_core_error_serialization`  
  **Notes:** Unified `CoreError` type parity exists between Rust and TypeScript (`app/src/types/ipc.ts`). `ipc_hello` command works. However, rotating structured logging on disk at `$XDG_STATE_HOME/starship-control/` is not yet configured (`main.rs` initializes stdout `fmt` only). Un-ticked in `PLAN.md`.

---

### Phase 1 — Dual Inference Engine (Local GGUF & Cloud APIs)

- **1.1 Unified `InferenceEngine` Trait (`starship-infer`)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-infer -- test_mock_inference_engine_stream`  
  **Notes:** `InferenceEngine` trait defined with async stream of `ChatChunk`. Unified `ChatRequest`, `Message`, and `ToolDefinition` abstractions verified.

- **1.2 Cloud SSE Client Implementation**  
  **Status:** `PARTIAL`  
  **Command:** `cargo check -p starship-infer`  
  **Notes:** OpenAI, Anthropic Claude, and Google Gemini SSE streaming clients implemented in `crates/starship-infer/src/cloud/`. `provider_configs` table exists. However, `api_key_encrypted` is stored in plaintext SQLite without encryption at rest or libsecret integration (Gap G2 / Task 10.7). Un-ticked in `PLAN.md`.

- **1.3 Local GGUF Engine Integration (`llama-cpp-2`)**  
  **Status:** `PARTIAL`  
  **Command:** `cargo test -p starship-infer -- test_local_runner_memory_allocation`  
  **Notes:** Hardware detection (`Vulkan`, `ROCm`, CPU threads) and VRAM capacity probes are implemented. However, `llama-cpp-2` native C++ bindings are not wired; `runner.rs` currently implements a simulated token streamer. Un-ticked in `PLAN.md`.

- **1.4 Streaming Token & Thought UI**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build`  
  **Notes:** `ChatCanvas.tsx` listens to `evt://token/stream` and `evt://thought/stream`, rendering real-time tokens, collapsible reasoning blocks, and performance indicators.

---

### Phase 2 — Model Manager & Hugging Face Downloader

- **2.1 Hugging Face Model Search & Metadata Parser**  
  **Status:** `VERIFIED`  
  **Command:** `cargo check -p starship-infer`  
  **Notes:** `crates/starship-infer/src/hf/search.rs` queries Hugging Face Hub API for GGUF repos, parses sibling files, and calculates VRAM fit (`FitsInVram`, `PartialOffload`, `CpuOnly`).

- **2.2 Resumable Chunked File Downloader**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-infer -- test_checksum_verification_success`  
  **Notes:** `crates/starship-infer/src/hf/downloader.rs` supports HTTP `Range` headers, `.part` resumption, and emits 4 Hz progress telemetry (`DownloadProgress`).

- **2.3 Streaming SHA-256 Checksum Verification**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-infer -- test_checksum_verification_success`  
  **Notes:** Streaming SHA-256 calculation verified; mismatched files are purged and emit typed `infer.checksum_mismatch`.

- **2.4 Model Control UI & Hardware Compatibility Gauge**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build`  
  **Notes:** `ModelControlView.tsx` and `ModelManager.tsx` render search, quant tags, download progress bars, and hardware fit badges.

---

### Phase 3 — App Discovery & `starship-plugin` SDK

- **3.1 Shared `starship-plugin` Crate Authoring**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-plugin`  
  **Notes:** 2 unit tests pass. Generates `$XDG_DATA_HOME/starship/plugins/<app-id>.json` manifests and opens Unix domain sockets at `$XDG_RUNTIME_DIR/starship/bridges/<app-id>.sock`.

- **3.2 XDG Manifest Watcher & Registry Scanner**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-mcp -- test_known_ecosystem_apps`  
  **Notes:** Registry scans manifest directories and syncs known sub-apps (`eep`, `ssp`, `alp`, `nwp`, `atp`) to `app_registry` table in SQLite.

- **3.3 Unix Socket Client & Linux `SO_PEERCRED` Validation**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-mcp -- test_unix_transport_client_handshake_and_ping`  
  **Notes:** Linux `SO_PEERCRED` UID inspection verified in `starship-mcp/src/client.rs`. (Binary path pinning and pidfd hardening scheduled for Phase 10).

- **3.4 Cockpit App Topology Drawer**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build`  
  **Notes:** `AppTopologyDrawer.tsx` displays real-time connection badges (`Live GUI`, `Headless CLI`, `Offline`), ping times, and active tool capabilities.

---

### Phase 4 — MCP Client & Hybrid Control Plane

- **4.1 Async MCP JSON-RPC 2.0 Client (`starship-mcp`)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-mcp -- test_mcp_session_tool_call_latency_under_1ms`  
  **Notes:** MCP protocol implementation supports `initialize`, `tools/list`, `tools/call`, and `tools/cancel` with sub-millisecond local latency.

- **4.2 Hybrid Execution Router (GUI Socket vs. Stdio Spawner)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-mcp -- test_hybrid_execution_router`  
  **Notes:** Successfully routes tool calls to active Unix domain sockets when connected, falling back to stdio child process spawning when offline.

- **4.3 Tool Definition Aggregator & Context Injector**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-mcp -- test_tool_aggregator_curated_tools`  
  **Notes:** Aggregates tools across all registered apps and formats them into LLM-compatible tool definitions.

---

### Phase 5 — Task Sheet Planner & Tiered Safety Arbiter

- **5.1 Task Sheet Planner (`starship-core`)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-core -- test_record_screen_and_clean_audio_decomposition`  
  **Notes:** Decomposes prompts into `tasksheet/1` DAG plans, resolving dependency ordering, cycle detection, and persisting to SQLite.

- **5.2 Tiered Safety Policy Engine**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-core -- test_tier_2_mutating_tool_blocked_without_approval`  
  **Notes:** Categorizes tool calls into Tier 1, 2, and 3. Tier 1 executes autonomously; Tier 2/3 strictly require human signature approval.

- **5.3 Cockpit Action Approval Card & Diff Viewer**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build`  
  **Notes:** `ActionApprovalCard.tsx` renders impact summaries, diff visualizer, keyboard shortcuts (`Enter`/`Esc`), and triggers signed approval commands.

---

### Phase 6 — Floating Companion HUD & Global Shortcut

- **6.1 Floating HUD Geometry & Window Behavior**  
  **Status:** `VERIFIED`  
  **Command:** `cargo check -p starship-control && cd app && npm run build`  
  **Notes:** HUD window configured in `tauri.conf.json` with resting 520×76 dimensions, borderless, transparent, always-on-top.

- **6.2 Wayland Global Shortcut Portal (`ashpd`)**  
  **Status:** `PARTIAL`  
  **Command:** `cargo check -p starship-control`  
  **Notes:** `ashpd::desktop::global_shortcuts` loop implemented in `app/src-tauri/src/shortcut.rs`. However, native X11 global shortcut fallback (e.g. `XGrabKey`) is absent. Un-ticked in `PLAN.md`.

- **6.3 HUD Prompting & Streamlined Approval Expansion**  
  **Status:** `VERIFIED`  
  **Command:** `cd app && npm run build`  
  **Notes:** `HudView.tsx` features model chip, prompt input, approval cards, and dynamic window expansion (76px $\leftrightarrow$ 240px).

---

### Phase 7 — Cross-App Pipeline & End-to-End Workflows

- **7.1 CET Telemetry & Cut Assembly Pipeline**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-core -- test_cet_telemetry_parsing_and_cut_detection`  
  **Notes:** Reads `.cet.jsonl` telemetry, detects audio silences and mouse clicks, and generates cut markers for EEP.

- **7.2 Audio Enhancement & Voice Synthesis Pipeline**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-core -- test_screencast_pipeline_dag_resolution`  
  **Notes:** Multi-app orchestrator routes audio to ALP for cleanup and TTS, and aligns tracks onto EEP timeline.

- **7.3 Systems Health & Diagnostic Pipeline (NWP + ATP)**  
  **Status:** `VERIFIED`  
  **Command:** `cargo test -p starship-core -- test_diagnostics_pipeline_generation`  
  **Notes:** Queries network telemetry and disk caches; generates structured approval task sheets.

---

### Phase 8 — Distribution & Packaging

- **8.1 Packaging Metadata & App Icons**  
  **Status:** `VERIFIED`  
  **Command:** `ls packaging/aur/PKGBUILD packaging/deb/control packaging/rpm/starship-control.spec`  
  **Notes:** Desktop entry, hicolor PNG icons (32px to 512px), AUR PKGBUILD, Debian control files, and RPM spec file exist.

- **8.2 GitHub Actions CI/CD Pipeline**  
  **Status:** `PARTIAL`  
  **Command:** `ls packaging/workflows/`  
  **Notes:** CI and release workflows were created under `packaging/workflows/` instead of `.github/workflows/`, and have not run on GitHub Actions. Un-ticked in `PLAN.md` (addressed in Task 9.1 & 9.2).

---

## 3. Architectural Differences vs. ARCHITECTURE.md §4

| Component | `ARCHITECTURE.md §4` Specification | Real Repository Implementation | Action Required |
|---|---|---|---|
| Workflows | `.github/workflows/ci.yml` | `packaging/workflows/ci.yml` (missing in `.github/`) | Task 9.1 moves workflows to `.github/workflows/` |
| Window Manager | `app/src-tauri/src/window/` | Handled directly via Tauri window APIs & IPC | Document pattern in `ARCHITECTURE.md` |
| Shortcuts File | `app/src-tauri/src/shortcuts.rs` | `app/src-tauri/src/shortcut.rs` | Sync filename in `ARCHITECTURE.md` |
| Frontend Store | `app/src/lib/` (Zustand stores) | React hooks + direct Tauri IPC invoke/listen | Retain lightweight React state; update doc |
| Markdown Component | `app/src/components/markdown/` | Embedded directly in `ChatCanvas.tsx` | Refactor into component or update doc |
| Core Executor | `crates/starship-core/src/executor/` | Partially under `safety/` & `pipeline/` | Fully implement in Phase 11 (`11.1–11.11`) |
| Core Context | `crates/starship-core/src/context/` | Built into `starship-mcp/src/aggregator.rs` | Align context builder in Phase 12 |
| Local Inference | `llama-cpp-2` native C++ runner | Simulated streaming runner with VRAM checks | Feature-flag or integrate in Phase 12/15 |
| Audit Verification | Immutable audit log | Standard SQLite table without hash-chaining | Implement SHA-256 chain in Task 10.8 |

---

## 4. Verification Evidence

The full output of the verification runs is saved to:
`docs/evidence/9.0.txt`
