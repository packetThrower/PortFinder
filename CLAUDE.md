# CLAUDE.md

## Project
PortFinder — Network switch port discovery tool using CDP/LLDP.
Go backend (gopacket) + Svelte 5/TypeScript frontend via Wails v2.

## Build
```bash
make i          # install frontend deps (pnpm)
make dev        # dev server with hot reload (needs sudo for capture)
make build      # production build
make bump       # new day version: YYYY.M.D
make patch      # increment patch: YYYY.M.D-N
make tag        # git tag + push from version.txt (triggers release CI)
```

## Key paths
main.go / app.go — Wails entrypoint and bound methods
backend/capture/ — gopacket packet capture (CDP/LLDP)
backend/privilege/ — platform-specific privilege detection + macOS BPF installer
frontend/src/App.svelte — single-component Svelte 5 UI (runes: $state, $derived, $effect)
frontend/src/App.css / style.css — OTEC theme with system dark/light matching
frontend/svelte.config.js — Svelte preprocessor config
packaging/macos/ — BPF helper installer, LaunchDaemon, uninstall script
packaging/ — Linux desktop entry, postinstall (CAP_NET_RAW)

## Conventions
CalVer versioning: YYYY.M.D-PATCH in version.txt
OTEC brand colors defined in CSS custom properties (style.css)
Wails bindings: update frontend/wailsjs/go/main/App.{js,d.ts} and models.ts when Go API changes
sudo wails dev creates root-owned files — avoid; use BPF installer instead
CI requires webkit2_41 build tag for Ubuntu 24.04
