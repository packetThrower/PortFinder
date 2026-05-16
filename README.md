<p align="center">
  <img src="resources/icons/icon.png" alt="PortFinder" width="160">
</p>

<h1 align="center">PortFinder</h1>

[![CI](https://img.shields.io/github/actions/workflow/status/packetThrower/PortFinder/ci.yml?branch=main&style=flat-square&logo=github&label=CI)](https://github.com/packetThrower/PortFinder/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/packetThrower/PortFinder?style=flat-square&logo=github&label=release&include_prereleases)](https://github.com/packetThrower/PortFinder/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/packetThrower/PortFinder/total?style=flat-square&logo=github&label=downloads)](https://github.com/packetThrower/PortFinder/releases)
[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?style=flat-square&logo=rust&logoColor=white)](Cargo.toml)
[![gpui](https://img.shields.io/badge/gpui-from%20Zed-1F1F28?style=flat-square)](https://www.gpui.rs/)

## Minimum OS Versions

**macOS** (Apple Silicon and Intel)
[![macOS 11+](https://img.shields.io/badge/macOS-11%2B-333?style=flat-square&logo=apple&logoColor=white)](#requirements)
[![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-arm64-333?style=flat-square&logo=apple&logoColor=white)](#requirements)
[![Intel](https://img.shields.io/badge/Intel-x86__64-333?style=flat-square&logo=apple&logoColor=white)](#requirements)

**Windows** (x64 and ARM64)
[![Windows 10 21H2+ x64](https://img.shields.io/badge/Windows%2010%2021H2%2B-x64-0078D4?style=flat-square&logo=windows&logoColor=white)](#requirements)
[![Windows 11 ARM64](https://img.shields.io/badge/Windows%2011-ARM64-0078D4?style=flat-square&logo=windows11&logoColor=white)](#requirements)

**Linux** (amd64 and arm64)
[![Ubuntu 24.04+](https://img.shields.io/badge/Ubuntu-24.04%2B-E95420?style=flat-square&logo=ubuntu&logoColor=white)](#requirements)
[![Debian 13+](https://img.shields.io/badge/Debian-13%2B-A81D33?style=flat-square&logo=debian&logoColor=white)](#requirements)
[![Fedora 40+](https://img.shields.io/badge/Fedora-40%2B-294172?style=flat-square&logo=fedora&logoColor=white)](#requirements)
[![Arch](https://img.shields.io/badge/Arch-1793D1?style=flat-square&logo=archlinux&logoColor=white)](#requirements)

Network switch port discovery tool. Captures CDP (Cisco Discovery Protocol), LLDP (Link Layer Discovery Protocol), and MNDP (MikroTik Neighbor Discovery Protocol) packets to identify what switch, port, and VLAN your device is connected to.

📖 **Docs:** <https://packetthrower.github.io/PortFinder/> · 📝 [**Changelog**](CHANGELOG.md)

> **Heads up — 4.x is a pure-Rust rewrite.** The Tauri 2 + Svelte 5
> stack (3.x) is preserved on the `tauri-version` branch. 4.x uses
> Zed's [gpui](https://www.gpui.rs/) for the UI; the capture engine
> (CDP / LLDP / MNDP parsers, BPF helper, pcap orchestration) ports
> across unchanged.

## What it does

1. Select a network interface (or sniff all)
2. Choose protocol: CDP (Cisco), LLDP (Aruba, HP, etc.), or MNDP (MikroTik)
3. Click Start and PortFinder captures the next discovery packet
4. Displays: Switch Name, Switch IP, Switchport, Native VLAN, Voice VLAN, MTU, Switch Model

## CLI

The same binary works headless. Run with no args to launch the GUI; pass a subcommand to use the CLI.

```bash
portfinder capture --interface en0 --protocol LLDP        # capture and print
portfinder capture --json                                  # machine-readable
portfinder list --with-ip                                  # interfaces with IPs
portfinder privileges                                      # diagnose access
portfinder --help                                          # see all options
```

Press Ctrl+C to interrupt a running capture. On macOS, the BPF helper installer drops a `portfinder` symlink in `/usr/local/bin`; without it, run the binary directly: `/Applications/PortFinder.app/Contents/MacOS/PortFinder capture ...`. On Windows, `PortFinder.exe` attaches to the parent console automatically when invoked from cmd / PowerShell.

## Install

### macOS — Homebrew

```bash
brew install --cask packetThrower/tap/portfinder
```

This pulls the `.dmg` from the latest release, drops `PortFinder.app` into `/Applications`, and symlinks the headless CLI to `$(brew --prefix)/bin/portfinder`. See the [tap README](https://github.com/packetThrower/homebrew-tap) for upgrade and uninstall details. Click **Install BPF Helper** in the app once for non-sudo capture.

For early access to alpha / beta / rc builds, install the parallel `@alpha` cask alongside stable:

```bash
brew install --cask packetThrower/tap/portfinder@alpha
```

### Windows — Scoop

```powershell
scoop bucket add packetThrower https://github.com/packetThrower/scoop-bucket
scoop install portfinder
```

Installs `PortFinder.exe` and exposes `portfinder` on your `PATH`. Update with `scoop update portfinder`; uninstall with `scoop uninstall portfinder`. You'll still need [Npcap](https://npcap.com/#download) installed for packet capture.

### All platforms — release artifacts

`.dmg` + `PortFinder-BPF-*.pkg` (macOS arm64 + amd64), `.deb` / `.rpm` / `.AppImage` / `.pkg.tar.zst` (Linux amd64 + arm64), and `-setup.exe` + `.msi` (Windows x64 + ARM64) on every [release](https://github.com/packetThrower/PortFinder/releases/latest).

## Requirements

- **libpcap** (Linux: `libpcap0.8`, macOS: included, Windows: [Npcap](https://npcap.com/))
- **Elevated privileges** for packet capture:
  - macOS: click **Install BPF Helper** in the app (one-time, prompts for admin password). The installer drops the helper binary under `/Library/Application Support/PortFinder/PortFinder BPF Helper` and registers a LaunchDaemon `io.github.packetThrower.PortFinder.BPFHelper`. In macOS System Settings → General → Login Items & Extensions the entry shows as **PortFinder BPF Helper**.
  - Linux: install the `.deb` / `.rpm` / `.pkg.tar.zst` (postinstall sets `CAP_NET_RAW`), or run as root.
  - Windows: install Npcap with **Allow non-administrators to capture** enabled, or run PortFinder as Administrator.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.80+ (stable). No Node / pnpm in the build path.
- Platform-specific:
  - Linux: `libpcap-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb1-dev libxcb-randr0-dev libxcb-xkb-dev libxcb-cursor-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev libfontconfig1-dev libfreetype-dev pkg-config`
  - macOS: Xcode command-line tools + `pkg-config` (via Homebrew)
  - Windows: [Npcap SDK](https://npcap.com/) on the `LIB` path

### Run

```bash
cargo run                             # debug build + launch GUI
cargo run -- capture --protocol lldp  # CLI mode (any subcommand → headless)
cargo build --release                 # production binary at target/release/PortFinder
```

The first `cargo build` compiles gpui's full dep graph (~830 crates) and takes a few minutes. Incremental builds are fast.

### Bundle locally

[cargo-packager](https://github.com/crabnebula-dev/cargo-packager) wraps the release binary into platform installers.

```bash
cargo install cargo-packager
cargo packager --release -f app -f dmg   # macOS
cargo packager --release -f deb          # Linux
cargo packager --release -f nsis         # Windows (NSIS .exe)
```

## Versioning

[SemVer](https://semver.org/) `MAJOR.MINOR.PATCH`. Version lives in `version.txt` and is propagated to `Cargo.toml`'s `[package].version` by `scripts/bump.mjs`. Major-version history:

- `4.x` — gpui rewrite (current `main`)
- `3.x` — Tauri 2 + Svelte 5 (`tauri-version` branch)
- `2.x` — Wails 2 + Go (`wails-version` branch)
- `1.x` — original Python (`python-legacy` branch)

```bash
node scripts/bump.mjs patch    # 4.0.0 -> 4.0.1
node scripts/bump.mjs minor    # 4.0.5 -> 4.1.0
node scripts/bump.mjs major    # 4.1.4 -> 5.0.0
node scripts/tag.mjs           # git tag + push (triggers GitHub release)
```

## Tech Stack

- **Capture:** Rust + [pcap](https://crates.io/crates/pcap) (libpcap / Npcap bindings) + [Tokio](https://tokio.rs/) for async + cancellation
- **UI:** [gpui](https://www.gpui.rs/) (Zed's GPU-accelerated UI framework) + [gpui-component](https://github.com/longbridge/gpui-component)
- **Bundler:** [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) (`.dmg` / `.deb` / `.rpm` / `.AppImage` / `.pkg.tar.zst` / NSIS / WiX)

## License

[GNU General Public License v3.0 or later](LICENSE). Forks are
welcome; derivative works must stay open under the same license.
Commercial use is permitted but can't close the source.
