# Color migration — Starship Control (SSC)

Date: 2026-11-26 · Branch: `colors/linear-palette-v2` · Plan step 5 of 6 (palette revision)

## Result

This is a **palette revision** of the earlier migration. The shared ATP token file was
updated (green/teal removed, positive → sky), and the app's primary accent is now
**Violet**, isolated in its own `app-accent.css`. The app was re-pointed at the new tokens
with no layout, spacing, radius, title-bar, window-config or behavior change — colors only.

- `app/src/styles/linear-theme.css` copied byte-for-byte from the attached ATP token file.
  `sha256 = a542c03e83fd42bff20815cac21e37182261ebb86f1ca6b04f2341b0c2b21a4e`
- `app/src/styles/app-accent.css` holds the Starship Control Violet primary and matches
  Plan §3 exactly. `sha256 = 13d47963c12a3233751ca0b628fa5fc50968a5254d84823f560da8b7d5ad4e8a`
- `app/src/tailwind.css` imports both token files and maps them with `@theme inline`
  (including `--color-primary`, `--color-primary-hover`, `--color-primary-fg`,
  `--color-primary-text`, `--color-primary-tint`). It contains no hex, rgb or hsl. Legacy
  CSS variable names used by components (`--accent-blue`, `--tint-green`, …) are aliased to
  the new role tokens, so no component layout changed.
- Primary buttons (`LinearButton`, `McpToolInspector`) use `--primary` fill with
  `--primary-fg` label in both themes.
- `echarts-linear-theme.ts` reads every color from CSS variables at runtime via
  `getComputedStyle(document.documentElement)` and exposes `getLinearSeriesOrder()` with the
  app order `--primary, --sky, --amber, --pink, --blue`. The chart re-renders on the existing
  theme-store change, so colors follow the live toggle.
- `check-linear.mjs` enforces: colors only in the two token files, no blur/glow/translucency,
  only `var(--shadow-card)`, retired-value rejection, foreign-primary rejection, green-hue
  rejection (75–170°), token + accent sha256 sync, and the Plan §8 WCAG contrast pairs.

## Token delta (old → new)

| Removed token | New role mapping | Where |
|---|---|---|
| `--blue-text` | `--primary-text` / `--sky-text` | links, focus ring, brand, active dot |
| `--green` / `--green-text` | `--sky` / `--sky-text` | success, running, healthy, positive delta |
| `--teal` | (removed) | read as green; dropped from series |
| `--chip-blue` / `--chip-green` | `--chip-sky` / `--primary-tint` | info chip / success chip |
| `--btn-primary-bg` / `--btn-primary-text` | `--primary` / `--primary-fg` | primary button |
| **Added** `--sky-text`, `--chip-sky` | — | text-safe accent, info/success chip |
| **Added** `--primary`, `--primary-hover`, `--primary-fg`, `--primary-text`, `--primary-tint` | — | per-app Violet accent (`app-accent.css`) |

## Audit

`docs/color-audit.md` records the pre-migration source scan: one finding, a comment in
`TitleBar.tsx` naming the close-hover color. No raw color utilities exist outside the token
files. Post-migration retired-value, foreign-primary and green-hue searches over `app/src`,
`DESIGN.md`, `AGENTS.md` are clean.

## Screenshots

Before (previous palette) and after (new palette) captures for Cockpit and HUD, both themes,
at 1280×800 and 1024×640: `docs/evidence/color-migration/before/` and `.../after/`.
Live theme-toggle capture: `docs/evidence/color-migration/theme-toggle-light.png`.

## Verification

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, 182 Rust tests — PASS
- `npm run lint`, `npm run typecheck` — PASS
- `npm run check:linear` — PASS (new token sha, accent sha, contrast pairs, retired/foreign/green clean)
- `npm run build` — PASS
- retired-value, foreign-primary and green-hue searches — clean
- token + accent sha256 — match references

## Notes / decisions

- The dark card shadow is intentionally kept as the single allowed shadow token.
- Close-button hover keeps a white icon via a CSS brightness/invert filter.
- `--gray` remains reserved for labelled neutral bars. Series order is the app order
  (primary, sky, amber, pink, blue); purple is skipped as a near-duplicate of the Violet primary.
- Primary button label uses `--primary-fg` (white in both themes) on `--primary`.