# CLAUDE.md

## Project
PortFinder — Network switch port discovery tool using CDP/LLDP.
Rust backend (pcap crate) + Svelte 5/TypeScript frontend via Tauri 2.x.

## Build
All scripts live in the root `package.json`. Run with `pnpm <script>`.
```bash
pnpm install                # root deps (Tauri CLI)
pnpm install --dir frontend # frontend deps (Svelte / Vite)
pnpm tauri:dev              # hot reload, runs the app
pnpm tauri:build            # production bundles
pnpm bump                   # new day version: YYYY.M.D
pnpm bump:patch             # increment patch: YYYY.M.D-N
pnpm tag                    # git tag + push from version.txt (triggers release CI)
```
Internal scripts (called by Tauri's beforeDevCommand / beforeBuildCommand):
- `pnpm dev` / `pnpm build` — Vite dev server / Vite production build (frontend only)

## Key paths
src-tauri/src/lib.rs — Tauri command handlers + CaptureState
src-tauri/src/main.rs — binary entrypoint (dispatches GUI vs CLI by argv)
src-tauri/src/cli.rs — clap-based headless CLI (capture / list / privileges)
src-tauri/src/capture/ — pcap capture, CDP and LLDP parsers (hand-rolled)
src-tauri/src/privilege/ — platform-specific privilege detection + macOS BPF installer
src-tauri/tauri.conf.json — window/bundle/identifier config
src-tauri/Cargo.toml — Rust deps (tauri, pcap, tokio, nix, serde)
src-tauri/scripts/postinstall.sh — Linux setcap CAP_NET_RAW hook
frontend/src/App.svelte — single-component Svelte 5 UI (runes)
frontend/src/types.ts — TypeScript interfaces shared with Rust commands
frontend/src/App.css / style.css — OTEC theme with system dark/light matching
packaging/macos/ — BPF helper installer pkg, LaunchDaemon, uninstall script

## Conventions
CalVer versioning: YYYY.M.D-PATCH in version.txt; `pnpm bump`/`pnpm bump:patch`
(scripts/bump.mjs) keep src-tauri/Cargo.toml and tauri.conf.json in sync.
OTEC brand colors defined in CSS custom properties (style.css).
Tauri commands use snake_case in Rust; serde rename_all = "camelCase"
keeps the JSON wire format consistent for the frontend.
sudo on dev creates root-owned files in target/, dist/, and node_modules/
— avoid; use the BPF installer instead so capture works without sudo.

## Platform notes
- macOS capture requires the BPF helper (one-click install in the app, or
  ChmodBPF from Wireshark). Window height is 460 on macOS, 500 elsewhere.
- Linux deb/rpm runs setcap cap_net_raw+ep on /usr/bin/portfinder via
  postInstallScript so non-root capture works.
- Windows requires Npcap (downloaded at CI build time; user installs at runtime).
  CI sets LIB to the Npcap SDK path so the pcap crate links against wpcap.lib.
