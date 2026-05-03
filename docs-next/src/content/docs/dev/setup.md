---
title: Setup
description: Get a working dev environment in five commands.
---

## Prerequisites

- [Rust](https://rustup.rs/) 1.80+
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/)
- Platform deps:
    - **Linux**: `libpcap-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`
    - **macOS**: Xcode command-line tools
    - **Windows**: [Npcap SDK](https://npcap.com/#download) on the link path

## First-time setup

```bash
git clone git@github.com:packetThrower/PortFinder.git
cd PortFinder
pnpm install                    # root deps (Tauri CLI)
pnpm install --dir frontend     # frontend deps
```

## Run

```bash
pnpm tauri:dev
```

Opens the app with Vite HMR for the frontend and `cargo watch` for the Rust side.

## Build

```bash
pnpm tauri:build
```

Produces a `.dmg` / `.deb` / `.rpm` / `.AppImage` / `.exe` in `src-tauri/target/release/bundle/`.

## Tests

```bash
cd src-tauri && cargo test --lib
```
