---
title: Architecture
description: How the Rust binary, gpui, and libpcap fit together.
---

```
┌──────────────────────────────────────────────────┐
│  PortFinder binary (Rust, single-crate)          │
│                                                  │
│  ┌────────────────┐                 ┌──────────┐ │
│  │  gpui window   │ ── flume chan ─►│  tokio   │ │
│  │  (app_view.rs) │ ◄── results ────│ runtime  │ │
│  └────────────────┘                 └────┬─────┘ │
│           ▲                              │       │
│           │ argv (no subcommand)         │       │
│  ┌────────┴────────┐                ┌────▼─────┐ │
│  │   main.rs       │                │ capture/ │ │
│  │  CLI ↔ GUI      │                │ pcap orch│ │
│  │  dispatch       │                └────┬─────┘ │
│  └────────┬────────┘                     │       │
│           │ argv (subcommand)            │       │
│  ┌────────▼────────┐                ┌────▼─────┐ │
│  │   cli.rs        │                │ parsers: │ │
│  │  clap headless  │                │ cdp/lldp │ │
│  └─────────────────┘                │ /mndp    │ │
│                                     └────┬─────┘ │
│                                          │       │
│                                     ┌────▼─────┐ │
│                                     │privilege/│ │
│                                     │BPF helper│ │
│                                     │installer │ │
│                                     └────┬─────┘ │
└──────────────────────────────────────────┼───────┘
                                           │
                                           ▼
                            libpcap / Npcap (wpcap.dll)
```

PortFinder 4.x is a single Rust crate. The GUI (gpui) and the headless
CLI (`portfinder capture …`) share one binary — `main.rs` decides
which path to take based on whether argv carries a subcommand. The
capture engine, parsers, and BPF privilege flow are identical along
both paths.

## Layout

| Path | Purpose |
|---|---|
| `src/main.rs` | Binary entrypoint. Declares modules, defines `CaptureRequest` / `CaptureResult` / `InterfaceInfo`, dispatches CLI vs GUI by argv. |
| `src/app_view.rs` | gpui UI: interface picker, protocol selector, Start / Stop, result panel, privilege banner. Hosts the tokio runtime that drives `capture::run`. |
| `src/cli.rs` | Headless `clap`-based CLI (`capture` / `list` / `privileges` subcommands). |
| `src/capture/` | pcap capture orchestration plus hand-rolled CDP, LLDP, and MNDP TLV parsers. |
| `src/privilege/` | Per-platform privilege detection + macOS BPF helper installer (`install_darwin.rs` inlines the install script). |
| `Cargo.toml` | Root crate manifest. `[package.metadata.packager]` carries the cargo-packager bundle config (icons, identifiers, deb deps, macOS plist path). |
| `build.rs` | Embeds `resources/icons/icon.ico` into `PortFinder.exe` (Windows). Marks `wpcap.dll` as delay-loaded so the binary launches even if Npcap isn't installed. |
| `resources/Info.plist` | macOS bundle Info.plist. `CFBundleIdentifier = io.github.packetThrower.PortFinder`. |
| `resources/icons/` | `.icns` / `.ico` / `.png` consumed by cargo-packager + the Windows `.rc`. |
| `packaging/macos/` | Standalone BPF helper `.pkg` builder + scripts. `PortFinder BPF Helper.sh` is the actual helper; the matching `io.github.packetThrower.PortFinder.BPFHelper.plist` is the LaunchDaemon. |
| `packaging/linux/` | `portfinder.desktop` + post-install hook (sets `CAP_NET_RAW`). |

## Capture flow

1. The user clicks **Start** in the gpui window (or runs `portfinder capture …`). Both paths build a `CaptureRequest` with the interface name + protocol string.
2. In the GUI, the request is dispatched onto a dedicated tokio runtime running on a background OS thread; the gpui task awaits a [`flume`](https://crates.io/crates/flume) channel for the result. In the CLI, the same `capture::run` future runs on a fresh `tokio::runtime::Runtime` synchronously via `block_on`.
3. `capture::run` creates a fresh `CancellationToken` (replacing any previous one, which gets cancelled) and dispatches to either:
    - **Single interface**: `tokio::task::spawn_blocking` opens a `pcap::Capture`, sets the BPF filter, and polls `next_packet()` with a 50 ms timeout. The token is checked between reads, so Stop responds within ~60 ms.
    - **Sniff-all** (`interface_name == ""`): `JoinSet` spawns one task per non-loopback interface. `tokio::select!` returns the first captured frame and cancels the rest.
4. The raw frame is parsed by `cdp::parse`, `lldp::parse`, or `mndp::parse` (hand-rolled TLV iterators) into a `CaptureResult`.
5. **Stop** cancels the token; the in-flight blocking task exits on its next loop tick. The GUI receives the resulting `Err("capture cancelled")` over the flume channel and renders "Stopped" in the status row.

## Privilege flow

macOS:
- `privilege::get_privilege_status()` probes `/dev/bpf0` (readable when the user is in the `access_bpf` group) and checks for the LaunchDaemon plist on disk.
- The 4.x label is `io.github.packetThrower.PortFinder.BPFHelper`. The legacy 3.x label `coop.otec.portfinder.ChmodBPF` is also probed so an existing 3.x install registers as "helper installed" until the user re-runs the new installer (which then unloads + removes the 3.x daemon).
- **Install BPF Helper** in the GUI runs `install_darwin.rs` — an inlined shell script invoked via `osascript ... with administrator privileges`. The standalone `PortFinder-BPF-<version>.pkg` (built by `packaging/macos/build-pkg.sh`, shipped on every release) performs the same install for sysadmins deploying via MDM.

Linux: capture works if the user is root or the binary has `CAP_NET_RAW` (`CapEff` in `/proc/self/status`). The `.deb` / `.rpm` / `.pkg.tar.zst` post-install runs `setcap cap_net_raw,cap_net_admin=eip` on `/usr/bin/PortFinder`.

Windows: `pcap::Device::list()` would crash the process if `wpcap.dll` isn't present; `build.rs` marks the DLL as delay-loaded so the binary launches anyway, and `privilege::get_privilege_status()` reports `npcap_installed = false` to drive the in-app "Download Npcap" banner.

## CI

| Workflow | Trigger | Output |
|---|---|---|
| `.github/workflows/ci.yml` | push / PR to `main` | `cargo clippy -D warnings` + `cargo test` on macOS arm64+amd64, Windows amd64+arm64, Linux amd64+arm64 |
| `.github/workflows/release.yml` | push of `v*` tag | cargo-packager builds `.dmg` / `.deb` / `.rpm` / `.AppImage` / `.pkg.tar.zst` / NSIS / WiX → GitHub Release |
| `.github/workflows/docs.yml` | push to `main` | Astro Starlight build → GitHub Pages |
