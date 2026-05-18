# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each major version line is a different implementation:

- **4.x** — Pure Rust + Zed gpui (current, `main`)
- **3.x** — Tauri 2 + Rust + Svelte 5 ([`tauri-version`](https://github.com/packetThrower/PortFinder/tree/tauri-version) branch)
- **2.x** — Wails 2 + Go + Svelte 5 ([`wails-version`](https://github.com/packetThrower/PortFinder/tree/wails-version) branch)
- **1.x** — Original Python implementation ([`python-legacy`](https://github.com/packetThrower/PortFinder/tree/python-legacy) branch)

## [Unreleased]

### Added
- **CLI logging flags.** `portfinder-cli` now accepts `-v` (debug
  level), `-vv` (trace), `-q` (warnings + errors only), and
  `--log-file <PATH>` (override the platform-default log path
  for this invocation). All four are global args, so they work
  alongside any subcommand.
- **Size-based log rotation.** When the debug log is enabled
  and the existing file is over 1 MiB, it's renamed to
  `portfinder.log.1` before the new session appends. Rotation
  happens at enable time (process start, UI toggle, CLI
  `--log-file`), not per-write.
- **Debug-level capture instrumentation.** `capture::run`,
  `capture_blocking`, and `open_capture` now log per-event
  diagnostics at debug + trace levels — useful for "I'm
  capturing but seeing no packets" bug reports.
- **Log-level slider** in the settings popover. Three stops —
  Normal (info), Verbose (debug), Trace — with hover-tooltip
  descriptions for each level. Applies live via
  `log::set_max_level` and persists across restarts. CLI
  `-v` / `-vv` / `-q` still override at runtime.
- **About section** in the settings popover. Version,
  Repository (GitHub), License (GPL-3.0), and a platform-
  specific capture-privilege row ("BPF helper" on macOS,
  "Npcap" on Windows, "Capture access" on Linux).
- **Start / Stop keyboard shortcut.** `Cmd+R` on macOS,
  `Ctrl+R` on Linux / Windows, scoped to the AppView so it
  doesn't fire while a popover is open. PortFinder app menu
  gains a matching "Start/Stop Capture" entry; Start and Stop
  buttons show the bound keystroke in their hover tooltips.
- **Copy as JSON button** on the result card. Right-aligned
  below the seven value rows; serializes the result with the
  same `serde_json::to_string_pretty` shape the CLI's
  `--json` flag produces. Flashes "Copied" for 1.2 s.
- **Capture history popover** next to the Copy as JSON
  button. Last 10 successful captures with relative
  timestamps ("2m ago · LLDP on en0 · Gi1/0/24") and the
  switch name + IP. Left-click restores an entry to the
  result card; right-click silently copies it as JSON.
- **Opt-in disk persistence for history.** New "Save capture
  history" Switch at the top of the settings popover (default
  off). When on, history is written to `history.json`
  alongside `settings.json` and reloaded on startup. Flipping
  it off deletes the file; the in-memory deque stays for the
  rest of the session.
- **"Settings folder" button** in the settings popover, next
  to (renamed) "Log folder". Reveals the config directory
  where `settings.json` and `history.json` live.

### Changed
- **Log levels audited.** `render: resize` events moved from
  info to debug (per-frame render-tick noise was the bulk of
  the log file). Settings-save failures and parser
  non-UTF-8-fallback warnings switched from `eprintln!` to
  `log::warn!` so they obey the in-app logger toggle instead
  of always hitting stderr.
- **Log startup banner** now emits from `set_logging_enabled`
  rather than `init_logging`, so the first line of every log
  file is the boot banner regardless of which enable path
  triggered it (persisted setting, CLI flag, UI toggle).
- **Main window is no longer resizable.** AppView already
  drives its own size via `window.resize` whenever the
  privilege banner or result card toggles state; a user-
  dragged corner used to snap back to the programmatic
  height on the next render, visible as a bounce. Disabling
  the resize handles outright (`is_resizable: false` in
  `WindowOptions`) removes the snap-back path entirely.
  Window stays movable / minimisable / closeable as normal.

### Removed
- **Stale `~/Desktop/portfinder-debug.log` from alpha installs**
  is auto-deleted on first launch of 4.0.2+ (capped at 10 MiB
  and gated to regular files only, so we don't trash anything
  unrelated that happened to land at that path).

### Fixed
- **`.deb` lintian cleanups** for Ubuntu / Debian installers
  (gdebi's "Lintian output" tab + `lintian PortFinder_*.deb`
  now report none of these):
  - `wrong-file-owner-uid-or-gid` on every payload entry —
    cargo-packager's tar preserved the GitHub runner's uid/gid
    (1001/1001) so `/usr/bin/PortFinder` installed owned by
    whatever uid 1001 happened to be on the target. The .deb
    post-process step now `chown -R 0:0` the unpacked tree
    before re-packing.
  - `malformed-contact Maintainer Will` — Maintainer field was
    a bare name, now `Will Lehnertz <...>` RFC822 form.
  - `missing-dependency-on-libc` — added `libc6` to Depends.
  - `recommended-field Section` — added `Section: net`.
  - `no-changelog` / `no-copyright-file` — package now ships
    `/usr/share/doc/portfinder/copyright` (DEP-5 machine-
    readable format) and `/usr/share/doc/portfinder/
    changelog.Debian.gz`.

## [4.0.1] - 2026-05-18

### Added
- **Opt-in debug logger** with in-app toggle. New title-bar
  hamburger menu opens a popover with a "Write debug log"
  switch + an "Open log folder" button. Default is OFF — a
  fresh install no longer drops `~/Desktop/portfinder-debug.log`
  on launch (a leftover from alpha testing). When enabled,
  logs live at the platform-conventional location:
  `~/Library/Logs/PortFinder/portfinder.log` on macOS
  (Console.app indexes it), `$XDG_STATE_HOME/portfinder/` on
  Linux, `%LOCALAPPDATA%\PortFinder\Logs\` on Windows. Toggle
  is live — no restart needed.

### Removed
- **`~/Desktop/portfinder-debug.log` is no longer written.**
  Existing files from alpha-era installs stay where they are
  (the new build just doesn't touch them); delete manually if
  you don't want them around.

## [4.0.0] - 2026-05-17

Graduates the gpui rewrite to stable. The major architectural
shift landed in `4.0.0-alpha.1` (Tauri + Svelte → pure Rust +
Zed gpui); this release captures the polish, cross-platform
fixes, and packaging work done across 16 alphas to make the
rewrite ship-ready. Per-alpha history is on the
[Releases page](https://github.com/packetThrower/PortFinder/releases).

### Added
- **Dynamic window sizing.** The gpui window auto-resizes to fit
  the current state — `~310` px tall in the idle state, growing
  for the privilege-warning banner, the result card, and the
  in-flight capture skeleton. Replaces the static `420×560` from
  alpha.1.
- **Live system theme follow.** macOS / GNOME / KDE Light↔Dark
  flips re-apply the brand palette without a relaunch via
  `observe_window_appearance`.
- **"Update available" footer pill.** Boot-time GitHub Releases
  poll surfaces a one-line pill in the window footer when a
  newer version is published. Click → opens the release page;
  ✕ → dismisses for the session. Respects channel policy
  (alpha builds see newer alphas; stable sees stable). Legacy
  calendar-style tags (`v2026.4.26` from the pre-4.x era) are
  filtered out so they don't perma-suggest as "newer".
- **Native macOS auth prompt for the BPF helper install.**
  AuthorizationServices via `security-framework` replaces the
  3.x `osascript`-shellout. Dialog shows "PortFinder wants to
  make changes" with the app icon instead of "osascript wants
  to make changes".
- **Cmd+Q and red-traffic-light both quit on macOS.** Explicit
  `QuitMode::LastWindowClosed` + a wired-up Application menu
  with a Quit action. gpui's macOS default leaves the app
  running in the Dock after the last window closes; PortFinder
  is single-window so this matches user expectation.
- **Windows console-subsystem CLI sibling.** `portfinder-cli.exe`
  ships alongside `PortFinder.exe` in the NSIS installer.
  PowerShell now waits for `portfinder-cli capture …` to exit
  and stdio routes correctly. PE subsystem byte is fixed at
  link time so a single binary can be GUI-friendly OR CLI-
  friendly but not both — two binaries is the only clean
  answer (wezterm + Zed land at the same shape).
- **AppStream metainfo on Linux.** GNOME Software now shows
  "PortFinder / packetThrower / GPL-3.0-or-later" in the
  installed-apps view instead of falling back to filename
  parsing ("PortFinder-4 / Unknown author / unknown").
- **Lowercase `/usr/bin/portfinder` symlink** on Linux
  (.deb / .rpm / pacman) so the command matches the project
  name everyone types. The capitalised `PortFinder` binary is
  kept for the .desktop / dock label / process name.
- **CDP / LLDP / MNDP test-mode reproducibility.** 20 capture-
  parser tests covering fixture frames for all three protocols.

### Changed
- **Window decorations:** `WindowDecorations::Client` requested
  via xdg-decoration so KDE Plasma's KWin doesn't draw a
  server-side title bar on top of our in-window gpui-component
  TitleBar widget. GNOME (Mutter) was already CSD-only so the
  bug was invisible there.
- **macOS .dmg builds via cross-compile** from a single Apple
  Silicon CI runner. The retired `macos-15-intel` runner had a
  small / scarce pool and slower CPU — release wall-clock cut
  by ~20 minutes per tag.
- **Linux .rpm libpcap SONAME.** `patchelf --replace-needed
  libpcap.so.0.8 libpcap.so.1` rewrites the Ubuntu-build-time
  SONAME to what Fedora / RHEL / openSUSE / Arch actually
  ship. Without this the binary couldn't load libpcap on
  Fedora — same bug existed in the 3.x .rpm but went
  unnoticed.
- **Linux launcher icon** ships once (was duplicated as both
  `PortFinder.desktop` and `portfinder.desktop` in the .deb).
- **CI matrix** retired the `macos-15-intel` runner from both
  lint/test and build. Cross-compile on `macos-26` is the
  Intel verification path now.
- **Release profile** uses `lto = "thin"` + `codegen-units = 16`
  on every target except `aarch64-pc-windows-msvc` (LTO disabled
  there — rustc thin-LTO crashes deterministically with
  `STATUS_ACCESS_VIOLATION` linking the gpui dep graph on
  Windows ARM64).

### Fixed
- **Local privilege escalation in the macOS BPF helper
  installer.** The pre-stable code wrote the bootstrap shell
  script to a predictable `/tmp/portfinder-bpf-<pid>.sh`
  using `File::create` (no `O_EXCL`, follows symlinks) and
  then ran it as root via AuthorizationServices. A local
  unprivileged user able to predict the pid could pre-symlink
  the path and race the auth-dialog Allow click to land
  arbitrary script execution as root. Switched to
  `tempfile::NamedTempFile` (`mkstemp` underneath:
  `O_CREAT | O_EXCL`, random suffix, owner-only mode). Reported
  internally during the 4.0.0 security pass.
- **Updater pill URL is allowlisted** before being passed to
  the OS URL handler. The GitHub Releases JSON's `html_url`
  field could in principle smuggle a non-`https://github.com/
  packetThrower/PortFinder/releases/` URL through to
  `cx.open_url`; any release whose `html_url` doesn't match
  the prefix is now dropped. Defence in depth — HTTPS + rustls
  rules out network MITM, but this also covers a hypothetical
  supply-chain compromise of the JSON itself.
- **Help-text `Usage:` line is consistent across platforms.**
  Without an explicit `bin_name`, clap derived the program
  name from `argv[0]`, which on Windows became
  `portfinder-cli.exe` even when the user typed `portfinder`
  through a Scoop shim. Pinned `bin_name = "portfinder"` so
  the displayed command name matches what the docs tell users
  to type.
- **Fedora `dnf install ./PortFinder-*.rpm` actually runs the
  GUI now.** Was failing with `libpcap.so.0.8: cannot open
  shared object file` regardless of whether libpcap was
  installed.
- **Ctrl+C from `portfinder capture` exits.** First press
  cancels gracefully, any subsequent press calls
  `process::exit(130)`. Without the second-press escape hatch
  a wedged blocking pcap read made the process unkillable
  short of SIGKILL.
- **CSD content area on Wayland.** Mutter reserves ~24 px for
  shadow + resize handles that aren't subtracted from
  `viewport_size()` — adding `HEIGHT_CSD_PADDING` keeps the
  version footer from clipping on GNOME.
- **Win arm64 release builds no longer require a re-run.**
  See "Release profile" above.
- **macOS .dmg actually attaches to the GitHub Release.**
  Earlier alpha CI uploaded the `macos-package` artifact
  correctly but the release job's `softprops/action-gh-release`
  step listed the old per-arch paths — silently dropped the
  macOS files. Hashes still showed up in SHA256SUMS because
  the SHA step globbed every artifact dir. Fixed in
  alpha.16's pipeline.

### Removed
- The argv-dispatch CLI path through `PortFinder.exe` on
  Windows is documented as ugly-but-still-works rather than
  the recommended CLI route. Use `portfinder-cli.exe` instead.
- The unsafe AttachConsole-without-stdio-reopen workaround is
  no longer the primary CLI path on Windows. (Modern Rust
  stdio doesn't actually cache handles per
  [rust-lang/rust#40490](https://github.com/rust-lang/rust/pull/40490)
  so the workaround was always less broken than folklore
  claimed — the real issue was the async-shell-prompt UX
  inherent to `windows_subsystem = "windows"`.)

## [4.0.0-alpha.1] - 2026-05-16

Major-version rewrite to pure Rust. Same product (still captures
CDP / LLDP / MNDP, still shows you which switch port you're on),
new stack: Zed's [gpui](https://www.gpui.rs/) replaces the Tauri 2
WebView + Svelte 5 frontend from the 3.x line. Single binary, no
Node / pnpm / Vite in the build path, smaller install footprint.

### Added
- **Pure-Rust GUI.** gpui (GPU-accelerated UI framework from Zed)
  plus [gpui-component](https://github.com/longbridge/gpui-component)
  for Button / Switch / Select / Theme. The window renders directly
  on Metal (macOS), DirectX 12 (Windows), and Vulkan + Wayland/X11
  (Linux) — no embedded browser, no IPC bridge to JS, no separate
  frontend build.
- **`PortFinder BPF Helper` is the new macOS Background Items name.**
  The 3.x helper installed itself as `ChmodBPF` under the legacy
  `coop.otec.portfinder.ChmodBPF` LaunchDaemon label, which showed
  up in System Settings → General → Login Items & Extensions as
  just "ChmodBPF". 4.x installs the helper at
  `/Library/Application Support/PortFinder/PortFinder BPF Helper`
  under the new `io.github.packetThrower.PortFinder.BPFHelper` label,
  so users see a recognisably-named entry instead of an opaque one.
  The installer (in-app button + standalone `.pkg`) unloads and
  removes the 3.x daemon on upgrade so there's no overlap.

### Changed
- **Reverse-DNS identifier scheme** for the app bundle and helper
  daemon. `com.packetthrower.portfinder` (3.x bundle) →
  `io.github.packetThrower.PortFinder`, matching what Baudrun uses.
  `coop.otec.portfinder.ChmodBPF` (3.x daemon) →
  `io.github.packetThrower.PortFinder.BPFHelper`. All `coop.otec`
  references removed.
- **Build pipeline:** `cargo build --release` produces the binary;
  [cargo-packager](https://github.com/crabnebula-dev/cargo-packager)
  wraps it into `.dmg` / `.deb` / `.rpm` / `.AppImage` / `.pkg.tar.zst`
  / NSIS / WiX bundles. Replaces the Tauri CLI's `tauri build` and
  the pnpm + Vite frontend build that fed it.
- **Linux post-install** now sets `cap_net_raw,cap_net_admin=eip`
  on the installed binary (3.x set only `cap_net_raw+ep`).
  CAP_NET_ADMIN was needed for some interface-state queries the
  `pcap` crate makes internally on newer libpcap versions.
- **Window size:** 420×560 on every platform (was 400×460 / 500 in
  3.x, with on-the-fly resize for the privilege banner). gpui's
  cross-platform resize story doesn't match Tauri's, so the new
  build sizes for the banner up front — the trade-off is ~70 px
  of dead space below the result card when the banner isn't there.

### Removed
- The 3.x `frontend/` tree (Svelte 5 + TypeScript + Vite).
- The 3.x `src-tauri/` tree (Tauri 2 + tauri-specta bindings).
- `package.json`, `pnpm-lock.yaml`, `node_modules/` at the project
  root. The remaining `docs-next/` Astro project keeps its own
  pnpm setup, but it's no longer in the application build path.
- The i18n surface (Spanish, French, German translations) — English-
  only at relaunch. Can return as Rust string tables if the
  contributor bandwidth's there.
- `install-cli.sh` / `uninstall-cli.sh` at the project root. The
  in-app BPF helper install (and the standalone `.pkg`) already
  drop the `/usr/local/bin/portfinder` symlink; the standalone
  scripts were only useful for the BPF-less install path, which
  is now rarer than it was on 3.x.

## [3.3.0-alpha.2] - 2026-05-02

Second alpha for the 3.3.0 cycle. Adds two field-tech UX moves to the
result card; design system context (`PRODUCT.md`, `DESIGN.md`,
`DESIGN.json`) lands in the repo for the first time.

### Added
- **Click-to-copy on every captured value.** Switch name, IP, port, VLAN, voice VLAN, MTU, and model each render as a chromeless inline button. Click once and the value is on your clipboard with a transient `✓` checkmark for confirmation; nothing to highlight, nothing to right-click. Saves the cmd-c step when pasting switch IPs into tickets, Slack, or a notebook. Hover and focus-visible affordances tint subtly using `color-mix(CanvasText, …)` so the styling reads correctly against any of the three per-OS card surfaces.
- **"Honest about absence" treatment.** Result fields the parsers couldn't find in the packet (voice VLAN on most non-Cisco gear, MTU on switches with LLDP-MED disabled, etc.) now render as italic-faded *not advertised* text instead of a bare em-dash. Translated in all four locales (en / es / fr / de).
- `PRODUCT.md` + `DESIGN.md` + `DESIGN.json` at the repo root, capturing the strategic + visual design system. Future agents and contributors get a consistent baseline; the impeccable tooling reads them automatically.

### Changed
- The design system documents `Settings on Every Machine` as the explicit visual North Star. Per-OS native chrome (macOS Tahoe / Windows Fluent / Linux Adwaita) is now a documented design choice, not an artifact.

## [3.3.0-alpha.1] - 2026-05-01

Pre-release for cross-platform smoke testing of the i18n work and the
batched backend / frontend infrastructure changes since 3.2.0. Not
recommended for daily use — feedback welcome before the 3.3.0 GA.

### Added
- **Localization scaffold** — every user-facing string in the GUI now flows through `svelte-i18n`. Bundles ship for English (authoritative), Spanish, French, and German; native-speaker review wanted before GA. Locale resolution: user override (footer picker, persisted in `localStorage`) → system locale via `tauri-plugin-os` → `en` fallback. Backend errors and CLI output stay English for now.
- `frontend/src/bindings.ts` is now auto-generated from the Rust IPC layer via [`tauri-specta`](https://github.com/oscartbeaumont/tauri-specta). Every `#[tauri::command]` in `src-tauri/src/lib.rs` and every `#[derive(Type)]` data struct flows through to a matching TypeScript declaration. Regenerated on every `cargo test`; a CI step fails if a contributor changed an IPC signature without committing the regenerated TS. Closes the class of bugs where `hasIp` / `switchIp` / `nativeVlan` casing drifted between Rust and TypeScript ([#16](https://github.com/packetThrower/PortFinder/issues/16)).

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

[Unreleased]: https://github.com/packetThrower/PortFinder/compare/v3.3.0-alpha.2...HEAD
[3.3.0-alpha.2]: https://github.com/packetThrower/PortFinder/compare/v3.3.0-alpha.1...v3.3.0-alpha.2
[3.3.0-alpha.1]: https://github.com/packetThrower/PortFinder/compare/v3.2.0...v3.3.0-alpha.1
[3.2.0]: https://github.com/packetThrower/PortFinder/compare/v3.1.4...v3.2.0
[3.1.4]: https://github.com/packetThrower/PortFinder/compare/v3.1.3...v3.1.4
[3.1.3]: https://github.com/packetThrower/PortFinder/compare/v3.1.2...v3.1.3
[3.1.2]: https://github.com/packetThrower/PortFinder/compare/v3.1.1...v3.1.2
[3.1.1]: https://github.com/packetThrower/PortFinder/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/packetThrower/PortFinder/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/packetThrower/PortFinder/releases/tag/v3.0.0
