# Compositor compatibility matrix

**Measured:** 2026-11-25 · **Source of truth for README "Tested on"** · **Probe:** `cargo run -p xtask --bin compat_probe`

Status values: **works** (measured), **degraded** (configuration verified, runtime pixel/layer check unavailable on this host), **not supported** (no verification evidence gathered here — not a claim of incompatibility).

| Compositor | Session / version | HUD always-on-top | Solid opaque rendering | Global shortcut `Super+Shift+Space` | Fallback used | Evidence |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| KDE Plasma 6 | Wayland · kwin 6.8.3 | degraded | degraded | works | Tauri `alwaysOnTop` + XDG GlobalShortcuts portal; in-app Cockpit toggle | [`compat/kde-plasma-kwin-6.8.3/`](evidence/compat/kde-plasma-kwin-6.8.3/probe.json), [`portal.log`](evidence/compat/kde-plasma-kwin-6.8.3/portal.log) |
| GNOME | Wayland · not measured | not supported | not supported | not supported | XDG GlobalShortcuts portal (expected) | NOT TESTED on this host |
| Sway | Wayland · not measured | not supported | not supported | not supported | XDG GlobalShortcuts portal (expected) | NOT TESTED on this host |
| Hyprland | Wayland · not measured | not supported | not supported | not supported | XDG GlobalShortcuts portal (expected) | NOT TESTED on this host |
| X11 | any · not measured | not supported | not supported | not supported | Cockpit in-app toggle only — native `XGrabKey` fallback is not implemented (`PLAN.md` 6.2) | NOT TESTED on this host |

## What was actually measured on KDE Plasma 6

- Session: `XDG_SESSION_TYPE=wayland`, `XDG_CURRENT_DESKTOP=KDE`, `kwin 6.8.3`, `WAYLAND_DISPLAY=wayland-1`.
- HUD config: `420×64`, `transparent: false`, `alwaysOnTop: true`, `decorations: false` in `app/src-tauri/tauri.conf.json`.
- Global shortcut: the app registered `toggle_hud` through the XDG GlobalShortcuts portal at runtime (see `portal.log`).
- Not measured: a pixel-level opacity capture and KWin keep-above layer assertion. `agent-desktop` reports `PLATFORM_NOT_SUPPORTED` for display capture on this KDE Wayland session, so those two rows stay `degraded` rather than being claimed as `works`.

## Re-run on another compositor

1. Log in to the target compositor.
2. Run `cargo run -p xtask --bin compat_probe`.
3. It writes `docs/evidence/compat/<session-key>/probe.json` with the detected session, HUD config and shortcut capabilities.
4. Start `target/debug/starship-control` and confirm `Super+Shift+Space` toggles the HUD, then add a dated note here and flip the relevant cells from `not supported` to `works` or `degraded`.

## Known gaps

- Native X11 `XGrabKey` global hotkey is not implemented; X11 users must toggle the HUD from the Cockpit (`PLAN.md` 6.2).
- The app's WebKit window is not exposed in this host's AT-SPI window list, so automatic window-level assertions are unavailable; manual verification is required.
