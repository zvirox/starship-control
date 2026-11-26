# Color migration — Starship Control (SSC)

Date: 2026-11-26 · Branch: `colors/linear-palette` · Plan step 5 of 6

## Result

- `app/src/styles/linear-theme.css` copied byte-for-byte from the attached ATP token file.
  `sha256 = 8f939da3eb48b2f04a80b5bc2f85c45a8ed2794cfc267744c32e8e5fae47d0f4`
- `app/src/tailwind.css` now only maps tokens with `@theme inline`; it contains no hex, rgb or hsl.
- Legacy CSS variable names used by components (`--accent-blue`, `--tint-red`, …) are
  aliased in `tailwind.css` to the new role tokens. No component layout, spacing,
  radius, window config or behavior was changed.
- Primary buttons use `--btn-primary-bg` / `--btn-primary-text` (dark: white fill with
  dark text; light: blue fill with white text). Other accents map to the shared series tokens.
- `echarts-linear-theme.ts` reads every color from CSS variables at runtime via
  `getComputedStyle(document.documentElement)` and exposes `getLinearSeriesOrder()`.
  The chart re-renders on the existing theme-store change, so colors follow the live toggle.
- `check:linear` now enforces: colors only in the token file, no blur/glow/translucency,
  only `var(--shadow-card)`, retired-value rejection, ATP sha256 sync and WCAG contrast pairs.

## Audit

`docs/color-audit.md` records the pre-migration source scan: one finding, a comment in
`TitleBar.tsx` naming the close-hover color. No raw color utilities existed outside the
two old token files. Post-migration retired-value search over `app/src` returns nothing.

## Screenshots

Before and after captures for Cockpit and HUD, both themes, at 1280×800 and 1024×640:
`docs/evidence/color-migration/before/` and `docs/evidence/color-migration/after/`.
Live theme-toggle capture: `docs/evidence/color-migration/theme-toggle-light.png`.

Toggle proof: `--blue` changed from `#346AFE` (dark) to `#2563EB` (light) on click and
the rendered screen hash changed without a reload.

## Verification

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, 182 Rust tests
- `npm run lint`, `npm run typecheck`, `npm run test:security` (4 pass)
- `npm run check:linear` — PASS
- `npm run build` — PASS
- retired-value search — clean
- token sha256 — matches ATP reference

## Notes / decisions

- The dark card shadow is intentionally kept as the single allowed shadow token.
- Close-button hover keeps a white icon via a CSS brightness/invert filter, avoiding a
  literal color outside the token file.
- `--gray` remains reserved for labelled neutral bars; blue and purple are not placed
  adjacent in one chart, per the plan's accepted color-distance caveat.
