# GAP-CLOSURE-PLAN.md — Starship Control: Gap Filling, Missing Features & Build Plan

**Version:** 1.0 · **Date:** 2026-11-22
**Continues:** `PLAN.md` (Phases 0–8). This file defines **Phases 9–16**.
**Authority chain:** `PRD.md` → `ARCHITECTURE.md` → `DESIGN.md` → `DATABASE.md` → `API.md` → `PLAN.md` → **this file**.
**Audience:** a coding agent (see `AGENTS.md`) executing one task at a time.

> **Honesty note.** This plan was written from the public docs (README, PLAN, ARCHITECTURE, API) and the repo's Actions page. The Rust/TypeScript source was **not** reviewed. Task 9.0 exists to reconcile this plan with the real code before anything else is built. Where this plan names a module or file that does not exist, create it; where it conflicts with the code, the code audit (`docs/evidence/AUDIT.md`) wins and this plan is amended in the same commit.

---

## 0. Execution Rules (read first)

1. **Vertical slices.** Every phase ends with a runnable app (`cargo tauri dev`) showing real behavior. No "backend now, UI later".
2. **Task size** ≤ 1–2 hours. One task = one commit. Commit message: `<task-id>: <summary>`.
3. **Evidence rule (non-negotiable).** A checkbox may be ticked only when the task's **Verify** command/test passes and its output is saved to `docs/evidence/<task-id>.txt` (or linked CI run). PLAN.md ticked items without evidence are treated as **unverified** until Task 9.0 re-checks them.
4. **Tests first** for every new state machine, parser and policy (table-driven tests).
5. **Safety invariants (never weaken):**
   - No mutating/destructive tool call without a valid, argument-bound approval.
   - Fallback paths **never** auto-approve, auto-retry Tier 3, or lower a risk tier.
   - LLM output and tool output are untrusted data.
6. **Security lints:** `deny(unsafe_code)` workspace-wide; the only allowed `unsafe` is `libc` peer-credential/`pidfd` calls isolated in `starship-mcp/src/security.rs` with a `// SAFETY:` comment each.
7. **Feature flags** for anything hardware-specific (`nvml`, `amdgpu-sysfs`) so CI stays green on machines without GPUs.
8. **Docs move with code.** If a task changes a contract, update `API.md` / `DATABASE.md` / `ARCHITECTURE.md` in the same commit.

---

## 1. Gap Register

| ID | Gap (observed) | Severity | Fixed in |
|----|----------------|----------|----------|
| G1 | PLAN.md ticks all 8 phases, but no CI workflow runs (Actions tab empty); no evidence of tests/verify steps | High | 9 |
| G2 | README claims "encrypted SQLite"; architecture has plain bundled SQLite + optional libsecret; `0600` is not encryption | High | 9 |
| G3 | Sub-apps declare their own `riskTier` → a buggy/compromised app can self-declare "safe" | High | 10 |
| G4 | Approval token is an opaque string not bound to step arguments; webview can call approve command | High | 10 |
| G5 | `SO_PEERCRED` checks UID only; manifest `binaryPath` is spawned without pinning; socket squatting possible | High | 10 |
| G6 | Audit log called "immutable" but is ordinary SQLite rows | Medium | 10 |
| G7 | No executor spec: retry, timeout, idempotency, partial-failure, rollback, crash-resume, re-plan | High | 11 |
| G8 | README mentions a "Lease & Lock Supervisor"; no module/spec for it (only `mcp.lease_expired`) | Medium | 11 |
| G9 | Checkpoints exist only for EEP/ALP; ATP deletes are irreversible; SSP has none | High | 11 |
| G10 | No plan validation (schema, cycles, arg checks), no repair loop; small local models will emit bad JSON | High | 12 |
| G11 | All sub-app tools injected into context (EEP has 490+ verbs) → context blow-up on 7B local models | High | 12 |
| G12 | Flagship flows are hard-coded pipelines; free-form planning unproven; quickstart prompt ("record 60 s…") not covered by `pipeline_screencast` | Medium | 12 |
| G13 | No mission progress model, ETA, stall detection, or crash-safe event log | Medium | 13 |
| G14 | No system health monitoring, overload detection, or resource-aware scheduling | Medium | 14, 15 |
| G15 | No fault-injection / mock-app / e2e test harness; Wayland HUD + global-shortcut claims unverified per compositor | Medium | 16 |
| G16 | Release/AUR/deb/rpm install commands in README unverified | Medium | 16 |

---

## Phase 9 — Truth, CI & Foundations

> **Goal:** Make "done" mean "proven". CI exists, tests run, README matches reality.

- [x] **9.0 Ground-Truth Audit**
  * Build workspace from clean clone. Run `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cd app && npm ci && npx tsc --noEmit && npm run build`.
  * For every ticked item in PLAN.md write one line in `docs/evidence/AUDIT.md`: `VERIFIED` (with command), `PARTIAL`, or `MISSING`. Un-tick anything not VERIFIED.
  * Diff the real crate/module layout against `ARCHITECTURE.md §4`; list differences.
  * *Verify:* `AUDIT.md` covers all PLAN.md items; counts of VERIFIED/PARTIAL/MISSING at the top.
- [x] **9.1 CI workflow** — `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, `cargo test --workspace`, `tsc --noEmit`, `npm run build`, `cargo deny`/`cargo audit`. Cache cargo + npm.
  * *Verify:* green run linked in `docs/evidence/9.1.txt`.
- [x] **9.2 Release workflow** — `.github/workflows/release.yml` building `.deb`, `.rpm`, `.tar.gz` + SHA256SUMS on tag push.
  * *Verify:* tag `v0.1.0-alpha.2` produces all three artifacts + checksums.
- [x] **9.3 Install smoke test in CI** — containers (ubuntu:24.04, fedora:latest, archlinux) install the package and run `starship-control --version` and `--self-test` (new flag: opens DB, runs migrations, exits 0).
  * *Verify:* matrix green (tested via `docs/evidence/9.3.txt`).
- [x] **9.4 README truth pass** — correct "encrypted SQLite" claim (see 10.7), "tested on" compositor list (only list what Phase 16 verifies), remove "zero latency" marketing, mark AUR as "planned" until published.
  * *Verify:* every factual README claim has a link to a test, doc section, or evidence file.
- [x] **9.5 Coverage & test baseline** — `cargo llvm-cov` report in CI; record baseline for `starship-core`, `starship-mcp`, `starship-db`.
  * *Verify:* coverage summary posted as CI artifact.

**DoD (Phase 9):** CI is green on main, a tagged release installs on three distros, and `AUDIT.md` is truthful.

---

## Phase 10 — Hardened Control Plane

> **Goal:** Close G3–G6. Safety is enforced by the host, not trusted from apps or the webview.

- [x] **10.1 Host-side tier policy** — new `starship-core/src/policy/tiers.rs` + `policy/tiers.toml` (shipped default). Effective tier = `max(app_declared, host_floor)`. Unknown tool → **Tier 2**. Tools matching `delete|purge|remove|overwrite|replace|format|truncate` → floor **Tier 3** unless allowlisted. Apps can raise, never lower.
  * *Verify:* table-driven test: app declares tier 1 for `files.delete` → effective tier 3.
- [x] **10.2 Deterministic diff summaries** — `diffSummary` is produced by the sub-app (new tool metadata `describeChange`) or by host code from arguments; **never** by the LLM. Missing → card shows raw arguments.
  * *Verify:* test with an LLM-supplied `diffSummary` field being ignored.
- [x] **10.3 Argument-bound approvals** — core creates `approval_request {id, nonce, argsHash(sha256 of canonical JSON), tier, expiresAt(5 min)}`; UI replies `plan_approve_step{stepId, nonce}`; at dispatch core re-hashes the *current* arguments; mismatch → `safety.invalid_signature_token`. Nonces single-use, HMAC'd with a per-process in-memory key.
  * *Verify:* tests: replay, expired, args mutated after approval, wrong step.
- [x] **10.4 Webview hardening** — strict CSP; sanitize streamed markdown (no raw HTML, no `javascript:` URLs); Tauri capabilities least-privilege per window; frontend approval handler requires `event.isTrusted`.
  * *Verify:* XSS payload in a mock tool result renders inert; capability manifest lists only needed commands.
- [x] **10.5 Plugin manifest pinning** — `binaryPath` must be absolute, not world-writable, not inside `/tmp` or `$HOME/Downloads`. First sight of a manifest → Cockpit prompt (TOFU) storing `{appId, binaryPath, sha256}`; any later change → re-prompt and **refuse to spawn** until approved.
  * *Verify:* tampered manifest test refuses spawn with `mcp.manifest_untrusted` (new code).
- [x] **10.6 Peer identity check** — after `SO_PEERCRED` (uid + pid), open `pidfd` and compare `/proc/<pid>/exe` to the manifest's pinned `binaryPath`; mismatch → `mcp.so_peercred_failed`. Bridge dir `0700`, socket `0600`.
  * *Verify:* test where a different binary listens on the app's socket path is rejected.
- [x] **10.7 Secrets at rest** — API keys stored via Secret Service (`libsecret`/`keyring` crate) when available; if unavailable, require explicit user opt-in to store in SQLite, encrypted with a key derived (Argon2id) from a user passphrase. Scrub keys from logs/LLM context (test with canary key).
  * *Verify:* canary key never appears in logs, DB plaintext (default mode), or prompts.
- [x] **10.8 Tamper-evident audit log** — add `prev_hash`, `entry_hash = sha256(prev_hash || canonical_row)`; CLI `starship-control audit verify` walks the chain. DB user cannot UPDATE/DELETE audit rows (SQLite triggers `RAISE(ABORT)`).
  * *Verify:* editing one row makes `audit verify` fail and name the row.

**DoD (Phase 10):** A malicious app/manifest/webview cannot execute a mutating step without a human-approved, argument-bound token; audit log tampering is detectable.

---

## Phase 11 — Executor Engine, Checkpoints & Fallback Plan

> **Goal:** Close G7–G9. Every mission survives failure, crash, and overload predictably.

### 11A. State machines

```
Mission: planning → ready → running ⇄ paused(user|governor|approval)
         → succeeded | partially_succeeded | failed | aborted

Step:    pending → ready → awaiting_approval → running
         → succeeded
         | failed_retryable → (backoff) → ready
         | failed_final → (fallback ladder) → ready | skipped | rolled_back
         | cancelled | interrupted (set on crash recovery)
```

- [x] **11.1 Step/mission state machine** in `starship-core/src/executor/state.rs` — pure, side-effect-free transition function + exhaustive tests (illegal transitions return `ipc.bad_state`).
  * *Verify:* property test: no sequence of events reaches an illegal state.
- [x] **11.2 Event-sourced run log** — append-only `mission_events` table (see Appendix B). Every transition, tool call, retry, fallback decision writes an event first, then acts.
  * *Verify:* rebuilding mission state from events equals live state in a test.
- [x] **11.3 DAG scheduler** — ready-set computation, bounded parallelism (default 2, governor-adjustable), cycle detection at load, per-step `timeoutMs` (default from `tool_stats` p95 × 3, min 10 s).
  * *Verify:* diamond DAG runs in expected order; a step exceeding timeout is cancelled via `tools/cancel`.
- [x] **11.4 Lease & lock supervisor** (the missing module from README) — `executor/lease.rs`: per-resource locks keyed `app:project:track`; leases carry `leaseTimeoutMs`; heartbeat renewal while a step reports progress; on expiry → `tools/cancel` + `mcp.lease_expired`; deadlock-free ordering (sorted lock acquisition).
  * *Verify:* two missions targeting the same EEP project serialize; a crashed holder's lease expires and is released.
- [x] **11.5 Idempotency & retry policy** — each tool carries `idempotent: bool` (host default `false`). Retry only if idempotent **or** the call has an idempotency key the app honors. Exponential backoff with jitter: 1 s, 3 s, 9 s; max 3. **Tier 3 steps: 0 automatic retries.**
  * *Verify:* test matrix of (idempotent × tier × error class) → expected retry count.
- [x] **11.6 Failure classifier** — map errors to classes (§ Fallback Plan table) with a single `fn classify(err) -> FailureClass`.
  * *Verify:* every code in `API.md §1.2` maps to exactly one class (test enumerates codes).
- [x] **11.7 Checkpoint contract** — extend `starship-plugin` (v1.1): required tools `checkpoint.create(label)` / `checkpoint.restore(id)` / `checkpoint.drop(id)` for any app exposing Tier 2+ tools; host calls `create` before the first Tier 2 step touching a resource and records `checkpoint_id` on the step.
  * *Verify:* mock app: failure after mutation restores prior state.
- [x] **11.8 Quarantine instead of delete (ATP)** — Tier 3 file removal moves items to `$XDG_DATA_HOME/starship-control/quarantine/<mission>/` with a manifest and TTL (default 7 days); `restore` and `purge` are separate Tier 3 steps. Package-cache cleaners that can't be reversed are labeled **IRREVERSIBLE** on the approval card and require a typed confirmation.
  * *Verify:* cleanup → restore round-trip restores byte-identical files; irreversible items show the warning.
- [x] **11.9 Crash recovery** — on startup, scan missions with `running` steps → mark `interrupted`, release stale leases, show a Resume / Restart-step / Abort card. Never auto-resume Tier 2/3 steps.
  * *Verify:* `kill -9` during a step, restart, card appears; resume re-requests approval.
- [x] **11.10 Fallback ladder engine** — implements §2 below as `executor/fallback.rs`, configurable per pipeline in `fallbacks.toml`.
  * *Verify:* each ladder rung has a test using the mock app (Phase 16 harness stubs are acceptable here).
- [x] **11.11 Failure report & handoff bundle** — on terminal failure generate `mission-<id>-report.md` (what ran, what failed, what was rolled back, current state, suggested manual steps) + export bundle (plan JSON, events, logs with secrets scrubbed).
  * *Verify:* bundle opens, contains no canary secrets.

**DoD (Phase 11):** Killing an app, the core process, or the network mid-mission leads to a defined, recorded, user-visible outcome — never a silent hang.

---

## 2. The Fallback Plan

### 2.1 Failure taxonomy → policy

| Class | Examples | Retry | Next rung | Rollback? | User notified |
|-------|----------|-------|-----------|-----------|---------------|
| `transient` | `mcp.socket_unreachable`, `mcp.handshake_timeout`, `infer.cloud_rate_limited` | Yes (3, backoff) | F1 | No | After 2nd failure |
| `app_failure` | `mcp.execution_failed`, app crash, `mcp.lease_expired` | If idempotent (1) | F1 → F2 → F5 | Yes if mutated | Yes |
| `resource` | `infer.model_out_of_memory`, `disk.no_space`, governor `Critical` | No | F3-model / safe-pause | No | Yes |
| `model_output` | invalid JSON plan, unknown tool, schema-invalid args, `infer.context_length_exceeded` | Repair ×2 | F3 → no-LLM template | No | Only if unresolved |
| `safety` | `safety.approval_rejected`, `safety.invalid_signature_token`, `safety.tier_violation` | **Never** | Skip/Stop | If partial | Yes (always) |
| `environment` | portal missing, DB write failed, shortcut denied | Yes (1) | Degraded mode | No | Yes |
| `user` | abort, rejected approval | No | Stop | Yes if partial | Confirmation |

### 2.2 Per-step ladder (stop at first success; each rung logged to `mission_events`)

- **F0 — Retry same call.** Only if allowed by 11.5. Backoff + jitter.
- **F1 — Alternate transport.** GUI socket failed → restart/relaunch app, or headless `--mcp` stdio; stdio failed → relaunch GUI. Re-run handshake + peer check each time.
- **F2 — Alternate tool/route.** Use the declared `alternatives` for the tool (e.g., batch vs per-interval cut), defined in `fallbacks.toml`; same tier or higher only.
- **F3 — Alternate model / re-plan.** Re-plan the *remaining* DAG with failure context (max **1 re-plan per mission**). New plan must pass validation (12.1) and **re-approval** for every Tier 2/3 step. Model chain: primary → secondary cloud → local small model → **no-LLM template mode** (hard-coded pipelines from `pipeline_*` still run without any model).
- **F4 — Degrade.** Skip steps flagged `optional: true` (e.g., intro narration), mark mission `partially_succeeded`, explain what's missing.
- **F5 — Rollback.** Restore checkpoint(s) / restore quarantine; verify restore; record outcome. If restore fails → `failed` with **Manual Recovery** section in the report.
- **F6 — Safe stop & handoff.** Cancel in-flight calls, release leases, release locks, write report/bundle, notify (HUD + tray + desktop notification), no further automatic action.

### 2.3 Infrastructure fallbacks

| Component down | Fallback |
|----------------|----------|
| Cloud model unreachable / auth fail | Next configured provider → local GGUF → no-LLM template mode |
| Local model OOM | Drop to smaller quant / fewer GPU layers / CPU, then cloud (if user enabled) |
| Sub-app missing | Headless `--mcp`; if no headless, pipeline degrades (F4) or asks user to install/launch |
| SQLite write failure | Read-only degraded mode, in-memory event buffer (cap 10k) flushed on recovery, banner shown |
| Wayland portal / global shortcut unavailable | Tray menu + in-app button toggle HUD; first-run notice explains why |
| HUD always-on-top unsupported by compositor | Fall back to a normal floating window; document per-compositor matrix (16.5) |
| Governor `Critical` | Safe-pause at the next step boundary; auto-resume rules in 14.7 |

### 2.4 Hard rules for fallbacks
1. Fallbacks never auto-approve or reduce a tier.
2. A re-plan is a **new plan version**; approvals do not carry over.
3. Retrying a Tier 3 step requires a fresh human approval.
4. Every rung decision records `{from, to, reason, failureClass}` for the audit trail.

---

## Phase 12 — Planner Reliability & Tool Routing

> **Goal:** Close G10–G12. Plans are valid by construction and affordable in a small context window.

- [x] **12.1 TaskSheet schema & validator** — JSON Schema for `tasksheet/1` (check in `schemas/tasksheet-1.json`); validator checks: schema, unique step ids, DAG acyclic, tools exist in the live registry, arguments validate against each tool's JSON Schema, max steps (default 30), max Tier 3 steps per plan (default 3).
  * *Verify:* 15 invalid fixtures each rejected with a specific error.
- [x] **12.2 Repair loop** — on validation failure, send errors back to the model (≤ 2 attempts), then escalate to F3 chain.
  * *Verify:* mock engine returns bad JSON twice then good → plan accepted; third failure → fallback.
- [ ] **12.3 Constrained decoding for local models** — generate a GBNF grammar from the schema and use it with `llama-cpp-2` sampling; cloud providers use native tool/JSON modes.
  * *Verify:* 20-prompt golden suite on a 7B Q4 model: ≥ 90 % schema-valid (record actual number; adjust target honestly).
- [ ] **12.4 Two-stage tool routing** — stage 1: model picks apps from a one-line capability list; stage 2: inject only those apps' tools, ranked by embedding/BM25 over tool descriptions, capped by token budget (`context/budget.rs`).
  * *Verify:* with a mock app exposing 500 tools, prompt stays under budget and the correct tool is present in ≥ 95 % of golden cases.
- [ ] **12.5 Plan preview & dry-run** — before executing, show the DAG with tiers, estimated duration (from `tool_stats`), estimated resource profile and checkpoints; "Dry run" executes Tier 1 inspection steps only and validates arguments against live app state.
  * *Verify:* dry run never calls a Tier 2/3 tool (test asserts).
- [ ] **12.6 Close the quickstart gap** — add `pipeline_record_and_edit` template (SSP record → CET read → ALP cleanup → EEP import/cuts) so the README's "record a 60-second demo…" prompt is real; planner prefers a matching template over free-form when intent matches (keyword + model confirmation).
  * *Verify:* the quickstart prompt yields a validated 5+ stage plan in no-LLM mode with `--intent record_and_edit`.
- [ ] **12.7 Planner eval harness** — `cargo xtask eval-planner` runs the golden prompts against configured engines and writes `docs/evidence/planner-eval.json` (valid %, tool-accuracy %, replan rate).
  * *Verify:* report generated; CI runs it against the mock engine only.

**DoD (Phase 12):** An invalid or oversized plan can never reach the executor; local-model planning has a measured, documented success rate.

---

## Phase 13 — Progress Tracking

> **Goal:** Close G13. At any moment the user (or a returning user) can see what is done, what is running, what's next, and how long it will take.

### Model
- **Step progress** `0.0–1.0` from the sub-app via MCP `notifications/progress` (`progressToken`, `progress`, `total`, `message`); if the app sends none, host synthesizes indeterminate progress.
- **Mission progress** = Σ(step.weight × step.progress) / Σ weights, where weight defaults to the step's historical median duration (fallback 1.0).
- **ETA** = Σ remaining weights × EWMA(duration/weight per tool) from `tool_stats`; show a range (p50–p90), mark `~` when < 5 samples.
- **Stall detection:** no progress event or heartbeat for `max(30 s, 3 × expected interval)` → state `stalled` → triggers F0/F1.

- [ ] **13.1 Progress protocol** — add `notifications/progress` handling to `starship-mcp/client.rs`; extend `starship-plugin` with a `ProgressReporter` helper; add `progressToken` to `tools/call`.
  * *Verify:* mock tool emits 10 progress events; host maps them to the right step.
- [ ] **13.2 Progress aggregator** — `executor/progress.rs`: computes step/mission progress, ETA, stall flag; throttled to ≤ 4 Hz per mission.
  * *Verify:* unit tests for weighting, ETA with/without history, throttling.
- [ ] **13.3 `tool_stats` learner** — after each successful step update `{tool, n, ewma_ms, p50, p90}` (see Appendix B). Ignore failed/cancelled runs.
  * *Verify:* ETA error on a replayed history < 25 % after 10 runs (record result).
- [ ] **13.4 Events & commands** — `evt://mission/progress`, `evt://step/state`, `cmd://mission_get_timeline` (events since cursor, for resync after reconnect).
  * *Verify:* frontend reconnect replays missed events with no gaps (cursor test).
- [ ] **13.5 Cockpit Mission Timeline UI** — DAG/timeline view: per-step state chips, progress bars, tier badges, retry/fallback markers, elapsed + ETA, live log drawer; "Why did this fallback happen?" link to the event.
  * *Verify:* visual check with a mock mission containing a retry, a fallback and a skipped step; screenshots saved to evidence.
- [ ] **13.6 HUD progress & tray state** — HUD shows a compact progress ring + current step; tray icon states: Idle / Thinking / Executing / Waiting Approval / **Stalled** / **Overloaded** / Failed.
  * *Verify:* each state reachable through the mock harness; screenshots.
- [ ] **13.7 Notifications** — desktop notification (portal `Notification` via `ashpd`) on: approval needed, mission finished, mission failed, stalled > 2 min. Rate-limit and dedupe.
  * *Verify:* with Cockpit hidden, completing a mock mission raises exactly one notification.
- [ ] **13.8 History & replay** — Cockpit "Missions" list with status, duration, outcome, fallback count; open any past mission's timeline from `mission_events` (read-only replay).
  * *Verify:* replay of a finished mission reproduces final state exactly.

**DoD (Phase 13):** Progress, ETA and stalls are visible in Cockpit, HUD and tray; a restarted app can show the last run accurately.

---

## Phase 14 — System Health & Overload Monitor + Dashboard

> **Goal:** Close G14. Starship Control knows how loaded the PC is, shows it, and protects the machine (and the mission) when it is overloaded.

### Metrics collected (new crate `crates/starship-health`)

| Group | Source (Linux) | Notes |
|-------|----------------|-------|
| CPU util, per-core, load avg, freq | `sysinfo` crate / `/proc/stat` | |
| **CPU/mem/IO pressure (PSI)** | `/proc/pressure/{cpu,memory,io}` | kernel ≥ 4.20 with PSI enabled; `some`/`full` avg10/avg60 |
| Memory: used/available, swap, swap-in rate | `/proc/meminfo`, `/proc/vmstat` | alert on **MemAvailable**, not "free" |
| Disk: free %, read/write MB/s, await | `sysinfo`, `/proc/diskstats` | per mounted fs of interest (home, data dir, models dir) |
| Network: rx/tx, connections | `/proc/net/dev`; **enriched by Net Watch Pro** when live | |
| Temperatures & fan | `/sys/class/hwmon/*` (use each sensor's `max`/`crit` when present) | |
| GPU util / VRAM / temp | NVIDIA: `nvml-wrapper` (feature `nvml`); AMD: `/sys/class/drm/card*/device/{gpu_busy_percent,mem_info_vram_*}` (feature `amdgpu-sysfs`); Intel/other: best-effort, else "n/a" | |
| Battery / AC | `/sys/class/power_supply` | laptop low-battery guard |
| Top processes (CPU, RSS) | `sysinfo` | tag Starship-owned PIDs (core + spawned sub-apps) |
| Starship's own footprint | self + children | CPU%, RSS, open sockets |

- [ ] **14.1 `starship-health` crate skeleton** — `Sampler` trait per metric group, `HealthSample` struct, graceful `Unavailable` for missing sensors (never panic, never fail the app).
  * *Verify:* on a container with no GPU/hwmon, sampler returns a sample with `gpu: None` and exits cleanly.
- [ ] **14.2 CPU/mem/disk/net samplers** using `sysinfo` + `/proc`; adaptive rate: **1 Hz** while dashboard/HUD visible, **0.2 Hz** otherwise, **1 Hz** while a mission runs.
  * *Verify:* sampler overhead (own CPU) < 1 % average on a quiet machine (measured, recorded).
- [ ] **14.3 PSI + thermal samplers** — parse PSI files (handle absence), hwmon scan with caching of sensor paths.
  * *Verify:* parser tests with real-format fixtures; missing PSI handled.
- [ ] **14.4 GPU samplers (feature-gated)** — NVML and amdgpu sysfs; VRAM used/total, util, temp.
  * *Verify:* feature builds in CI (stub/mocked); manual evidence on real hardware attached when available.
- [ ] **14.5 Storage** — in-memory ring buffer (15 min @ 1 Hz); SQLite rollups: 1-min averages/max kept 7 days, 1-hour kept 90 days (tables in Appendix B). Prune job daily.
  * *Verify:* 24 h simulated feed produces expected row counts and prune works.
- [ ] **14.6 Overload detector** — rules with **hysteresis** (enter after N consecutive samples, exit after M below a lower threshold) to prevent flapping.

  | Signal | Elevated | Constrained | Critical | Exit |
  |--------|----------|-------------|----------|------|
  | CPU util (60 s avg) | > 80 % | > 90 % | > 97 % for 30 s | < 70 % for 30 s |
  | CPU PSI some avg10 | > 20 | > 40 | > 60 | < 15 |
  | MemAvailable | < 20 % | < 10 % | < 5 % **or** PSI mem full avg10 > 1 | > 25 % |
  | Swap-in rate | > 5 MB/s | > 20 MB/s | > 50 MB/s | < 2 MB/s |
  | Disk free (data/models dir) | < 15 % | < 8 % | < 3 % or < 2 GB | > 12 % |
  | IO PSI some avg10 | > 20 | > 40 | > 60 | < 15 |
  | Temperature | within 15 °C of `max` | within 7 °C | within 3 °C of `crit` | 10 °C below enter |
  | VRAM used | > 85 % | > 92 % | > 97 % | < 80 % |
  | Battery (not on AC) | < 30 % | < 15 % | < 7 % | on AC |

  Thresholds are **defaults** (user-editable in Settings). Overall state = worst signal; `Normal → Elevated → Constrained → Critical`.
  * *Verify:* replay tests: synthetic traces produce expected state transitions with no flapping.
- [ ] **14.7 Governor (actions per state)** — in `starship-core/src/governor.rs`:
  * **Elevated:** warning chip; stop background prefetch (e.g., model metadata refresh); log event.
  * **Constrained:** parallelism → 1; no new *heavy* steps (by resource profile, 15.1); unload idle local model; lower local-inference threads.
  * **Critical:** **safe-pause** at next step boundary (never kill mid-write); request checkpoint if supported; refuse new local inference; notify. Auto-resume after state is ≤ Elevated for 60 s **only if** the mission was governor-paused (never if user-paused or awaiting approval).
  * Governor **never** kills user applications; it only cancels its own calls and processes.
  * *Verify:* `stress-ng` scenario (CPU + memory) drives state to Constrained/Critical; a running mock mission pauses, then resumes after load stops.
- [ ] **14.8 Health commands & events** — `cmd://health_get_snapshot`, `cmd://health_get_history{range,resolution}`, `cmd://health_get_thresholds/set_thresholds`, `evt://health/sample` (1 Hz max), `evt://health/state_changed`, `evt://health/alert`.
  * *Verify:* IPC schema tests (Rust ↔ TS types identical).
- [ ] **14.9 Cockpit "Systems" tab (Dashboard)** — layout:
  * Header: overall state chip (Normal/Elevated/Constrained/Critical) + governor action in effect.
  * Gauge row: CPU, RAM, GPU, VRAM, Disk, Temp (with PSI bars under CPU/RAM/IO).
  * 15-min sparklines per metric with **mission start/end markers** overlaid.
  * Top processes table (Starship-owned rows highlighted, "Starship total" row).
  * Alerts timeline (what tripped, when, what governor did).
  * Settings drawer for thresholds, sampling, auto-resume.
  * Follows `DESIGN.md` glass tokens; no new colors without updating `DESIGN.md`.
  * *Verify:* mock feed drives each state; screenshots in evidence; keyboard navigable.
- [ ] **14.10 HUD & tray integration** — HUD shows a small health dot (green/amber/red) and expands the plan view with a banner when the governor pauses a mission; tray state **Overloaded**.
  * *Verify:* injected Critical state shows banner within 2 s.
- [ ] **14.11 Health in missions** — each mission stores peak CPU/RAM/temp and number of governor interventions in `missions` summary for the report/history.
  * *Verify:* completed mock mission has populated summary.

**DoD (Phase 14):** The Systems tab shows live health, overload is detected without flapping, and the governor pauses/throttles missions and recovers automatically.

---

## Phase 15 — Resource-Aware Scheduling (Admission Control)

> **Goal:** Prevent overload instead of reacting to it.

- [ ] **15.1 Resource profiles** — extend tool metadata: `resourceProfile {cpu: low|med|high, memMb, gpu: bool, diskIo: low|high, longRunning: bool}`; unknown → `high`. Sub-app SDK (`starship-plugin` v1.1) supports declaring it; host can override in `policy/resources.toml`.
  * *Verify:* unknown tools default to `high` (test).
- [ ] **15.2 Admission controller** — a step starts only if `headroom(now) ≥ profile` (using current health + reservations of running steps); otherwise queued with reason "waiting for resources" (visible in timeline).
  * *Verify:* with simulated 85 % CPU, a `high` step queues, a `low` step runs.
- [ ] **15.3 Local-inference guard** — before loading a GGUF, compare VRAM/RAM headroom (live, not just static probe); if insufficient → F3-model path (smaller quant/offload/cloud) *before* attempting the load.
  * *Verify:* simulated low memory triggers fallback with a clear user message and no OOM kill.
- [ ] **15.4 Process containment (optional, feature flag)** — run spawned headless apps via `systemd-run --user --scope -p MemoryMax=… -p CPUWeight=…` when available, so a runaway app can't take the desktop down; fall back to plain spawn with a warning when not.
  * *Verify:* a mock app that allocates unbounded memory is killed by its scope, mission goes to `failed_final` → F1/F5 as appropriate, desktop stays responsive.
- [ ] **15.5 Fair queueing across missions** — max concurrent missions (default 1 running, others queued), priority = user-initiated > scheduled.
  * *Verify:* two missions queue and run in order.

**DoD (Phase 15):** Heavy steps are scheduled only when the machine can take them; a runaway sub-app cannot freeze the desktop (when containment is available).

---

## Phase 16 — Test Harness, Chaos & Release Readiness

> **Goal:** Close G15–G16. Prove the claims.

- [ ] **16.1 `starship-mock-app` crate** — a configurable fake sub-app (socket + `--mcp` stdio) implementing `starship-plugin`: scripted tools with controllable latency, progress events, failure modes (`crash`, `hang`, `bad_json`, `slow_progress`, `deny_checkpoint`, `oom`), via env/JSON scenario files.
  * *Verify:* scenario files exercise every failure mode.
- [ ] **16.2 Fault-injection scenarios** (integration tests in `tests/chaos/`): app killed mid-step; socket squatter; manifest tampered; model returns bad JSON; cloud 429/500; DB read-only; disk full (tmpfs); CPU/mem stress via `stress-ng`; core `kill -9` mid-mission.
  * *Verify:* each scenario asserts final mission state, audit entries, no leaked locks, no orphan processes.
- [ ] **16.3 End-to-end pipelines against real apps** — manual+scripted run of `pipeline_screencast`, `pipeline_record_and_edit`, `pipeline_diagnostics` with real EEP/SSP/ALP/NWP/ATP builds; record versions, logs, short screen capture in `docs/evidence/e2e/`. Any app lacking a working `--mcp` mode is listed as a **blocker**, and the corresponding pipeline step is marked `MOCK-ONLY` in README until fixed.
  * *Verify:* evidence folder present; README support table reflects reality.
- [ ] **16.4 Security test suite** — tests for: tier floor, approval binding/replay, manifest pinning, peer exe check, CSP/XSS render, secret scrubbing, audit chain. Wire into CI.
  * *Verify:* all pass; a mutation (disable one check) makes a test fail.
- [ ] **16.5 Compositor matrix** — manually verify HUD always-on-top, transparency, global shortcut on GNOME, KDE Plasma 6, Sway, Hyprland, X11; table in `docs/COMPAT.md` (`works / degraded / not supported` + fallback used). README "tested on" mirrors this file only.
  * *Verify:* `COMPAT.md` filled with dated test notes and versions.
- [ ] **16.6 Packaging verification** — install/uninstall/upgrade on Ubuntu LTS, Fedora, Arch; publish AUR package (or remove README AUR instructions); checksums + signature (minisign or GPG) for releases.
  * *Verify:* clean-VM install logs in evidence.
- [ ] **16.7 Threat model doc** — `docs/THREAT_MODEL.md` (assets, trust boundaries: LLM, sub-apps, webview, same-user processes; mitigations mapped to task IDs 10.x).
  * *Verify:* every High gap in §1 maps to a mitigation + test.
- [ ] **16.8 Performance budgets** — idle core < 150 MB RSS and < 1 % CPU; HUD toggle < 150 ms; event latency core→UI < 50 ms at 1 Hz health + 4 Hz progress.
  * *Verify:* benchmark script output saved; budgets enforced as CI warnings.

**DoD (Phase 16):** Every claim in the README has evidence; failure modes are tested, not assumed.

---

## 3. Recommended Order & Parallelism

```
9 (truth/CI) → 10 (hardening) → 11 (executor+fallback) ─┬→ 12 (planner)
                                                        ├→ 13 (progress)  ─┐
                                                        └→ 14 (health) → 15 (admission)
                                                                           ↓
                                                                  16 (harness/release)
```
- 16.1 (mock app) is needed earlier than Phase 16 by 11.10 and 13.x tests — build the **minimal** mock app at the start of Phase 11 and extend it in Phase 16.
- 13 and 14 are independent and can be built in parallel after 11.
- Do not start 15 until 14.7 (governor) is verified.

---

## Appendix A — New/Changed Modules

```
crates/
├── starship-core/src/
│   ├── policy/{tiers.rs,tiers.toml,resources.toml}
│   ├── executor/{state.rs,scheduler.rs,lease.rs,retry.rs,fallback.rs,progress.rs,report.rs}
│   ├── planner/{schema.rs,validate.rs,repair.rs,routing.rs,templates/}
│   ├── governor.rs
│   └── context/budget.rs
├── starship-health/            # NEW — samplers, ring buffer, detector
├── starship-mcp/src/{security.rs (peer exe/pidfd), trust.rs (manifest TOFU)}
├── starship-plugin/            # v1.1: progress, checkpoint.*, resourceProfile, idempotent, health/ping
├── starship-db/src/{events.rs,health.rs,tool_stats.rs,audit.rs(hash chain)}
└── starship-mock-app/          # NEW — fault-injectable fake sub-app
schemas/tasksheet-1.json
fallbacks.toml
docs/{evidence/,COMPAT.md,THREAT_MODEL.md}
.github/workflows/{ci.yml,release.yml}
```

## Appendix B — Migration `0002_gap_closure.sql` (sketch; reconcile with `DATABASE.md`)

```sql
-- append-only run log
CREATE TABLE mission_events (
  id INTEGER PRIMARY KEY, mission_id INTEGER NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  step_id INTEGER, ts_ms INTEGER NOT NULL, kind TEXT NOT NULL,     -- transition|tool_call|retry|fallback|progress|governor|approval
  payload_json TEXT NOT NULL
);
CREATE INDEX idx_events_mission ON mission_events(mission_id, id);

-- extend steps
ALTER TABLE plan_steps ADD COLUMN state TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE plan_steps ADD COLUMN attempt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plan_steps ADD COLUMN checkpoint_id TEXT;
ALTER TABLE plan_steps ADD COLUMN optional INTEGER NOT NULL DEFAULT 0;
ALTER TABLE plan_steps ADD COLUMN idempotency_key TEXT;
ALTER TABLE plan_steps ADD COLUMN progress REAL NOT NULL DEFAULT 0;
ALTER TABLE plan_steps ADD COLUMN failure_class TEXT;

CREATE TABLE tool_stats (
  app_id TEXT, tool TEXT, n INTEGER, ewma_ms REAL, p50_ms REAL, p90_ms REAL,
  PRIMARY KEY (app_id, tool)
);

CREATE TABLE health_rollup_1m (ts INTEGER PRIMARY KEY, cpu_avg REAL, cpu_max REAL, mem_avail_pct_min REAL,
  swap_in_max REAL, disk_free_pct_min REAL, temp_max REAL, gpu_util_max REAL, vram_pct_max REAL,
  psi_cpu_max REAL, psi_mem_max REAL, psi_io_max REAL, state TEXT);
CREATE TABLE health_rollup_1h (ts INTEGER PRIMARY KEY, cpu_avg REAL, cpu_max REAL, mem_avail_pct_min REAL,
  swap_in_max REAL, disk_free_pct_min REAL, temp_max REAL, gpu_util_max REAL, vram_pct_max REAL,
  psi_cpu_max REAL, psi_mem_max REAL, psi_io_max REAL, state TEXT);
CREATE TABLE health_alerts (id INTEGER PRIMARY KEY, ts INTEGER, signal TEXT, level TEXT, value REAL, action TEXT, mission_id INTEGER);

CREATE TABLE trusted_apps (app_id TEXT PRIMARY KEY, binary_path TEXT NOT NULL, sha256 TEXT NOT NULL, approved_at INTEGER NOT NULL);

-- audit hash chain
ALTER TABLE audit_log ADD COLUMN prev_hash TEXT;
ALTER TABLE audit_log ADD COLUMN entry_hash TEXT;
CREATE TRIGGER audit_no_update BEFORE UPDATE ON audit_log BEGIN SELECT RAISE(ABORT,'audit_log is append-only'); END;
CREATE TRIGGER audit_no_delete BEFORE DELETE ON audit_log BEGIN SELECT RAISE(ABORT,'audit_log is append-only'); END;

-- mission summary
ALTER TABLE missions ADD COLUMN peak_cpu REAL;
ALTER TABLE missions ADD COLUMN peak_mem_pct REAL;
ALTER TABLE missions ADD COLUMN governor_interventions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE missions ADD COLUMN outcome TEXT;
```
> Note: the audit triggers block retention pruning — implement pruning as an explicit, audited "archive + new chain anchor" operation.

## Appendix C — New IPC Surface

**Commands:** `mission_get_timeline`, `mission_resume`, `mission_restart_step`, `mission_export_bundle`, `health_get_snapshot`, `health_get_history`, `health_get_thresholds`, `health_set_thresholds`, `plan_dry_run`, `app_trust_approve`, `app_trust_revoke`.
**Events:** `evt://mission/progress`, `evt://step/state`, `evt://mission/fallback`, `evt://health/sample`, `evt://health/state_changed`, `evt://health/alert`, `evt://app/trust_required`.
**Error codes (add to `API.md §1.2`):** `mcp.manifest_untrusted`, `mcp.peer_exe_mismatch`, `mcp.checkpoint_unsupported`, `exec.step_timeout`, `exec.stalled`, `exec.replan_exhausted`, `exec.rollback_failed`, `plan.invalid_schema`, `plan.cycle_detected`, `plan.unknown_tool`, `plan.args_invalid`, `health.sensor_unavailable`, `governor.paused`, `governor.admission_denied`.

## Appendix D — Acceptance Checklist (whole program)

- [ ] CI green; release installs on Ubuntu, Fedora, Arch (evidence attached)
- [ ] `AUDIT.md` shows no `MISSING` items still ticked in PLAN.md
- [ ] Tier floors, approval binding, manifest pinning, peer check, audit chain — all tested
- [ ] Kill-app / kill-core / network-loss / disk-full / overload chaos scenarios pass
- [ ] Fallback ladder demonstrated: retry → alt transport → re-plan → degrade → rollback → safe stop
- [ ] Progress, ETA, stall detection visible in Cockpit, HUD, tray; replay works
- [ ] Systems dashboard live; overload detection has no flapping; governor pauses and resumes
- [ ] No-LLM template mode runs all three pipelines
- [ ] README claims match `COMPAT.md` and evidence
