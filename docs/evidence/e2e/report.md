# Real-app E2E matrix

Generated: 2026-11-25

| App | Binary | MCP support | Status |
| :-- | :-- | :-- | :-- |
| Screen Studio Pro | /usr/bin/screenstudio-pro | Missing | MOCK-ONLY |
| Easy Edits Pro | /home/zvirox/.local/bin/easy-edits-pro | SocketBridgeNonMcp | MOCK-ONLY |
| Audio Labs Pro | /usr/bin/audio-labs-pro | NoMcp | MOCK-ONLY |
| Net Watch Pro | /usr/bin/net-watch-pro | NoMcp | MOCK-ONLY |
| Agentic Tuneup Pro | /usr/bin/agentic-tuneup-pro | StdioMcp | REAL |

## Pipeline steps
### pipeline_screencast
- `ssp` / `recording.inspect_take` (Tier 1) — MOCK-ONLY
- `alp` / `audio.cleanup` (Tier 2) — MOCK-ONLY
- `alp` / `tts.synthesize` (Tier 2) — MOCK-ONLY
- `eep` / `timeline.apply_cuts` (Tier 2) — MOCK-ONLY
- `eep` / `timeline.align_audio` (Tier 2) — MOCK-ONLY
### pipeline_record_and_edit
- `ssp` / `ssp.recording.start` (Tier 1) — MOCK-ONLY
- `ssp` / `ssp.recording.stop` (Tier 1) — MOCK-ONLY
- `ssp` / `ssp.recording.start` (Tier 1) — MOCK-ONLY
- `alp` / `alp.denoise.spectral` (Tier 2) — MOCK-ONLY
- `eep` / `eep.timeline.cut_interval` (Tier 2) — MOCK-ONLY
- `eep` / `eep.timeline.auto_trim_silence` (Tier 2) — MOCK-ONLY
### pipeline_diagnostics
- `nwp` / `network.scan` (Tier 1) — MOCK-ONLY
- `atp` / `disk.scan` (Tier 1) — MOCK-ONLY
- `atp` / `cache.purge_all` (Tier 3) — MOCK-ONLY

## Blockers
- ssp: not installed (binary: /usr/bin/screenstudio-pro)
- eep: custom socket bridge, not MCP-compatible (binary: /home/zvirox/.local/bin/easy-edits-pro)
- alp: no working --mcp stdio mode (binary: /usr/bin/audio-labs-pro)
- nwp: no working --mcp stdio mode (binary: /usr/bin/net-watch-pro)