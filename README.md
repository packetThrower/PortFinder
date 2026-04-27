# PortFinder

Network switch port discovery tool. Captures CDP (Cisco Discovery Protocol) and LLDP (Link Layer Discovery Protocol) packets to identify what switch, port, and VLAN your device is connected to.

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

Uses [CalVer](https://calver.org/) format `YYYY.M.D-PATCH` (e.g., `2026.4.26`, `2026.4.26-1`). Version is stored in `version.txt` and propagated to `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` by `scripts/bump.mjs`.

```bash
pnpm bump          # new day release: 2026.4.26
pnpm bump:patch    # increment patch: 2026.4.26-1, 2026.4.26-2, ...
pnpm tag           # git tag + push (triggers GitHub release)
```

## Tech Stack

- **Backend:** Rust + [pcap](https://crates.io/crates/pcap) (libpcap bindings) + [Tokio](https://tokio.rs/) for async/cancellation
- **Frontend:** [Svelte 5](https://svelte.dev/) + TypeScript + Vite
- **Desktop:** [Tauri](https://tauri.app/) v2
- **Bundler:** Tauri's built-in bundler (`.dmg`, `.deb`, `.rpm`, `.msi`)

## Branches

- `main` — current Tauri 2.x + Rust + Svelte 5 implementation
- `wails-version` — snapshot of the previous Wails 2 + Go + Svelte 5 implementation
- `react-frontend` — snapshot of the React frontend (pre-Svelte migration)
- `python-legacy` — original Python implementation
