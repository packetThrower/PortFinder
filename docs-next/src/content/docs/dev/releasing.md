---
title: Releasing
description: SemVer, the bump scripts, and what the release workflow does.
---

Versioning follows [SemVer](https://semver.org/) `MAJOR.MINOR.PATCH`. Each major version line is a different implementation:

- **3.x** — Tauri 2 + Rust + Svelte 5 (current)
- **2.x** — Wails 2 + Go + Svelte 5 (`wails-version` branch)
- **1.x** — Python (`python-legacy` branch)

## Cut a release

```bash
# 1. Bump the version
pnpm bump          # patch (alias for bump:patch)  3.0.0 -> 3.0.1
pnpm bump:patch    # patch                          3.0.0 -> 3.0.1
pnpm bump:minor    # minor                          3.0.5 -> 3.1.0
pnpm bump:major    # major                          3.1.4 -> 4.0.0

# 2. Commit the version bump
git add -A
git commit -m "Bump to <new-version>"
git push origin main

# 3. Tag and push (triggers the release workflow)
pnpm tag
```

`scripts/bump.mjs` keeps these in sync:

- `version.txt`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- root `package.json`

## What the release workflow does

`v*` tag push → `.github/workflows/release.yml`:

1. Runs CI (fmt, clippy, test, frontend build) as a gate
2. Matrix build on `ubuntu-22.04`, `macos-latest`, `windows-latest`
3. `tauri-apps/tauri-action` builds the bundle for each platform
4. macOS additionally builds the BPF helper `.pkg`
5. All artifacts attach to a GitHub Release named `PortFinder v<version>`

## Manual checks before tagging

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --lib`
- Open the app via `pnpm tauri:dev` and capture a real packet
