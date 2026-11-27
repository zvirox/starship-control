# PHASE-17-FIX-PLAN.md — Make Starship Control Actually Work (No Fakes)

**Version:** 1.0 · **Date:** 2026-11-27 · **Audited commit:** `8be383e`
**Continues:** `GAP-CLOSURE-PLAN.md` (Phases 9–16). This file defines **Phase 17 (17A–17F)**.
**Audience:** a coding agent (`AGENTS.md`) executing one task at a time.

> **Honesty note.** Findings come from reading the code at `8be383e`; nothing was built or run. Every root cause below cites a file so the agent can confirm it first (Task 17.0). If the code disagrees with this plan, the code wins and the plan is amended in the same commit.
>
> **Correction to the earlier audit:** it described the local GGUF engine as real. It is not. `crates/starship-infer/src/local/runner.rs::stream_chat` returns canned, keyword-matched text, and no llama.cpp dependency exists anywhere in the workspace. Cloud clients (OpenAI-compatible, Anthropic, Gemini) are real HTTP clients.

---

## 0. Rules for Phase 17

1. **No fake data in any release build.** Simulation lives only in `starship-mock-app` or behind `#[cfg(feature = "demo")]`, which release CI forbids. If a real thing is unavailable, show an honest empty state or a typed error, never an invented success.
2. **Done means wired.** A task is complete only when (a) a test drives it **through the IPC command** (not just the library), and (b) its line in the Manual Test Checklist (§7) passes.
3. **Evidence file per task:** `docs/evidence/17.x.txt` with the command output and, for UI tasks, a screenshot.
4. Never log or send API keys; decrypt only at the point of use.
5. One task = one commit: `17.x: <summary>`.

---

## 1. Root-Cause Register

| ID | Symptom you reported / found | Root cause (file) | Fixed in |
|----|-----------------------------|-------------------|----------|
| R1 | Tools "succeed" with invented results | `starship-mcp/src/router.rs::route_and_call` step 3 falls back to `simulate_tool_call` (hard-coded "14 segments, 84.5 s", "-18.5 dB") and never flags it | 17.1 |
| R2 | App launch "works" with no app | `app/src-tauri/src/ipc/commands.rs` (~L979) returns `spawned: true, pid: 12345` and marks GUI active | 17.2 |
| R3 | Screencast plan uses invented cuts | `starship-core/src/pipeline/media.rs` default cut points (4.5–7.2 s, 22–28.5 s) | 17.3 |
| R4 | Fake charts | `ModelControlView.tsx` (preset arrays + `Math.random` every 3 s), `HudView.tsx` (`Math.random`), `AppTopologyView.tsx` and `MissionTimelineView.tsx` mock fallbacks | 17.5 |
| R5 | **Cannot chat with a downloaded model** | `LocalGgufRunner::stream_chat` returns canned text; no llama.cpp in any `Cargo.toml` | 17.10 |
| R6 | **Cloud API key "added but nothing loads"** | Key is stored as `enc_v1:…` (`Database::set_provider_config`), but `cmd_mission_send_prompt` passes `api_key_encrypted` straight to the HTTP client. `decrypt_api_key` is only called in a test, so every cloud request sends the encrypted blob as the key | 17.11 |
| R7 | **No model discovery (OpenRouter etc.)** | Nothing calls a provider's `/models` endpoint. `models` rows exist only from migration seed (`qwen2.5-coder-7b-q4`) and HF downloads. UI lists `ipc_hello.available_models` = DB rows | 17.20–17.25 |
| R8 | **No cloud model picker in chat** | Follows from R7; also `CockpitView.tsx` falls back to hard-coded `"qwen2.5-coder-7b-q4"` when nothing is selected | 17.23, 17.15 |
| R9 | Chat has no memory | `cmd_mission_send_prompt` sends only `vec![Message::user(prompt_text)]`; stored history is never loaded | 17.12 |
| R10 | Model can't act on tools | Stream loop only accumulates text; `tool_calls` are never parsed into plan steps | 17.13 |
| R11 | **Plugin scan finds no Zvirox apps** | `registry.rs::scan_and_sync_manifests` reads only `$XDG_DATA_HOME/starship/plugins/*.json`. It first **seeds fake manifests** with placeholder paths (`/usr/bin/<app>`, fixed versions). The sibling apps never write manifests (they don't integrate `starship-plugin`). Real binaries live elsewhere (e.g. `~/.local/bin/easy-edits-pro`), so `binary_path.exists()` is false and every app shows "offline" | 17.30–17.36 |
| R12 | Bad timestamps | `registry.rs::chrono_like_now` hard-codes month/day `-11-22` and approximates the year | 17.33 |
| R13 | Missions don't run as a system | `Scheduler`, `FallbackEngine`, retry, lease, admission, progress are only referenced inside `executor/` and tests; no `run_mission` command | 17.40–17.44 |
| R14 | Pipeline tools fail on the one real app | Templates emit `disk.scan` / `cache.purge_all`; ATP advertises `tuneup.scan_junk`, `atp.system.clean_package_cache` | 17.44 |
| R15 | Hard-coded impact text | Plans say "Purges 4.2 GB…" in `cmd_plan_generate` and `pipeline/diagnostics.rs` regardless of reality | 17.8 |
| R16 | No way to see if the engine works | No diagnostics or live engine view in the UI | 17.50–17.53 |

---

## 2. Phase 17A — Remove Every Fake

- [ ] **17.0 Confirm root causes.** For each R1–R15, add a line to `docs/evidence/17.0.txt`: file, line, `CONFIRMED` or `DIFFERENT (explain)`. Amend this plan if any differ.
- [ ] **17.1 Delete production simulation (R1).**
  * Remove `simulate_tool_call` from `starship-mcp/src/router.rs`. When no socket or binary works, return `CoreError::mcp("mcp.app_unavailable", retryable=false, …)` with `details: {appId, checked: [socketPath, binaryPath], reason}`.
  * Move the simulation to `starship-mock-app` (already exists) and to `#[cfg(feature="demo")]` with a visible **DEMO** badge and `simulated: true` in every result.
  * Executor treats `mcp.app_unavailable` as failure class `environment`.
  * *Verify:* IPC-level test: approve a step for an app with no binary and no socket → step ends `failed` with `mcp.app_unavailable`; no `"status":"success"` is ever produced. `cargo build --release` has no `simulate_` symbols.
- [ ] **17.2 Real launch only (R2).** `cmd_topology_launch_app` must `spawn()` a real binary and return its real PID, or return `mcp.app_spawn_failed`. Delete the "simulate GUI launch" branch and the fixed PID 12345. After spawn, wait up to 5 s for the bridge socket and report `launched`, or `launched_no_bridge` (honest).
  * *Verify:* test with a missing binary → typed error; test with a stub binary that creates the socket → `launched` with the true PID.
- [ ] **17.3 No invented cuts (R3).** In `pipeline/media.rs`, remove the default `CutInterval`s. With no CET telemetry, return `pipeline.telemetry_required`, and the UI shows "Record or load a session first". Cuts come only from real `detect_cuts`.
  * *Verify:* unit + IPC test: no telemetry → error, never a plan with cuts.
- [ ] **17.4 Remove canned model replies.** Delete the keyword-matched response code and `synthesize_tasksheet_json` from `local/runner.rs` (replaced by 17.10).
- [ ] **17.5 Real data for every chart (R4).**
  * Backend: record per-request metrics in the stream loop (time-to-first-token, tokens/s, tokens in/out, total latency, provider/model, error) into new table `infer_metrics` and emit `evt://infer/metrics`. Add `cmd_infer_metrics_history{range}`.
  * `ModelControlView.tsx`: initial state is empty, no preset arrays, no `Math.random`. Plot `infer_metrics` (tokens/s, latency). Empty state: "No inference yet — send a message". Show source and last-updated time.
  * `HudView.tsx`: replace random series with real health samples (`evt://health/sample`: CPU/RAM) or real tokens/s while streaming; empty state otherwise.
  * `AppTopologyView.tsx`: delete the "ecosystem mock for web preview" fallback; show real discovery results or an error.
  * `MissionTimelineView.tsx`: remove `mockSteps` and the mock toggle from production (keep in Storybook or dev-only).
  * *Verify:* grep gate (17.7) passes; with the app idle, charts are empty; after one chat message, a point appears with true numbers (compare to the log).
- [ ] **17.6** Replace placeholder `TuneupTelemetryChart`-style initial data anywhere else the gate finds (see 17.7).
- [ ] **17.7 No-fakes CI gate.** `cargo xtask check-no-fakes` fails if non-test code contains: `Math.random`, `simulate_tool_call`, `pid: Some(12345)`, `mockSteps`, `Default simulated`, hard-coded numeric telemetry arrays, or `"qwen2.5-coder-7b-q4"` outside the migration. Wire into CI and release workflows.
- [ ] **17.8 Truthful diff summaries (R15).** Remove "Purges 4.2 GB…" strings. Summaries are produced by the app's `describeChange` or by host code from real arguments or a real Tier-1 scan result (e.g. scan first, then show actual bytes). If unknown, show raw arguments, never an invented number.

---

## 3. Phase 17B — Real Chat Engine (local and cloud)

- [ ] **17.10 Real local inference (R5).** Implement `LocalGgufRunner` for real. Pick **one** path and document the choice in `ARCHITECTURE.md`:
  * **Path A (recommended first — fastest to a working chat):** managed **`llama-server`** sidecar. Spawn it with `-m <gguf> -c <ctx> -ngl <layers> --port <free-port> --host 127.0.0.1`, wait for `/health`, then talk to its OpenAI-compatible `/v1/chat/completions` stream using the existing `OpenAiClient`. Manage lifecycle (start on first use, idle unload, kill on exit, crash restart), use hardware-probe layer offload (`local/hardware.rs`), and refuse to load if headroom is insufficient (`local/guard.rs`). Locate or download the `llama-server` binary and verify its SHA-256; error `infer.runtime_missing` with an install hint if absent.
  * **Path B (later):** in-process `llama-cpp-2` behind feature `local-llama` (load in `spawn_blocking`, stream tokens, GBNF grammar support).
  * Both: apply the model's chat template, honor stop tokens, support cancel, and return real errors (`infer.model_load_failed`, `infer.out_of_memory`).
  * *Verify:* IPC test with a tiny GGUF (≤ 50 MB, fetched in CI cache) streams ≥ 16 real tokens, with `tokens/s` recorded. No canned text anywhere. (CI may use a small model; the full model is covered by the Manual Test Checklist.)
- [x] **17.11 Decrypt API keys at point of use (R6).** Add `Database::get_provider_credentials(provider_id) -> Option<SecretString>` that decrypts via `decrypt_api_key`. Use it in `cmd_mission_send_prompt`, discovery (17.20) and connection tests. Never log it.
  * *Verify:* wiremock server asserts `Authorization: Bearer <plaintext>` (and `x-api-key` for Anthropic). Regression test fails if the `enc_v1:` blob is ever sent.
- [ ] **17.12 Conversation memory (R9).** Build `messages` from `list_messages(mission_id)` (roles user, assistant, tool) with a token budget (trim oldest, keep system prompt and the last N turns). Persist assistant text and tool messages.
  * *Verify:* two-turn test: "My name is X" then "What is my name?" returns X from a mock engine that echoes its context.
- [ ] **17.13 Tool-call loop (R10).** Parse `tool_calls` from stream chunks (OpenAI/OpenRouter `delta.tool_calls`, Anthropic `tool_use`, Gemini `functionCall`). Convert them to a validated plan (12.1) → approval → execution (17.40) → feed results back as a `tool` message and continue until the model finishes. Cap loop depth (default 6).
  * *Verify:* mock provider requests `atp: tuneup.get_health`; a real ATP call result appears in the next model turn.
- [ ] **17.14 Downloaded-model hygiene.** After HF download: verify GGUF magic bytes and size, set `file_path`, `download_status='ready'`, and show it in the picker. Add `cmd_model_delete` (removes file and row) and resume of partial downloads. A model row whose file is missing shows **Not downloaded** and cannot be selected.
  * *Verify:* download test against a local HTTP fixture; corrupt file is rejected.
- [ ] **17.15 No silent default model (R8).** Remove the `"qwen2.5-coder-7b-q4"` fallback in `CockpitView.tsx`. With no usable model, the composer is disabled with a "Choose a model" call to action. Remove the seeded qwen row, or seed it as `remote_only` (not installed).
  * *Verify:* fresh profile → composer disabled; after selecting a model → enabled.
- [ ] **17.16 Streaming robustness.** Surface HTTP/stream errors in the chat as readable messages (401 → "API key rejected by <provider>", 404 → "model not found", 429 → "rate limited, retry in Ns"), with retry. Remove the artificial 15 ms sleep per chunk.

---

## 4. Phase 17C — Cloud Model Discovery & Picker (OpenRouter, OpenAI, Anthropic, Gemini)

> Endpoint details below are from the providers' public APIs; the agent must confirm each against current docs when implementing (17.26).

- [ ] **17.20 `ModelDiscovery` trait** in `starship-infer` (`async fn list_models(&self) -> Result<Vec<DiscoveredModel>>`), with `DiscoveredModel { id, display_name, context_window, supports_tools, supports_streaming, input_price, output_price, raw }`:
  * **OpenAI-compatible (OpenAI, OpenRouter, custom):** `GET {base_url}/models`, `Authorization: Bearer`. OpenRouter returns richer fields (`name`, `context_length`, `pricing`, `supported_parameters`) → use them to set tools support and prices.
  * **Anthropic:** `GET {base_url}/models` with `x-api-key` and `anthropic-version`; paginate with `limit` and `after_id`.
  * **Gemini:** `GET {base_url}/models` with the key; paginate with `pageToken`; keep models whose `supportedGenerationMethods` includes `generateContent`.
  * Timeouts 15 s, one retry on 5xx/429, never log the key.
  * *Verify:* wiremock fixtures for all four, including pagination and an empty list.
- [ ] **17.21 Persist discovered models.** Migration `0003_model_discovery.sql`: add to `models` the columns `provider_id`, `context_window`, `supports_tools`, `input_price`, `output_price`, `is_favorite`, `discovered_at`. Upsert on refresh. Remove discovered rows no longer returned, **except** favorites and models used by existing missions (mark them `unavailable`). Fix any CHECK constraints that block new values.
  * *Verify:* refresh twice → no duplicates; a disappearing model is flagged, not deleted, if used.
- [ ] **17.22 IPC.** `cmd_provider_list`, `cmd_provider_test_connection{providerId}` (returns `{ok, latencyMs, modelCount?, errorCode, message}`), `cmd_provider_refresh_models{providerId}`, `cmd_model_set_favorite`, `cmd_mission_set_model{missionId, modelId}`. Auto-run test + refresh right after a key is saved. Error mapping: 401/403 → `infer.auth_failed`, 404 → wrong base URL, network → `infer.network_unreachable`.
  * *Verify:* IPC tests per error mapping; saving a key triggers discovery.
- [ ] **17.23 UI — settings.** In `ModelSettingsModal.tsx`: after saving, show **Test connection** and **Fetch models** with a live result ("OpenRouter: 312 models loaded" or the exact error). Show "key saved ✓ / last verified time". Never display the key after save.
- [ ] **17.24 UI — chat model picker.** In the composer: a searchable combobox grouped **Local / OpenRouter / OpenAI / Anthropic / Gemini**, with search by id or name, favorites pinned on top, badges (tools ✓, context size, $/M tokens), "Refresh" and "Not configured → Add API key" entries. Handles 300+ items with virtualization. Persist last choice. Allow switching model mid-mission (`cmd_mission_set_model`) and show the model used on each assistant message.
  * *Verify:* with the OpenRouter fixture (300 models) the list renders and filters under 100 ms; selecting a model and sending hits that model id.
- [ ] **17.25 Manual model id.** An "Enter model id" field for providers (e.g. a new or private model not in the list), validated by a 1-token test call.
- [ ] **17.26 OpenRouter correctness.** Ignore SSE comment lines (keep-alives start with `:`), read mid-stream `error` objects, support optional `HTTP-Referer` / `X-Title` headers (settings), handle `:free` models' rate limits with a clear message, and pass `tools` only when the model supports them. Add `cargo xtask provider_smoke <provider>` that uses the saved key to list models and send one short prompt, so you can verify with your own key in one command.
- [ ] **17.27 Cost and context guard.** Show estimated context use per request; warn before sending if the prompt exceeds the model's context window.

---

## 5. Phase 17D — Real App Discovery (Plugin Scan)

- [ ] **17.30 Multi-source discovery (R11).** New `starship-mcp/src/discovery/` scanning, in order:
  1. Manifest dir `$XDG_DATA_HOME/starship/plugins/*.json` (apps that integrate the plugin SDK).
  2. Executables on `PATH`, `~/.local/bin`, `/usr/local/bin`, `/usr/bin`, `/opt/*/bin`, `~/Applications` (AppImage).
  3. `.desktop` files in `$XDG_DATA_DIRS/applications` and `~/.local/share/applications` (parse `Exec=`, `Name=`, `Version`).
  4. Flatpak (`flatpak list --app --columns=application,version`) if available.
  5. Live sockets in the bridge dir (`$XDG_RUNTIME_DIR/starship/bridges/*.sock`).
  * Known-app table: `app_id → binary-name aliases`, e.g. `eep: [easy-edits-pro]`, `ssp: [screen-studio-pro, screenstudio-pro]`, `alp: [audio-labs-pro]`, `nwp: [net-watch-pro]`, `atp: [agentic-tuneup-pro]`. Confirm real names from each repo's `tauri.conf.json` `productName` / `mainBinaryName`.
  * *Verify:* test with a temp `HOME`/`PATH` containing stub binaries, a `.desktop` file, and a socket → all found, each tagged with its source.
- [ ] **17.31 Verify, don't assume.** For each candidate run `<bin> --version` (2 s timeout, killed after) and the MCP probe (`starship/hello`). Classify as `mcp_ready`, `bridge_incompatible` (EEP's custom bridge), `binary_only`, `not_installed`, with a plain reason shown in the UI plus a suggested action (e.g. "Update to v0.2 for MCP support").
  * *Verify:* classification test table using the stub apps from `starship-mock-app`.
- [ ] **17.32 Remove seeded fake manifests.** Delete `seed_default_manifests_if_needed` and the placeholder `binary_path`/`version` values. Keep the known-app catalog separate from the installed list: the UI shows **Installed / Not installed (download link)**, never invented entries.
- [ ] **17.33 Fix timestamps (R12).** Replace `chrono_like_now` with real UTC (`time` or `chrono`) or SQLite `strftime`. Test with a fixed clock.
- [ ] **17.34 Rescan.** A **Rescan** button plus `evt://topology/changed`, driven by a file watcher (`notify`) on the manifest and bridge dirs and a 30 s low-priority poll. UI shows scan time and sources checked.
- [ ] **17.35 Trust prompt.** Newly found binaries go through the existing trust-on-first-use flow (10.5) before any tool call.
- [ ] **17.36 Sibling-app integration (separate repos; tracked here).** Per app, one PR + contract test `cargo xtask app_contract <appId>` (handshake, `tools/list`, one safe `tools/call`):
  * **SSP, ALP, NWP:** implement `--mcp` stdio using `starship-plugin`, write the manifest on start, create the bridge socket when the GUI runs.
  * **EEP:** add an adapter exposing its existing bridge (`command_list`, `doc_inspect`, …) as `initialize`, `starship/hello`, `tools/list`, `tools/call`.
  * **ATP:** already works; publish its real tool catalog as the source of truth (see 17.44).
  * *Verify:* `xtask app_contract` green for each app on a machine with the app installed; results go into the README support table (no MOCK-ONLY claims without evidence).

---

## 6. Phase 17E — Wire the Mission Runner (use the engines you built)

- [ ] **17.40 `MissionRunner` (R13).** In `starship-core/src/executor/runner.rs`, compose the existing parts: load plan → validate against the **live** tool catalog → `Scheduler` DAG → per step: `AdmissionController` → approval gate (Tier 2/3 wait for `cmd_plan_approve_step`, argument-bound) → `LeaseSupervisor` → `HybridExecutionRouter` with timeout → progress via `ProgressAggregator` → `RetryPolicy` / `FallbackEngine` → checkpoint/quarantine/rollback → every transition written to `mission_events` → `Governor` pause/resume. Crash recovery on startup (11.9).
- [ ] **17.41 IPC.** `cmd_mission_run`, `cmd_mission_pause`, `cmd_mission_resume`, `cmd_mission_abort`, `cmd_mission_restart_step`; events `evt://mission/progress`, `evt://step/state`, `evt://mission/fallback`.
  * *Verify (the key test):* an IPC-level integration test starts the real command path against `starship-mock-app` with scenario `crash`, `hang`, and `slow_progress` and asserts final mission state, retry count, fallback record, no leaked lease, and audit entries. Also `kill -9` of the core mid-mission → recovery card on restart.
- [ ] **17.42 Planner uses the model.** `cmd_plan_generate` calls the validated LLM planner (12.1–12.4) with the live tool catalog using the selected model; templates are used only when the user chooses **Template mode** or no model is usable (and the UI says so). Delete the `contains("disk")` keyword branches.
  * *Verify:* with a mock engine, the plan contains tools that exist in the catalog; an unknown tool is rejected with `plan.unknown_tool`.
- [ ] **17.43 UI wiring.** **Run**, **Pause**, **Abort** buttons; timeline and progress driven by real events; retry and fallback markers from `mission_events`; recovery card after a crash; step results shown from the real tool output.
- [ ] **17.44 Catalog-resolved templates (R14).** Template steps declare `{app, capability}` (e.g. `atp / scan_junk`) and resolve to the real tool name from `tools/list` at plan time; if unresolved, the plan is invalid with a clear reason. Update `pipeline_diagnostics` to use ATP's real tools.
  * *Verify:* diagnostics pipeline against a real ATP run executes `tuneup.scan_junk` end to end (evidence attached).

---

## 7. Phase 17F — "Is it working?" Diagnostics & Engine Console

- [ ] **17.50 Diagnostics page** (Settings → Diagnostics) with **Run all checks**. Each row shows PASS / FAIL / SKIPPED plus the real reason and a fix hint:
  1. Database opens, migrations current.
  2. **Apps:** per known app — found (source, path, version), identity check, MCP handshake, `tools/list` count, one safe Tier-1 call and its latency.
  3. **Providers:** key present, **Test connection**, model count, last refresh.
  4. **Local model:** file exists, size/SHA OK, runtime found (`llama-server`), load, generate 16 tokens with tokens/s.
  5. **Cloud model:** send "reply with OK" to the selected model; show the reply and latency.
  6. Health sampler live values; governor state; disk space.
  * Also expose as CLI `starship-control --self-test --json`, and **Export report**.
- [ ] **17.51 Engine Console.** A live panel (and `--engine-log`) listing real events with timestamps: IPC commands, router path chosen (`socket` / `stdio` / `unavailable`), tool call args (secrets redacted) and results, token stream stats, provider requests (URL host and status only), governor decisions. Each event labeled with its true source; a global **DEMO** banner appears if the demo feature is on.
- [ ] **17.52 Status bar truth.** The header shows real state: selected model and provider (with a ✓/✗ from the last test), number of apps `mcp_ready`, and health state. No static labels.
- [ ] **17.53 First-run checklist.** On first launch show actionable setup: choose or download a model or add an API key → discover apps → run Diagnostics.

---

## 8. Manual Test Checklist (what you run to confirm it works)

| # | Test | Pass condition |
|---|------|----------------|
| T1 | Launch app with no apps installed and no models | Apps page says "Not installed" (no fake entries); chat composer disabled with "Choose a model"; all charts empty with honest empty states |
| T2 | Install/launch ATP, press **Rescan** | ATP appears with real path and version, classification `mcp_ready`, tool count > 0 |
| T3 | Same with EEP / SSP / ALP / NWP | Each shows found/not found with a specific reason (e.g. "bridge incompatible") — never a fake "online" |
| T4 | Add OpenRouter key → **Test connection** → **Fetch models** | Shows "N models loaded"; with a wrong key shows "API key rejected" |
| T5 | Open model picker in chat | OpenRouter models are listed, searchable, grouped; choose one; send a message; reply streams; assistant message shows the model id |
| T6 | Repeat T4–T5 for OpenAI / Anthropic / Gemini keys you have | Same behavior |
| T7 | Download a small GGUF from the Models tab → select it → chat | Real, non-canned reply; tokens/s appears in the chart; asking "what is 17×23" gets a model answer, not a template |
| T8 | Send two related messages | Second reply uses context from the first |
| T9 | Ask "check my system health" with ATP running | Model requests a tool, you approve, real ATP result appears and the model summarizes it |
| T10 | Run a mission, then kill the app process mid-step | On restart a recovery card appears; nothing is silently marked successful |
| T11 | Disconnect network, send a cloud message | Clear network error with Retry; no fake reply |
| T12 | Settings → Diagnostics → Run all | Every row is real; failures name the cause |

---

## 9. Order, Dependencies, Definition of Done

```
17.0 → 17.1–17.8 (remove fakes) ──┬→ 17.10–17.16 (real chat) → 17.20–17.27 (discovery/picker)
                                  ├→ 17.30–17.36 (app discovery)
                                  └→ 17.40–17.44 (runner)  → 17.50–17.53 (diagnostics, built incrementally from the start)
```
- Do **17.11 first** (one-line decrypt fix, unblocks every cloud test) and **17.1** (stops misleading success).
- Build **17.50/17.51 incrementally** alongside each task so every fix is testable in the UI.
- 17.36 needs work in the sibling repos; until each app passes `xtask app_contract`, its row stays "bridge incompatible" in the UI. That is the truth, not a bug.

**Phase 17 DoD**
- [ ] `cargo xtask check-no-fakes` green in CI; release build has no demo code.
- [ ] T1–T12 pass on a clean machine; evidence screenshots and logs attached.
- [ ] Cloud and local chat both produce real model output; OpenRouter models discovered and selectable.
- [ ] App scan finds installed Zvirox apps and explains the ones it cannot drive.
- [ ] A mission runs through the runner with retry, fallback and recovery proven at the IPC level.
- [ ] README support table lists only measured results; GAP-CLOSURE-PLAN ticks for 11.x, 13.x, 14.7 and 15.x are re-verified as **wired**, not just unit-tested.

## Appendix — New/Changed Files

```
crates/starship-infer/src/{discovery.rs, local/runner.rs (real), local/sidecar.rs}
crates/starship-mcp/src/{discovery/, router.rs (no simulate)}
crates/starship-core/src/executor/runner.rs
crates/starship-db/src/migrations/0003_model_discovery.sql  (+ infer_metrics table)
app/src-tauri/src/ipc/{providers.rs, diagnostics.rs, mission_run.rs}
app/src/components/{settings/DiagnosticsView.tsx, console/EngineConsole.tsx, chat/ModelPicker.tsx}
xtask: check-no-fakes, provider_smoke, app_contract
```
