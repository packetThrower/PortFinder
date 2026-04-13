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
  - Linux: run as root, or the packaged binary has `CAP_NET_RAW` set
  - macOS: run with `sudo`, or install ChmodBPF (comes with Wireshark)
  - Windows: run as Administrator

## Development

### Prerequisites

- [Go](https://go.dev/) 1.24+
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/)
- [Wails CLI](https://wails.io/) v2: `go install github.com/wailsapp/wails/v2/cmd/wails@latest`

### Setup

```bash
make i          # install frontend dependencies
make dev        # start dev server with hot reload
```

### Build

```bash
make build      # production build
make bump       # update version to today's date (CalVer YYYY.M.D)
```

## Linux Packaging

Requires [NFPM](https://nfpm.goreleaser.com/): `go install github.com/goreleaser/nfpm/v2/cmd/nfpm@latest`

```bash
make package-deb        # .deb package
make package-rpm        # .rpm package
make package-archlinux  # .pkg.tar.zst package
make package-linux      # all three
```

## Versioning

Uses [CalVer](https://calver.org/) format `YYYY.M.D` (e.g., `2026.4.13`). Version is stored in `version.txt` and injected at build time.

## Tech Stack

- **Backend:** Go + [gopacket](https://github.com/google/gopacket) (libpcap bindings)
- **Frontend:** React + TypeScript + Vite
- **Desktop:** [Wails](https://wails.io/) v2
- **Packaging:** NFPM for Linux, Wails for macOS/Windows
