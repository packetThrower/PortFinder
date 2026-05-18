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
| `src/lib.rs` | Library crate. Hosts the module declarations and the shared types (`CaptureRequest`, `CaptureResult`, `InterfaceInfo`) plus `init_logging`. Both binary entry points consume this. |
| `src/main.rs` | `PortFinder` binary entry point (GUI). `windows_subsystem = "windows"` on Windows release builds. Dispatches CLI vs GUI by argv on macOS / Linux; on Windows the dispatch is preserved for `cargo run` but shipping CLI usage routes through the sibling binary below. |
| `src/bin/portfinder-cli.rs` | `portfinder-cli` binary entry point. No `windows_subsystem` attribute → console subsystem on Windows, so PowerShell waits for it and stdio routes correctly. Gated behind `[features] windows-cli` so non-Windows builds don't compile it. |
| `src/app_view.rs` | gpui UI: interface picker, protocol selector, Start / Stop, result panel, privilege banner, settings popover, history popover. Hosts the tokio runtime that drives `capture::run`. |
| `src/cli.rs` | Headless `clap`-based CLI (`capture` / `list` / `privileges` subcommands plus the global `-v` / `-vv` / `-q` / `--log-file` flags). |
| `src/capture/` | pcap capture orchestration plus hand-rolled CDP, LLDP, and MNDP TLV parsers. |
| `src/privilege/` | Per-platform privilege detection + macOS BPF helper installer (`install_darwin.rs` inlines the install script). |
| `src/settings.rs` | Persistent settings (`settings.json`) + history (`history.json`) + live logger pipe (`LogPipe`). `set_logging_enabled` swaps `RwLock<Option<File>>` so the in-app toggle takes effect without a relaunch. |
| `src/updater.rs` | Boot-time GitHub-Releases check for a newer version, behind the footer's "Update available" pill. |
| `Cargo.toml` | Root crate manifest. `[package.metadata.packager]` carries the cargo-packager bundle config (icons, identifiers, deb deps, macOS plist path). |
| `build.rs` | Embeds `resources/icons/icon.ico` into `PortFinder.exe` (Windows). Marks `wpcap.dll` as delay-loaded so the binary launches even if Npcap isn't installed. |
| `resources/Info.plist` | macOS bundle Info.plist. `CFBundleIdentifier = io.github.packetThrower.PortFinder`. |
| `resources/icons/` | `.icns` / `.ico` / `.png` consumed by cargo-packager + the Windows `.rc`. |
| `packaging/macos/` | Standalone BPF helper `.pkg` builder + scripts. `PortFinder BPF Helper.sh` is the actual helper; the matching `io.github.packetThrower.PortFinder.BPFHelper.plist` is the LaunchDaemon. |
| `packaging/linux/` | `portfinder.desktop` + post-install hook (sets `CAP_NET_RAW`) + DEP-5 `copyright` + `changelog.Debian.template` (lintian compliance). |
| `packaging/windows/` | Windows-only cargo-packager override (`Packager.json`) that adds the `portfinder-cli.exe` sibling to the NSIS / WiX bundles. See its README for the drift warning. |

## Windows dual binary

The Windows installer ships **both** `PortFinder.exe` (GUI) and `portfinder-cli.exe`. A PE binary has a single "subsystem" byte set at link time and one `.exe` has to pick one:

- `IMAGE_SUBSYSTEM_WINDOWS_GUI` (`windows_subsystem = "windows"` in `src/main.rs`): no console allocated on launch; the shell fire-and-forgets the process. Right for File Explorer double-click — no black console window flashes up next to the GUI.
- `IMAGE_SUBSYSTEM_WINDOWS_CUI` (default for `src/bin/portfinder-cli.rs`, which has no `windows_subsystem` attribute): kernel allocates a console; PowerShell waits for the process to exit before redrawing the prompt; stdio routes correctly. Right for CLI usage.

There's no runtime override. `AttachConsole` works for stdio routing but the shell has already redrawn its prompt by the time CLI output arrives. Real-world precedent: wezterm ships `wezterm` + `wezterm-gui`; Zed ships `cli` + `zed`.

Cargo wires this up via `[features] windows-cli = []` plus `required-features = ["windows-cli"]` on `[[bin]] name = "portfinder-cli"`, so non-Windows `cargo build --release` doesn't compile it. The Windows release workflow passes `--config "$(Get-Content packaging/windows/Packager.json -Raw)"` to cargo-packager, which adds the CLI to the bundle and runs `cargo build --release --features windows-cli` for the build.

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
