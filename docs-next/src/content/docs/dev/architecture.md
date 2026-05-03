---
title: Architecture
description: How the Rust shell, the Svelte UI, and libpcap fit together.
---

```
┌──────────────────────────────────────────────────┐
│  Tauri shell (Rust)                              │
│                                                  │
│  ┌────────────────┐    invoke()    ┌───────────┐ │
│  │  WebView       │ ─────────────► │ commands  │ │
│  │  Svelte 5 UI   │ ◄───────────── │           │ │
│  └────────────────┘    JSON        └─────┬─────┘ │
│                                          │       │
│                                    ┌─────▼─────┐ │
│                                    │ capture/  │ │
│                                    │ privilege/│ │
│                                    │ cli/      │ │
│                                    └─────┬─────┘ │
└──────────────────────────────────────────┼───────┘
                                           │
                                           ▼
                                       libpcap
```

## Layout

| Path | Purpose |
|---|---|
| `src-tauri/src/lib.rs` | Tauri command handlers + capture state |
| `src-tauri/src/main.rs` | Binary entrypoint — dispatches GUI vs CLI by argv |
| `src-tauri/src/cli.rs` | Headless `clap`-based CLI |
| `src-tauri/src/capture/` | pcap capture, CDP and LLDP parsers |
| `src-tauri/src/privilege/` | Per-platform privilege detection + macOS BPF installer |
| `src-tauri/tauri.conf.json` | Window, bundle, identifier config |
| `src-tauri/Cargo.toml` | Rust deps (`tauri`, `pcap`, `tokio`, `nix`, `serde`, `clap`, `window-vibrancy`) |
| `frontend/src/App.svelte` | Single-component Svelte 5 UI (runes) |
| `frontend/src/types.ts` | TypeScript interfaces shared with Rust commands |
| `frontend/src/App.css`, `style.css` | Native-per-platform theme |
| `packaging/macos/` | BPF helper installer pkg + LaunchDaemon |

## IPC contract

Rust commands use `snake_case`; `serde rename_all = "camelCase"` keeps the JSON wire format consistent for the frontend.

| Rust command | Frontend `invoke()` | Returns |
|---|---|---|
| `get_version` | `invoke('get_version')` | `string` |
| `get_interfaces` | `invoke('get_interfaces')` | `InterfaceInfo[]` |
| `start_capture` | `invoke('start_capture', {request})` | `CaptureResult` |
| `stop_capture` | `invoke('stop_capture')` | `void` |
| `check_privileges` | `invoke('check_privileges')` | `boolean` |
| `get_privilege_status` | `invoke('get_privilege_status')` | `PrivilegeStatus` |
| `install_bpf_helper` | `invoke('install_bpf_helper')` | `void` (errors on non-macOS) |

## Capture flow

1. `start_capture` creates a fresh `CancellationToken`, replacing any prior one (which gets cancelled).
2. The token + `CaptureRequest` go into `capture::run`, which dispatches to either:
    - **Single interface**: `tokio::spawn_blocking` opens a `pcap::Capture`, sets the BPF filter, and polls `next_packet()` with a 50ms timeout. The token is checked between reads.
    - **Sniff-all**: `JoinSet` spawns one task per non-loopback interface. `tokio::select!` returns the first frame and cancels the rest.
3. The raw frame is parsed by `cdp::parse` or `lldp::parse` (hand-rolled TLV iterators) into a `CaptureResult`.
4. `stop_capture` cancels the token; the in-flight blocking task exits on its next loop tick.

## CI

| Workflow | Trigger | Output |
|---|---|---|
| `.github/workflows/ci.yml` | push / PR to `main` | `cargo fmt` / `cargo clippy -D warnings` / `cargo test` / frontend build |
| `.github/workflows/release.yml` | push of `v*` tag | matrix build (Linux / macOS universal / Windows) → GitHub Release |
| `.github/workflows/docs.yml` | push to `main` | Astro Starlight build → GitHub Pages |
