# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each major version line is a different implementation:

- **3.x** — Tauri 2 + Rust + Svelte 5 (current, `main`)
- **2.x** — Wails 2 + Go + Svelte 5 ([`wails-version`](https://github.com/packetThrower/PortFinder/tree/wails-version) branch)
- **1.x** — Original Python implementation ([`python-legacy`](https://github.com/packetThrower/PortFinder/tree/python-legacy) branch)

## [Unreleased]

### Changed
- CDP / LLDP / MNDP TLV decoding now logs a one-line warning to stderr when a string TLV (system name, port description, board name, etc.) contains non-UTF-8 bytes that have to be substituted. The displayed result is unchanged — flaky-firmware diagnosis used to be silent ([#24](https://github.com/packetThrower/PortFinder/issues/24)).
- `list_interfaces()` (the `get_interfaces` Tauri command and `portfinder list`) now caches results for 5 seconds. Tabs-away-and-back / fast double-clicks on the refresh button no longer re-scan via libpcap, which on slow hosts saves a few hundred milliseconds per call ([#23](https://github.com/packetThrower/PortFinder/issues/23)).
- Stop button now responds within ~60 ms instead of up to ~510 ms. The pcap blocking timeout dropped from 500 ms to 50 ms, so the capture loop checks the cancellation token an order of magnitude more often. Sniff-all mode also tears down the losing interface tasks 10× faster after a winner is found ([#18](https://github.com/packetThrower/PortFinder/issues/18)).

## [3.2.0] - 2026-05-01

### Added
- **MNDP** (MikroTik Neighbor Discovery Protocol) as a third capture protocol alongside CDP and LLDP. RouterOS devices that ship with `discovery-protocol = mndp` and never speak LLDP now show up with switch name, sender-side interface, board model, and IP populated. UDP/5678 broadcast; TLV parser handles Identity / Platform / Board / Interface / IPv4 / IPv6 ([#26](https://github.com/packetThrower/PortFinder/issues/26)).

### Changed
- Resized the RJ-45 jack on the app icon to match the padding on Baudrun's serial-port icon (610×392 box, ~207 px L/R padding instead of filling the canvas). Side-by-side the two apps now read as siblings on the dock / launcher / app grid.

## [3.1.4] - 2026-04-27

### Fixed
- Windows CLI printing two stray `System error 1376 has occurred. / The specified local group does not exist.` blocks before the capture result. The `net localgroup Npcap` probe used to detect non-admin Npcap installs now silences its stdout/stderr (and uses `CREATE_NO_WINDOW` so the GUI build doesn't flash a console window).

## [3.1.3] - 2026-04-27

### Changed
- LLDP capture results now combine Port ID and Port Description when both are present (`"1/1/1 (Duty PC)"`). Previously only the description was shown when both existed, which hid the actual port number.

### Fixed
- Windows ARM64 release build failing in CI: `pnpm/action-setup@v6` writes broken `.cmd.EXE` symlinks on `windows-11-arm` runners. Pinned both workflows back to `pnpm/action-setup@v4` until the upstream regression is resolved.

## [3.1.2] - 2026-04-27

### Fixed
- Windows binary refusing to launch when Npcap isn't installed. The OS dialog `wpcap.dll was not found` appeared before the privilege-warning banner had a chance to render and direct users to the Npcap download. `wpcap.dll` is now marked as a delay-loaded import via `build.rs`, and pcap calls are gated on `npcap_installed` so the app launches cleanly and shows the Download Npcap link instead.

### Fixed
- Linux ARM64 release build failing with `xdg-open binary not found` — `ubuntu-24.04-arm` runners ship a more minimal package set than `ubuntu-latest` and don't include `xdg-utils` by default. Added it to the apt install step.

## [3.1.0] - 2026-04-26

### Added
- Linux ARM64 release artifacts: `_arm64.deb`, `_aarch64.AppImage`, `.aarch64.rpm`.
- Windows ARM64 release artifact: `_arm64-setup.exe`.
- macOS Liquid Glass effect: `NSVisualEffectView` vibrancy material applied to the window via the `window-vibrancy` crate.
- Per-OS card layouts that adopt each platform's surface conventions (macOS Tahoe, Windows 11 Fluent, GNOME Adwaita).
- iOS-style toggle switch for the "Only show interfaces with an IP" control.
- Refresh button next to the interface dropdown.
- `MTU` field in the captured-result display, parsed from LLDP's IEEE 802.3 Maximum Frame Size TLV.
- `Switch Model` populated for LLDP from the System Description TLV (CDP already had it via Platform).
- `install-cli.sh` and `uninstall-cli.sh` standalone scripts at the repo root for users who want only the `portfinder` CLI symlink without the BPF helper.
- BPF helper installer (and the in-app *Install BPF Access* button) now creates `/usr/local/bin/portfinder` automatically so the CLI is callable from any shell.
- New app icon: navy gradient with a metallic RJ-45 jack (Baudrun-style aesthetic).
- MkDocs documentation site at <https://packetthrower.github.io/PortFinder/>, deployed automatically on push to main.
- Status / tech badges in the README (release, downloads, CI, docs, platforms, Tauri/Rust/Svelte).
- macOS screenshot embedded in the README.
- Branch protection on `main` with a CI rollup status check that lets docs-only PRs through while gating code changes on the full build.

### Changed
- Native widget styling per platform — fonts, spacing, radii, button shapes track macOS / Windows / Linux native conventions.
- Window resized to 400×460 to fit the tightened layout.
- Card backgrounds inverted to gray-on-white in light mode (matching macOS Settings' surface convention).
- Protocol selector is now a dropdown matching the interface dropdown style (was a segmented control).

### Fixed
- Wi-Fi capture on macOS: `BIOCPROMISC: Operation not supported` now triggers a fall-back to non-promiscuous mode instead of erroring out.
- TypeScript field casing matched to Rust's serde camelCase output (`hasIp`, `switchIp`, `nativeVlan`, `voiceVlan`) — previously the IP filter checkbox silently failed and capture-result fields appeared empty.

## [3.0.0] - 2026-04-26

### Changed
- **Complete rewrite from Wails 2 + Go to Tauri 2 + Rust.** Frontend (Svelte 5 + TypeScript + Vite) is unchanged.
- Versioning switched from CalVer (`YYYY.M.D-PATCH`) to SemVer.
- Build system migrated from `Makefile` + NFPM + Wails CLI to `package.json` scripts + Tauri's bundler.

### Added
- Headless CLI mode — the same binary runs as a GUI when launched without args, or as a CLI with subcommands (`capture`, `list`, `privileges`) when given any. Each supports `--json` for scripting.
- Hand-rolled CDP and LLDP parsers in Rust.
- Tokio-based capture cancellation with race-to-first across interfaces in sniff-all mode.
- macOS BPF helper installer rewritten in Rust (in-app *Install BPF Access* button, plus the standalone `PortFinder-BPF-*.pkg`).
- Per-platform privilege detection in Rust (`/proc/self/status` on Linux, `/dev/bpf0` + `dseditgroup` on macOS, Npcap registry + admin check on Windows).
- macOS universal binary — the `.dmg` runs on Apple Silicon and Intel from a single download.

## 2.x — Wails 2 + Go

The `2.x` line lived as `2026.4.x[-PATCH]` CalVer tags. Highlights, in rough order:

- Migrated frontend from React to Svelte 5.
- Added IP-only interface filter.
- Added platform-specific window sizing.
- Added macOS BPF helper installer (Go).
- Added Linux `setcap CAP_NET_RAW` postinstall via NFPM.
- Added Windows Npcap detection.
- Reformatted release artifacts to include the version in their filenames.

Detailed history and source on the [`wails-version`](https://github.com/packetThrower/PortFinder/tree/wails-version) branch and its tags.

## 1.x — Python

- `v0.2.0` (2021-01-18) — Windows EXE and Linux binary
- `v0.1.0` (2021-01-07) — Initial Python script

Source on the [`python-legacy`](https://github.com/packetThrower/PortFinder/tree/python-legacy) branch.

[Unreleased]: https://github.com/packetThrower/PortFinder/compare/v3.2.0...HEAD
[3.2.0]: https://github.com/packetThrower/PortFinder/compare/v3.1.4...v3.2.0
[3.1.4]: https://github.com/packetThrower/PortFinder/compare/v3.1.3...v3.1.4
[3.1.3]: https://github.com/packetThrower/PortFinder/compare/v3.1.2...v3.1.3
[3.1.2]: https://github.com/packetThrower/PortFinder/compare/v3.1.1...v3.1.2
[3.1.1]: https://github.com/packetThrower/PortFinder/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/packetThrower/PortFinder/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/packetThrower/PortFinder/releases/tag/v3.0.0
