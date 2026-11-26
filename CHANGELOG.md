# Changelog

All notable changes to Starship Control are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
