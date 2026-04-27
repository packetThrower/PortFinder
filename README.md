<p align="center">
  <img src="src-tauri/icons/icon.png" alt="PortFinder" width="160">
</p>

<h1 align="center">PortFinder</h1>

[![Release](https://img.shields.io/github/v/release/packetThrower/PortFinder?logo=github&color=blue)](https://github.com/packetThrower/PortFinder/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/packetThrower/PortFinder/total?logo=github&color=blue)](https://github.com/packetThrower/PortFinder/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/packetThrower/PortFinder/ci.yml?branch=main&label=CI&logo=github)](https://github.com/packetThrower/PortFinder/actions/workflows/ci.yml)
[![Release workflow](https://img.shields.io/github/actions/workflow/status/packetThrower/PortFinder/release.yml?label=release&logo=github)](https://github.com/packetThrower/PortFinder/actions/workflows/release.yml)
[![Docs](https://img.shields.io/github/actions/workflow/status/packetThrower/PortFinder/docs.yml?label=docs&logo=materialformkdocs)](https://packetthrower.github.io/PortFinder/)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey?logo=apple)](https://github.com/packetThrower/PortFinder/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-stable-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev/)

Network switch port discovery tool. Captures CDP (Cisco Discovery Protocol) and LLDP (Link Layer Discovery Protocol) packets to identify what switch, port, and VLAN your device is connected to.

📖 **Docs:** <https://packetthrower.github.io/PortFinder/> · 📝 [**Changelog**](CHANGELOG.md)

<p align="center">
  <img src="docs/assets/screenshots/macos.png" alt="PortFinder on macOS" width="420">
</p>

## What it does

1. Select a network interface (or sniff all)
2. Choose protocol: CDP (Cisco) or LLDP (Aruba, HP, etc.)
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
