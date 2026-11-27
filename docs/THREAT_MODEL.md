# THREAT_MODEL.md — Starship Control

**Version:** 1.0 · **Date:** 2026-11-27 · **Task:** `GAP-CLOSURE-PLAN.md` 16.7
**Scope:** the shipped Linux desktop application (`starship-control`), its Tauri webview, the
MCP control plane, sub-apps, the local database, and the release pipeline.
**Method:** asset inventory → trust boundaries → threat enumeration (STRIDE-style) →
mitigation mapped to the implementing task and the test that proves it.

This document is the evidence for 16.7. Every **High** gap in `GAP-CLOSURE-PLAN.md §1`
maps to at least one mitigation and one automated test in the coverage matrix (§6).

---

## 1. Assets

| ID | Asset | Why it matters |
|----|-------|----------------|
| A1 | Cloud provider API keys | Billing and account compromise if leaked |
| A2 | Approval authority (signed tokens) | The only thing standing between an LLM and Tier 2/3 actions |
| A3 | User files on disk | Tier 3 operations (delete, cache wipe) are destructive |
| A4 | Audit log integrity | Post-incident forensics and accountability |
| A5 | Mission plans, events, tool arguments | Contain user goals and file paths; privacy-sensitive |
| A6 | Local model weights / GPU memory | Availability of the machine |
| A7 | The host user's session | Same-user processes can reach sockets, DB and IPC |

## 2. Trust boundaries

```
                 ┌───────────────────────────────┐
   untrusted ──► │  LLM (cloud or local GGUF)     │  A1 never leaves the host except to its provider
                 └──────────────┬────────────────┘
                                │ plans / tool args (untrusted data)
                 ┌──────────────▼────────────────┐
                 │  Rust core (Tauri backend)     │  trusted computing base
                 │  SafetyArbiter · planner       │
                 │  executor · MCP pool · DB      │
                 └───┬───────────┬───────────┬────┘
        IPC (allowlist)│   Unix socket│   SQLite│
                 ┌───▼────┐  ┌────▼─────┐ ┌───▼──────────┐
   untrusted ──► │ Webview│  │ Sub-apps │ │ Local DB file │ ◄── same-user processes
                 └────────┘  └──────────┘ └───────────────┘
```

- **B1 LLM → core.** Model output is untrusted data. It may be malformed, adversarial, or
  prompt-injected via tool results and file names.
- **B2 sub-app → core.** Sub-apps are separate processes with their own manifests and
  sockets; a buggy or compromised app may self-declare a low risk tier, squat a socket, or
  ship a swapped binary.
- **B3 webview → core.** The React UI is a renderer; it must not be able to approve actions
  or invoke arbitrary privileged commands.
- **B4 same-user process → core.** Any process running as the same UID can attempt to
  connect to sockets, read the DB, or tamper with files.
- **B5 core → OS.** The core is the only component allowed to mutate the system, and only
  after the tiered-safety policy.

## 3. Threat table

| ID | Boundary | Threat | Mitigation (task) | Proven by |
|----|----------|--------|-------------------|-----------|
| T1 | B1 | Malformed / oversized plan reaches the executor | JSON-Schema validator, DAG + arg checks, step/tier caps (12.1); repair loop then fallback (12.2); GBNF for local models (12.3) | `12.1`–`12.3` evidence; 15 invalid fixtures |
| T2 | B1 | Prompt injection through tool output, file names, window titles | External strings wrapped in `<untrusted_context>`; LLM diff summaries ignored and host-computed (10.2) | `test_llm_diff_summary_ignored_and_overridden` |
| T3 | B1 | Model picks a tool the host cannot bound | Two-stage routing + token budget injects only known tools (12.4); unknown tools default to Tier 2 (10.1) | `12.4`, `test_table_driven_tier_policy` |
| T4 | B2 | Sub-app self-declares `riskTier: safe` | Host floor `max(app_declared, host_floor)`; destructive keywords force Tier 3 (10.1) | `test_table_driven_tier_policy` |
| T5 | B2 | Swapped or tampered sub-app binary | Manifest pinning (TOFU SHA-256) + `/proc/<pid>/exe` peer check + `SO_PEERCRED` UID (10.5, 10.6) | `test_tampered_manifest_refuses_spawn`, `test_peer_binary_mismatch_rejected` |
| T6 | B2 | Socket squatting by another same-user process | Peer exe must match the pinned manifest path; `pidfd` where available (10.6) | `test_peer_binary_mismatch_rejected` |
| T7 | B2 | A sub-app hangs, crashes, or lies about progress | Per-step timeouts, lease expiry + cancel, stall detection, failure classifier, fallback ladder (11.3–11.6, 11.10, 13.2) | `11.3`–`11.6`, `11.10` evidence |
| T8 | B2 | Irreversible deletion without a recoverable state | Checkpoint contract before first Tier 2 mutation (11.7); quarantine instead of delete for ATP (11.8) | `11.7`, `11.8` evidence |
| T9 | B3 | Webview triggers a privileged action directly | All privileged work is a typed Rust command; approval requires a signed, argument-bound token (10.3) | `test_argument_bound_approval_*` (replay / wrong step / expired rejected) |
| T10 | B3 | XSS / remote content executes in the webview | Strict CSP, self-hosted assets, no remote scripts (10.4) | `10.4` evidence |
| T11 | B4 | Another same-user process reads API keys | Secrets encrypted at rest (`enc_v1:`), Secret Service integration, scrubbed from logs/prompts (10.7) | `test_secrets_at_rest_encrypted_in_sqlite`, `test_canary_key_never_appears_in_plaintext_or_prompts` |
| T12 | B4 | Audit rows edited or deleted | Hash-chained audit log + immutability triggers; `audit verify` command (10.8) | `test_tamper_evident_audit_log_hash_chain_and_triggers` |
| T13 | B4 | Replay of an old approval token | Token bound to plan, step, tool and argument hash; single-use; expiry (10.3) | `test_argument_bound_approval_replay_rejected` |
| T14 | B5 | Crash mid-mission leaves orphaned locks or half-applied state | Event-sourced run log, lease supervisor, crash recovery marks `interrupted`, never auto-resumes Tier 2/3 (11.2, 11.4, 11.9) | `11.2`, `11.4`, `11.9` evidence |
| T15 | B5 | Overload drives the machine to a lockup | Health sampler + governor safe-pause at step boundary (14.x, 15.x) | `14.7`, `15.x` evidence |
| T16 | release | Tampered or unverifiable package | Locked nfpm manifests, `SHA256SUMS`, GPG `SHA256SUMS.sig`; CI container install checks (16.6) | `16.6` evidence / release run |

## 4. Safety invariants (must never be weakened)

1. No mutating (Tier 2) or destructive (Tier 3) tool call executes without a valid,
   argument-bound, signed approval.
2. Fallbacks never auto-approve, never lower a tier, and never auto-retry Tier 3.
3. LLM output, tool output, file names, window titles and CET sidecars are untrusted data.
4. Every Tier 2/3 transition is written to the audit chain **before** the action runs.
5. A re-plan is a new plan version; approvals do not carry over.

## 5. Residual risks (accepted, documented)

| Risk | Why accepted | Control |
|------|--------------|---------|
| A same-user process can `ptrace`/read memory of the core | Linux same-UID model; blocking it requires kernel hardening (Yama, hidepid) outside the app | Documented; secrets are not held longer than needed; DB secrets encrypted at rest |
| `style-src 'unsafe-inline'` in CSP | Tailwind/inline style attributes are required by the current UI build | No remote script sources; `script-src 'self'` |
| `img-src https:` allows remote image loads | Needed for provider/model artwork | No user-controlled URL is rendered as an image without allowlisting; network egress is visible |
| Local GGUF inference runs in-process | Native bindings are memory-unsafe by nature | `deny(unsafe_code)` workspace-wide; `llama-cpp-2` isolation is a tracked open item (`PLAN.md` 1.3) |
| Dark-mode card shadow is the only translucency besides scrim/fades | Explicit design decision | Enforced by `check:linear` |

## 6. High-gap coverage matrix (`GAP-CLOSURE-PLAN.md §1`)

| Gap | Severity | Mitigation task | Test / evidence |
|-----|----------|-----------------|-----------------|
| G1 — no CI, no evidence | High | 9.1 CI workflows (lint, clippy, tests, build, security-audit) | `docs/evidence/9.1.txt`; `.github/workflows/ci.yml` |
| G2 — plaintext keys / "encrypted" claim | High | 10.7 SecretVault `enc_v1`, scrubbing | `docs/evidence/10.7.txt` (2 tests) |
| G3 — app-declared risk tier trusted | High | 10.1 host tier floor; 10.2 host-computed diff | `docs/evidence/10.1.txt`, `10.2.txt` |
| G4 — approval not bound; webview can approve | High | 10.3 argument-bound signed token; 10.4 CSP | `docs/evidence/10.3.txt`, `10.4.txt` |
| G5 — UID-only peer check, unpinned binary, squatting | High | 10.5 manifest pinning; 10.6 peer exe check | `docs/evidence/10.5.txt`, `10.6.txt` |
| G7 — no executor spec (retry/timeout/rollback/crash) | High | 11.1–11.6, 11.9, 11.10 | `docs/evidence/11.1–11.6.txt`, `11.9.txt`, `11.10.txt` |
| G9 — checkpoints missing for some apps; ATP deletes irreversible | High | 11.7 checkpoint contract; 11.8 quarantine | `docs/evidence/11.7.txt`, `11.8.txt` |
| G10 — no plan validation / repair | High | 12.1 validator; 12.2 repair; 12.3 GBNF | `docs/evidence/12.1.txt`–`12.3.txt` |
| G11 — all tools injected, context blow-up | High | 12.4 two-stage routing + budget | `docs/evidence/12.4.txt` |

Medium gaps G6, G8, G12–G16 are also mitigated (audit chain 10.8, lease supervisor 11.4,
pipelines 12.6, progress 13.x, health 14.x, admission 15.x, harness 16.1–16.5, packaging 16.6).

## 7. How to re-verify

```bash
# Security suite (tier floor, approval binding, manifest pinning, peer exe, CSP,
# secret scrubbing, audit chain) — mutation-tested in CI per 16.4.
cargo test --workspace

# Design/contract gates
cd app && npm run check:linear && npm run typecheck && npm run test:security
```

Evidence for every row lives in `docs/evidence/<task>.txt`. This document is re-checked
whenever a safety-relevant task lands; the `check:linear` and CI gates prevent silent drift.
