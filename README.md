# PortFinder

Network switch port discovery tool. Captures CDP (Cisco Discovery Protocol) and LLDP (Link Layer Discovery Protocol) packets to identify what switch, port, and VLAN your device is connected to.

## What it does

1. Select a network interface (or sniff all)
2. Choose protocol: CDP (Cisco) or LLDP (Aruba, HP, etc.)
3. Click Start and PortFinder captures the next discovery packet
4. Displays: Switch Name, Switch IP, Switchport, Native VLAN, Voice VLAN, Switch Model

## Requirements

- **libpcap** (Linux: `libpcap-dev`, macOS: included, Windows: [Npcap](https://npcap.com/))
- **Elevated privileges** for packet capture:
  - Linux: install the .deb/.rpm package (postinstall sets `CAP_NET_RAW`), or run as root
  - macOS: click "Install BPF Access" in the app (one-time), or install ChmodBPF from Wireshark
  - Windows: install Npcap with "Allow non-administrators to capture" enabled

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.80+ (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/)
- [Tauri CLI](https://tauri.app/) v2: `cargo install tauri-cli --version "^2.0"`
- Platform-specific deps:
  - Linux: `libpcap-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`
  - macOS: Xcode command-line tools
  - Windows: [Npcap SDK](https://npcap.com/) on the link path

### Setup

```bash
pnpm install        # install root deps (Tauri CLI)
pnpm i              # install frontend deps
pnpm tauri:dev      # hot reload — opens the app
```

### Build

```bash
pnpm tauri:build    # produces .dmg / .deb / .rpm / .msi
```

## Versioning

Uses [CalVer](https://calver.org/) format `YYYY.M.D-PATCH` (e.g., `2026.4.15`, `2026.4.15-1`). Version is stored in `version.txt` and propagated to `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` by `scripts/bump.mjs`.

```bash
pnpm bump        # new day release: 2026.4.15
pnpm bump:patch  # increment patch: 2026.4.15-1, 2026.4.15-2, ...
pnpm tag         # git tag + push (triggers GitHub release)
```

## Tech Stack

- **Backend:** Rust + [pcap](https://crates.io/crates/pcap) crate (libpcap bindings) + [Tokio](https://tokio.rs/) for async/cancellation
- **Frontend:** [Svelte 5](https://svelte.dev/) + TypeScript + Vite
- **Desktop:** [Tauri](https://tauri.app/) v2
- **Bundler:** Tauri's built-in bundler (`.dmg`, `.deb`, `.rpm`, `.msi`)
