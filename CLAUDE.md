# CLAUDE.md

## Project
PortFinder — Network switch port discovery tool using CDP / LLDP / MNDP.
Pure-Rust single-binary app: Zed's gpui (GPU-accelerated UI), gpui-component
(Button / Switch / Select / Theme), and the pcap crate. 4.x is the gpui
rewrite; 3.x was Tauri (Rust backend + Svelte 5 frontend) on the
`tauri-version` branch, 2.x was Wails+Go on `wails-version`, 1.x was Python
on `python-legacy`.

## Build
Cargo-driven, no Node/pnpm in the build path. The release pipeline uses
cargo-packager to wrap the binary into platform bundles.
```bash
cargo run                   # debug build + launch GUI
cargo build --release       # production binary at target/release/PortFinder
cargo run -- capture --protocol lldp   # CLI mode (any subcommand → headless)
cargo packager --release -f app -f dmg # local macOS bundle (needs cargo-packager)

node scripts/bump.mjs patch # SemVer patch (4.0.0 -> 4.0.1)
node scripts/bump.mjs minor # minor (4.0.5 -> 4.1.0)
node scripts/bump.mjs major # major (4.1.4 -> 5.0.0)
node scripts/tag.mjs        # git tag + push from version.txt (triggers release CI)
```
The bump script writes `version.txt` and `Cargo.toml`'s `[package].version`
in lockstep. The release workflow rewrites the bundled `Info.plist`'s
`CFBundleShortVersionString` / `CFBundleVersion` from the tag at build
time, so Info.plist isn't part of the bump set.

## Docs
Astro Starlight deployed to GitHub Pages on every push to main.
- `docs-next/` — Starlight site (its own pnpm project, separate from the
  Rust crate)
- `docs-next/astro.config.mjs` — site config (title, sidebar, custom Hero
  override)
- `docs-next/src/content/docs/` — page content (index, install, usage, dev,
  404)
- `docs-next/src/components/Hero.astro` — drenched-navy landing hero
- `docs-next/src/styles/theme.css` — palette + typography (Fraunces / IBM Plex)
- `docs-next/scripts/sync-changelog.mjs` — copies repo-root CHANGELOG.md
  into the site on predev/prebuild
- `.github/workflows/docs.yml` — Pages build + deploy
- Live at https://packetthrower.github.io/PortFinder/
- Preview locally: `pnpm --dir docs-next install && pnpm --dir docs-next dev`

## Key paths
src/main.rs — binary entrypoint; declares modules; dispatches CLI vs GUI
              by argv; defines the shared data types (CaptureRequest,
              CaptureResult, InterfaceInfo).
src/app_view.rs — gpui UI: interface picker, protocol selector, Start/Stop,
                  result panel, privilege-warning banner. Bridges to the
                  capture module via a tokio runtime on a dedicated OS
                  thread and a flume channel.
src/cli.rs — clap-based headless CLI (capture / list / privileges).
src/capture/ — pcap capture orchestration plus hand-rolled CDP, LLDP, MNDP
               parsers. Ports across from 3.x unchanged.
src/privilege/ — platform-specific privilege detection + macOS BPF helper
                 installer. `install_darwin.rs` inlines the install script
                 that's also shipped as a standalone .pkg via
                 `packaging/macos/build-pkg.sh`.
Cargo.toml — root crate manifest with cargo-packager metadata (bundle ids,
             icons, deb dependencies, macOS plist path).
build.rs — embeds resources/icons/icon.ico into PortFinder.exe and tells
           the linker to delay-load wpcap.dll on Windows.
resources/Info.plist — macOS bundle Info.plist (CFBundleIdentifier
                       `io.github.packetThrower.PortFinder`).
resources/icons/ — icon.icns / .ico / .png used by cargo-packager + the
                   Windows .rc.
resources/icon.rc — Windows resource script that embeds icon.ico into the
                    PE resource section.
packaging/macos/ — BPF helper bits. `PortFinder BPF Helper.sh` is the
                   actual helper script (filename is what shows in macOS
                   Background Items); the matching LaunchDaemon plist is
                   `io.github.packetThrower.PortFinder.BPFHelper.plist`.
                   `build-pkg.sh` produces the standalone BPF .pkg.
packaging/linux/ — `portfinder.desktop` + `portfinder-postinstall.sh`
                   (sets CAP_NET_RAW on the installed binary).

## Conventions
SemVer (MAJOR.MINOR.PATCH) in version.txt. `scripts/bump.mjs` keeps
version.txt and Cargo.toml in sync; nothing else needs updating on bump.
Reverse-DNS identifier convention: `io.github.packetThrower.PortFinder`
for the app bundle, `io.github.packetThrower.PortFinder.BPFHelper` for the
LaunchDaemon. Replaces the legacy `coop.otec.portfinder.ChmodBPF` /
`com.packetthrower.portfinder` identifiers from 3.x.
Binary name in Cargo (`[[bin]] name = "PortFinder"`) is capitalised to
match CFBundleExecutable; the package name stays lowercase as is
conventional.
gpui-component widgets need `gpui_component::init(cx)` called before the
first render or `Select::new` panics on the missing Theme global.

## Platform notes
- macOS capture requires the BPF helper. One-click install in the app
  (osascript prompt), or the standalone `PortFinder-BPF-<version>.pkg`
  from the release assets. macOS Background Items shows the helper as
  "PortFinder BPF Helper" (the filename of the installed binary). The
  installer cleans up the legacy 3.x `coop.otec.portfinder.ChmodBPF`
  daemon if present.
- Linux .deb / .rpm / pacman packages run `setcap cap_net_raw,cap_net_admin=eip`
  on `/usr/bin/PortFinder` so non-root capture works.
- Windows requires Npcap (downloaded by CI at build time for linking;
  user installs at runtime). The `pcap` crate links against `wpcap.lib`,
  and build.rs marks `wpcap.dll` as delay-loaded so PortFinder.exe
  launches even when Npcap isn't installed yet — the in-app banner
  walks the user through the install.
- All builds are unsigned ad-hoc on macOS (`signing-identity = "-"` in
  Cargo.toml). Quarantined downloads need a right-click → Open or
  `xattr -cr`. Notarization is out of scope until an Apple Developer
  account is in place.

## What 4.x dropped from 3.x
- Tauri 2.x (`src-tauri/` tree gone)
- Svelte 5 / TypeScript frontend (`frontend/` tree gone)
- pnpm / Node.js in the build path (`package.json`, `pnpm-lock.yaml`,
  `node_modules/` gone)
- tauri-specta TypeScript binding generation
- The 3.x i18n surface (English-only at launch; can come back as Rust
  string tables if needed)
