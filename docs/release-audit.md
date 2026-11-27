# Release & Packaging Audit — Starship Control

**Originally Audited:** 2026-11-24  
**Updated:** 2026-11-25 (Post-`nfpm` Migration Reconciliation)  
**Repository:** `starship-control` (`https://github.com/zvirox/starship-control`)

> **2026-11-27 Status Summary:** The unified `nfpm` packaging migration is implemented on `main`: `.github/workflows/ci.yml`, `.github/workflows/release.yml` (calling `zvirox/release-workflows/.github/workflows/linux-release.yml@v1`), `packaging/nfpm.yaml` (`.deb`, `.rpm`, Arch `.pkg.tar.zst`) and `packaging/scripts/{postinstall.sh,preremove.sh}` are in place; legacy `packaging/aur/`, `packaging/deb/`, `packaging/rpm/` and `packaging/workflows/` are removed. Workspace version is `0.1.0-beta.6` across `Cargo.toml`, `app/package.json` and `app/src-tauri/tauri.conf.json`. Local `nfpm` package generation and `SHA256SUMS` are verified in `docs/evidence/9.2.txt`; tagged-release clean-container install and GPG signature verification are task `16.6` (pending). The 2026-11-25 summary described the `feat/linear-ui` branch at `0.1.0-beta.1`.

**Release policy:** no release timeline; the maintainer decides if and when to tag and push a release. Tag/version data below is historical audit history, not a schedule.

*(Historical 2026-11-24 baseline snapshot preserved below.)*

---

## 1. Workflows in `.github/workflows/`

* **`.github/workflows/ci.yml`**:
  * **Trigger:** `push` on branches `main`, `pull_request` on `main`.
  * **Jobs:** `rust-check` (clippy, formatting, workspace tests), `frontend-check` (typecheck, lint, build).
  * **Build/Uploads:** None. Pure CI gate.
* **`.github/workflows/release.yml`**:
  * **Trigger:** `push` on tags matching `v*`.
  * **Jobs:** `build-and-release` (runs on `ubuntu-24.04`).
  * **Builds & Uploads:**
    - Tarball: `starship-control-${TAG_NAME}-x86_64-linux.tar.gz`
    - Debian: `starship-control_${VERSION}_amd64.deb` (assembled via manual `dpkg-deb`)
    - RPM: `starship-control-${VERSION}-1.x86_64.rpm` (assembled via manual `rpmbuild`)
    - `SHA256SUMS`
  * **Releases to:** GitHub Releases via `softprops/action-gh-release@v2`.

---

## 2. Existing Packaging Files & Configuration

* **`app/src-tauri/tauri.conf.json`**:
  * `"bundle": { "active": true, "targets": "all" }`
* **`packaging/` directory contents:**
  * `packaging/starship-control.desktop` (freedesktop desktop entry)
  * `packaging/icons/hicolor/{32x32,128x128,256x256,512x512}/apps/starship-control.png`
  * `packaging/deb/control`, `packaging/deb/postinst`, `packaging/deb/prerm`
  * `packaging/rpm/starship-control.spec`
  * `packaging/aur/PKGBUILD`
  * `packaging/workflows/ci.yml`, `packaging/workflows/release.yml`

---

## 3. Existing Tags and Release Assets

* **Tags:** `v0.1.0-alpha.1`, `v0.1.0-alpha.2`, `v0.1.0-alpha.3`
* **Latest Release (`v0.1.0-alpha.3`):**
  * `SHA256SUMS` (225 B)
  * `starship-control-v0.1.0-alpha.3-x86_64-linux.tar.gz` (10.33 MiB)
  * `starship-control_0.1.0.alpha.3_amd64.deb` (10.32 MiB)
  * *(RPM was attempted via rpmbuild with `|| true` and omitted on missing rpmbuild runner spec)*

---

## 4. Version Alignment Check

| Location | Version | Matches Latest Tag (`v0.1.0-alpha.3`)? |
|---|---|---|
| `Cargo.toml` (workspace) | `0.1.0` | Base semver matches (`0.1.0` vs tag `v0.1.0-alpha.3`) |
| `app/package.json` | `0.1.0` | Base semver matches |
| `app/src-tauri/tauri.conf.json` | `0.1.0` | Base semver matches |

---

## 5. Naming Matrix & Identifier Alignment

* **GitHub Repository Name:** `starship-control`
* **Cargo Binary Name:** `starship-control` (`app/src-tauri/Cargo.toml`)
* **Tauri `productName`:** `"Starship Control"` (Human-readable)
* **Tauri Identifier:** `com.starship.control`
* **Desktop File:** `packaging/starship-control.desktop`
* **Icon Name:** `starship-control.png`
* **Mismatch Assessment:** None. All identifiers and binary names match `starship-control`.

---

## 6. Runtime Dependencies & Extra Files

* **Target Binary:** `/usr/bin/starship-control` (single binary hosting Cockpit and HUD windows)
* **Desktop File:** `/usr/share/applications/starship-control.desktop`
* **Icons:** `/usr/share/icons/hicolor/<size>/apps/starship-control.png`
* **Documentation & License:** `/usr/share/doc/starship-control/LICENSE` (or `/usr/share/licenses/starship-control/`)
* **Runtime Dependencies:**
  * Debian/Ubuntu: `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`
  * Fedora/RPM: `webkit2gtk4.1`, `gtk3`, `libayatana-appindicator-gtk3`
  * Arch Linux: `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`
* **Extra daemons / units:** None. (Orchestrates external daemons over Unix domain sockets).

---

## 7. Secrets Used

* **`GITHUB_TOKEN`**: Standard repository actions token with `contents: write`.
* No external third-party registry secrets (no AUR keys, no Flatpak keys).

---

## 8. Packaging Formats & Migration Action Table

| Format | Present | Job / File Location | Action |
|---|---|---|---|
| **Debian (.deb)** | Yes | `.github/workflows/release.yml` (`dpkg-deb`) | **Replace** with unified `nfpm` workflow |
| **RPM (.rpm)** | Yes | `.github/workflows/release.yml` (`rpmbuild`) | **Replace** with unified `nfpm` workflow |
| **Arch (.pkg.tar.zst)** | Partial | `packaging/aur/PKGBUILD` (unbuilt in CI) | **Replace** with unified `nfpm` workflow |
| **Tarball (.tar.gz)** | Yes | `.github/workflows/release.yml` | **Delete** (per uniform plan: 3 packages only) |
| **AppImage** | No | N/A | **N/A** (not produced) |
| **Flatpak** | No | N/A | **N/A** (not produced) |
| **AUR Publishing** | No | `packaging/aur/` directory | **Delete** `packaging/aur/` directory |
| **Checksums** | Yes | `SHA256SUMS` in release | **Keep** & unify with signature generation |
