# Changelog

All notable changes to Starship Control are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Threat model (`docs/THREAT_MODEL.md`, Phase 16.7)**: assets A1–A7, trust boundaries B1–B5 (LLM, sub-apps, webview, same-user processes, core→OS), threat table T1–T16 mapped to implementing tasks and tests, accepted residual risks, and a High-gap coverage matrix proving every High gap in `GAP-CLOSURE-PLAN.md §1` has a mitigation + test (`docs/evidence/16.7.txt`).
- **Performance budgets (Phase 16.8)**: `cargo run -p xtask --bin perf_budget` runs the real app and enforces idle footprint (< 150 MB PSS), idle CPU (< 1 %), HUD toggle (< 150 ms), and event dispatch (< 50 ms); a warn-only `performance-budget` CI job runs under Xvfb. `cargo run -p starship-control -- --perf-probe [idle|latency]` exposes the raw measurements (`docs/evidence/16.8.txt`).
- **Adaptive health sampling wired (Phase 14.2)**: 1 Hz while a window is visible, 0.2 Hz when minimized to the tray; `/proc/net/tcp` connection counting no longer allocates a string per line, and the sampler overhead test measures thread CPU time instead of wall time (0.49–0.54 % of one core at 1 Hz).

### Fixed

- **Documentation drift:** reconciled `PLAN.md` / `GAP-CLOSURE-PLAN.md` status lines, phase DoD wording and Appendix D checkboxes with the verified codebase; refreshed `9.1`/`9.3` evidence, added the missing `9.2`/`9.4`/`9.5` evidence files, and updated version/branch references in `AUDIT.md` and `docs/release-audit.md`.

## [0.1.0-beta.5] - 2026-11-26

### Changed

- **Colorful dashboard and charts**: Each chart series and component role now uses a distinct hue instead of collapsing to violet/sky. Throughput = blue + amber, latency bars = blue + pink, model-states donut = sky/amber/purple/red, and the global chart palette is `primary, blue, amber, pink, purple, orange, sky, red`.
- **Distinct component accents**: `--accent-blue` → blue, `--accent-teal` → purple, positive/healthy → sky, with amber/pink/purple/orange/red kept separate. Chips and badges gained per-hue solid tints (blue, amber, pink, purple, orange, sky, red) in `app-accent.css`.
- Primary buttons, links and focus rings remain **Violet**; no green is used anywhere (the `check:linear` gate still passes).

## [0.1.0-beta.4] - 2026-11-26

### Changed

- **Violet primary accent**: Starship Control's per-app accent is now **Violet**, isolated in `app/src/styles/app-accent.css` (`--primary #8B3DFF` dark / `#6D28D9` light, `--primary-text #B28CFF` / `#6D28D9`). Primary buttons, links, focus rings, active nav and selected tabs use the Violet accent; all other tokens stay identical to the shared six-app palette.
- **Green and teal removed**: Positive, healthy, online, done and up now use `--sky` (replacing green); teal is dropped from series lists. Chart series order is `--primary, --sky, --amber, --pink, --blue` (purple skipped as a near-duplicate of the Violet primary).
- **Design gate**: `check:linear` now also validates `app-accent.css` (sha256 sync), rejects foreign-app primary accents and any green hue (75–170°), and enforces the full Plan §8 contrast matrix alongside the retired-value list. `DESIGN.md` (v2.2) and `AGENTS.md` mirror the Violet palette.

## [0.1.0-beta.3] - 2026-11-26

### Changed

- **Shared Linear color palette migration**: Replaced the SSC palette with the byte-identical six-app `linear-theme.css` (dark `#1C1C1C`/`#292929` surfaces, `#346AFE` blue, `#1A9942` green, `#FC4232` red). Dark primary buttons now use white fill with dark text; light stays blue.
- **Runtime chart colors**: `echarts-linear-theme.ts` reads CSS variables via `getComputedStyle` and re-colors on live theme toggle; no hex literals remain in chart code.
- **Design gate**: `check:linear` now enforces token-only colors, retired-value rejection, ATP token sha256 sync, the single allowed `--shadow-card`, and WCAG contrast pairs; `DESIGN.md` and `AGENTS.md` mirror the new palette.

## [0.1.0-beta.2] - 2026-11-25

### Changed
- **Finalized & Locked Linear Design System (`DESIGN.md` v2.1)**: Standardized on opaque Linear surfaces (`#050505`/`#111213`/`#1A1B1E` dark and `#F3F4F6`/`#FFFFFF`/`#F4F5F7` light), `#3355FF` primary accent, self-hosted `Inter` + `JetBrains Mono` typography, `1280×800` Cockpit (`40px` TitleBar, collapsible `240px → 56px` Sidebar, `36px` Breadcrumb), and solid opaque `420×64` Floating HUD with live dark/light theme synchronization and zero blur regressions (`npm run check:linear`).
- **Documentation & Roadmap Sync**: Aligned `PRD.md`, `ARCHITECTURE.md`, `DATABASE.md`, `API.md`, `PLAN.md`, `GAP-CLOSURE-PLAN.md`, `AGENTS.md`, `README.md`, `docs/release-audit.md`, and `docs/evidence/AUDIT.md` with actual crate implementations (`starship-health`, `xtask`, migrations `0001`–`0006`, Phases 9–15 complete, Phase 16 & native `llama-cpp-2` bindings remaining open).
- **Packaging & Release Signing**: Locked `nfpm` `.deb`/`.rpm`/`.pkg.tar.zst` distribution policy, retired AUR/VM plans, and enabled GPG `SHA256SUMS.sig` release signing.

## [0.1.0-beta.1] - 2026-11-24

### Added
- **Unified Linear-Style Design System**: Adopted solid matte surfaces (`#050505`, `#111213`, `#1A1B1E`), `1px` precision hairline borders (`rgba(255,255,255,0.08)`), `#3355FF` accent, and zero blur across Cockpit and HUD.
- **ECharts 6 Telemetry & Command Center**: Integrated Linear-themed real-time charts for CPU, RAM, GPU, VRAM, disk, and inference headroom diagnostics.
- **Packaging & CI**: Configured `.github/workflows/ci.yml` and reusable `nfpm` release packaging (`packaging/nfpm.yaml`) generating `.deb`, `.rpm`, and `.pkg.tar.zst` packages.
- **Ecosystem Bridge**: Unified orchestration, discovery, safety arbiter, event-sourced run log, and admission control for Easy Edits Pro, Screen Studio Pro, Audio Labs Pro, Net Watch Pro, and Agentic Tuneup Pro.
