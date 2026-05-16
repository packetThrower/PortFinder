---
title: Setup
description: Get a working dev environment in three commands.
---

PortFinder 4.x is a single-crate Rust app — no Node, no pnpm, no Vite
in the build path. The 3.x Tauri / Svelte tree is on the
[`tauri-version` branch](https://github.com/packetThrower/PortFinder/tree/tauri-version)
if that's the build you want to hack on.

## Prerequisites

- [Rust](https://rustup.rs/) 1.80+ (stable)
- Platform deps:
    - **Linux**: `libpcap-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb1-dev libxcb-randr0-dev libxcb-xkb-dev libxcb-cursor-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev libfontconfig1-dev libfreetype-dev pkg-config`
    - **macOS**: Xcode command-line tools + `pkg-config` (via Homebrew)
    - **Windows**: [Npcap SDK](https://npcap.com/#download) on the `LIB` path

## First-time setup

```bash
git clone git@github.com:packetThrower/PortFinder.git
cd PortFinder
```

That's it — Cargo fetches the rest on the first build.

## Run

```bash
cargo run                             # debug build + launch GUI
cargo run -- capture --protocol lldp  # CLI mode (any subcommand → headless)
```

The very first `cargo build` compiles gpui's full dep graph (~830
crates) and takes a few minutes. Incremental builds are fast.

## Build

```bash
cargo build --release
# binary at target/release/PortFinder
```

For platform installers, install
[cargo-packager](https://github.com/crabnebula-dev/cargo-packager) and
let it wrap the release binary:

```bash
cargo install cargo-packager
cargo packager --release -f app -f dmg   # macOS
cargo packager --release -f deb          # Linux
cargo packager --release -f nsis         # Windows (NSIS .exe)
```

The release workflow runs cargo-packager from CI; locally you only
need it if you're testing bundle output.

## Tests

```bash
cargo test
```

`cargo test` runs the in-binary unit tests in `src/capture/` and
`src/privilege/`. There's no library target, so don't pass `--lib`
— it errors with "no library targets found".
