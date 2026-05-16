---
title: Releasing
description: SemVer, the bump scripts, and what the release workflow does.
---

Versioning follows [SemVer](https://semver.org/) `MAJOR.MINOR.PATCH`. Each major version line is a different implementation:

- **4.x** — Pure Rust + Zed gpui (current)
- **3.x** — Tauri 2 + Rust + Svelte 5 (`tauri-version` branch)
- **2.x** — Wails 2 + Go + Svelte 5 (`wails-version` branch)
- **1.x** — Python (`python-legacy` branch)

## Cut a release

```bash
# 1. Bump the version
node scripts/bump.mjs patch    # 4.0.0 -> 4.0.1
node scripts/bump.mjs minor    # 4.0.5 -> 4.1.0
node scripts/bump.mjs major    # 4.1.4 -> 5.0.0

# 2. Commit the version bump
git add -A
git commit -m "Bump to <new-version>"
git push origin main

# 3. Tag and push (triggers the release workflow)
node scripts/tag.mjs
```

`scripts/bump.mjs` keeps these in sync:

- `version.txt`
- `Cargo.toml` (`[package].version`)

The release workflow rewrites the bundled `Info.plist`'s `CFBundleShortVersionString` / `CFBundleVersion` from the tag at build time, so Info.plist isn't part of the bump set.

## What the release workflow does

`v*` tag push → `.github/workflows/release.yml`:

1. Runs CI (`cargo clippy -D warnings` + `cargo test`) on every supported host as a gate.
2. Matrix build:
   - macOS: arm64 (`macos-26`) + amd64 (`macos-15-intel`)
   - Windows: amd64 (`windows-latest`) + arm64 (`windows-11-arm`)
   - Linux: amd64 (`ubuntu-latest`) + arm64 (`ubuntu-24.04-arm`)
3. Each runner rewrites `Cargo.toml`'s version (and on macOS, `Info.plist`'s `CFBundleShortVersionString` / `CFBundleVersion`) from the tag.
4. [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) bundles the release binary into the platform's native installers (`.dmg` + `.app.zip`, `.deb`, `.AppImage`, NSIS `.exe`, WiX `.msi` on stable amd64).
5. Linux additionally builds `.rpm` and `.pkg.tar.zst` via [fpm](https://fpm.readthedocs.io/) using the same staging directory. Both share the same post-install hook (`packaging/linux/portfinder-postinstall.sh`) that sets `CAP_NET_RAW` on the installed binary.
6. macOS additionally runs `packaging/macos/build-pkg.sh` to produce `PortFinder-BPF-<version>.pkg`, the standalone BPF helper installer.
7. A `SHA256SUMS` file is generated over every artifact and attached to the release.
8. All artifacts attach to a GitHub Release with auto-generated notes (commit log between the previous tag and this one). Tags with a hyphen in the SemVer pre-release slot (`v4.0.0-alpha.1`) flag as pre-release.

## Manual checks before tagging

- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo run` and capture a real packet on at least one platform
