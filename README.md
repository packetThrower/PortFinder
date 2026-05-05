<p align="center">
  <img src="src-tauri/icons/icon.png" alt="PortFinder" width="160">
</p>

<h1 align="center">PortFinder</h1>

[![CI](https://img.shields.io/github/actions/workflow/status/packetThrower/PortFinder/ci.yml?branch=main&style=flat-square&logo=github&label=CI)](https://github.com/packetThrower/PortFinder/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/packetThrower/PortFinder?style=flat-square&logo=github&label=release&include_prereleases)](https://github.com/packetThrower/PortFinder/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/packetThrower/PortFinder/total?style=flat-square&logo=github&label=downloads)](https://github.com/packetThrower/PortFinder/releases)
[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?style=flat-square&logo=rust&logoColor=white)](src-tauri/Cargo.toml)
[![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?style=flat-square&logo=tauri&logoColor=black)](https://tauri.app)
[![Svelte](https://img.shields.io/github/package-json/dependency-version/packetThrower/PortFinder/dev/svelte?filename=frontend%2Fpackage.json&style=flat-square&logo=svelte&logoColor=white&label=Svelte&color=FF3E00)](https://svelte.dev)

## Minimum OS Versions

**macOS** (Apple Silicon and Intel)  
[![macOS 11+](https://img.shields.io/badge/macOS-11%2B-333?style=flat-square&logo=apple&logoColor=white)](#requirements)
[![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-arm64-333?style=flat-square&logo=apple&logoColor=white)](#requirements)
[![Intel](https://img.shields.io/badge/Intel-x86__64-333?style=flat-square&logo=apple&logoColor=white)](#requirements)

**Windows** (x64 and ARM64)  
[![Windows 10 21H2+ x64](https://img.shields.io/badge/Windows%2010%2021H2%2B-x64-0078D4?style=flat-square&logo=windows&logoColor=white)](#requirements)
[![Windows 11 x64](https://img.shields.io/badge/Windows%2011-x64-0078D4?style=flat-square&logo=windows11&logoColor=white)](#requirements)
[![Windows 11 ARM64](https://img.shields.io/badge/Windows%2011-ARM64-0078D4?style=flat-square&logo=windows11&logoColor=white)](#requirements)

**Linux** (amd64 and arm64)  
[![Ubuntu 24.04+](https://img.shields.io/badge/Ubuntu-24.04%2B-E95420?style=flat-square&logo=ubuntu&logoColor=white)](#requirements)
[![Debian 13+](https://img.shields.io/badge/Debian-13%2B-A81D33?style=flat-square&logo=debian&logoColor=white)](#requirements)
[![Fedora 40+](https://img.shields.io/badge/Fedora-40%2B-294172?style=flat-square&logo=fedora&logoColor=white)](#requirements)
[![Arch](https://img.shields.io/badge/Arch-1793D1?style=flat-square&logo=archlinux&logoColor=white)](#requirements)
[![openSUSE Tumbleweed](https://img.shields.io/badge/openSUSE-Tumbleweed-73BA25?style=flat-square&logo=opensuse&logoColor=white)](#requirements)
[![AppImage: libwebkit2gtk-4.1 + FUSE](https://img.shields.io/badge/AppImage-libwebkit2gtk--4.1%20%2B%20FUSE-2166B7?style=flat-square&logo=appimage&logoColor=white)](#requirements)

Network switch port discovery tool. Captures CDP (Cisco Discovery Protocol), LLDP (Link Layer Discovery Protocol), and MNDP (MikroTik Neighbor Discovery Protocol) packets to identify what switch, port, and VLAN your device is connected to.

📖 **Docs:** <https://packetthrower.github.io/PortFinder/> · 📝 [**Changelog**](CHANGELOG.md)

<p align="center">
  <img src="docs/assets/screenshots/macos.png" alt="PortFinder on macOS" width="420">
</p>

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

Press Ctrl+C to interrupt a running capture. On macOS, run the binary directly: `/Applications/PortFinder.app/Contents/MacOS/portfinder capture ...`. On Windows, `PortFinder.exe` attaches to the parent console automatically when invoked from cmd / PowerShell.

To get `portfinder` on your `PATH` on macOS without installing the BPF helper:

```bash
sudo ./install-cli.sh      # symlinks /usr/local/bin/portfinder → app bundle
sudo ./uninstall-cli.sh    # removes the symlink
```

The BPF helper installer (in-app *Install BPF Access* button or `PortFinder-BPF-*.pkg`) creates the same symlink for you, so you only need these scripts if you're keeping things minimal.

## Install

### macOS — Homebrew

```bash
brew install --cask packetThrower/tap/portfinder
```

This pulls the universal `.dmg` from the latest release, drops `PortFinder.app` into `/Applications`, and symlinks the headless CLI to `$(brew --prefix)/bin/portfinder`. See the [tap README](https://github.com/packetThrower/homebrew-tap) for upgrade and uninstall details. Click **Install BPF Access** in the app once for non-sudo capture.

For early access to alpha / beta / rc builds, install the parallel `@alpha` cask alongside stable:

```bash
brew install --cask packetThrower/tap/portfinder@alpha
```

`PortFinder Alpha.app` and `portfinder-alpha` coexist with the stable install.

### Windows — Scoop

```powershell
scoop bucket add packetThrower https://github.com/packetThrower/scoop-bucket
scoop install portfinder
```

Installs `PortFinder.exe` and exposes `portfinder` on your `PATH`. Update with `scoop update portfinder`; uninstall with `scoop uninstall portfinder`. See the [bucket README](https://github.com/packetThrower/scoop-bucket) for details. You'll still need [Npcap](https://npcap.com/#download) installed for packet capture.

For pre-release builds:

```powershell
scoop install portfinder-prerelease
```

CLI shim is `portfinder-alpha` and the Start menu shortcut is `PortFinder Alpha`. Coexists with the stable install.

### All platforms — release artifacts

`.dmg` (macOS), `.deb` / `.rpm` / `.AppImage` (Linux amd64 + arm64), and `-setup.exe` (Windows x64 + ARM64) on every [release](https://github.com/packetThrower/PortFinder/releases/latest).

## Requirements

- **libpcap** (Linux: `libpcap-dev`, macOS: included, Windows: [Npcap](https://npcap.com/))
- **Elevated privileges** for packet capture:
  - Linux: install the `.deb` / `.rpm` package (postinstall sets `CAP_NET_RAW`), or run as root
  - macOS: click "Install BPF Access" in the app (one-time), or install ChmodBPF from Wireshark
  - Windows: install Npcap with "Allow non-administrators to capture" enabled

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.80+ (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/)
- Platform-specific deps:
  - Linux: `libpcap-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`
  - macOS: Xcode command-line tools
  - Windows: [Npcap SDK](https://npcap.com/) on the link path

The Tauri CLI ships as a project devDep — no global install needed.

### Setup

```bash
pnpm install                   # root deps (Tauri CLI)
pnpm install --dir frontend    # frontend deps (Svelte / Vite)
pnpm tauri:dev                 # hot reload — opens the app
```

### Build

```bash
pnpm tauri:build               # produces .dmg / .deb / .rpm / .msi
```

## Versioning

[SemVer](https://semver.org/) `MAJOR.MINOR.PATCH`. The current `3.x` line is the Rust + Tauri rewrite; the previous Go + Wails line was `2.x` (see `wails-version` branch) and the original Python implementation was `1.x` (see `python-legacy`). Version lives in `version.txt` and is propagated to `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and root `package.json` by `scripts/bump.mjs`.

```bash
pnpm bump          # patch (alias for bump:patch): 3.0.0 -> 3.0.1
pnpm bump:patch    # patch:                        3.0.0 -> 3.0.1
pnpm bump:minor    # minor:                        3.0.5 -> 3.1.0
pnpm bump:major    # major:                        3.1.4 -> 4.0.0
pnpm tag           # git tag + push (triggers GitHub release)
```

## Tech Stack

- **Backend:** Rust + [pcap](https://crates.io/crates/pcap) (libpcap bindings) + [Tokio](https://tokio.rs/) for async/cancellation
- **Frontend:** [Svelte 5](https://svelte.dev/) + TypeScript + Vite
- **Desktop:** [Tauri](https://tauri.app/) v2
- **Bundler:** Tauri's built-in bundler (`.dmg`, `.deb`, `.rpm`, `.msi`)

## Branches

- `main` — current `3.x` line: Tauri 2 + Rust + Svelte 5
- `wails-version` — `2.x` line: Wails 2 + Go + Svelte 5
- `react-frontend` — snapshot of the React frontend (pre-Svelte migration on the `2.x` line)
- `python-legacy` — `1.x` line: original Python implementation

## License

[GNU General Public License v3.0 or later](LICENSE). Forks are
welcome; derivative works must stay open under the same license.
Commercial use is permitted but can't close the source.
