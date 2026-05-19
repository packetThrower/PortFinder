# PortFinder TODO

Running list of follow-up work. Loosely ordered by "value /
effort". GitHub Issues is still the right place for anything that
needs discussion or that an outside contributor might want to
pick up — this file is for ideas / notes that don't yet warrant
a tracked issue.

## Logging

All landed in 4.1.0 (see CHANGELOG).

## Known open items from the 4.0 cycle

- [ ] **macOS notarization + Windows code signing**
  ([#13](https://github.com/packetThrower/PortFinder/issues/13)).
  Blocked on a paid Apple Developer account ($99/yr) and a
  Windows code-signing cert. Until then the macOS .dmg ships
  ad-hoc signed (Gatekeeper "unidentified developer", right-
  click → Open workaround) and Windows users get a SmartScreen
  prompt on first run.
- [ ] **Real in-app updater.** Today the "Update available" pill
  opens the GitHub release page; the user manually downloads +
  installs. A proper updater (Sparkle on macOS, custom on
  Windows + Linux) downloads, verifies signature, swaps in the
  new binary, prompts for restart. Significant feature, gated on
  notarization above for macOS.
- [ ] **Refresh screenshots.** Docs site + README still reference
  3.x-era PNGs. Take fresh ones from 4.0.x on each platform
  (macos.png, windows.png, linux.png, cli.png).
- [x] **i18n** — landed in 4.1.0. Seven shipped locales (en/de/
  es/fr/it/pt/ja) via `rust-i18n`, OS-locale detection at
  startup, in-app picker that switches live.

## CI / build

- [ ] **Auto-rerun-on-flake.** The Win arm64 rustc thin-LTO crash
  is currently mitigated by disabling LTO for that target. If a
  similar flake pops up on a different runner, we'd want a
  retry loop in `release.yml` rather than manually clicking
  rerun. GitHub Actions has `actions/retry` but it's better as
  a workflow-level retry on the build step than wrapping the
  whole job.
- [ ] **Stable `gpui` commit pin.** Currently `gpui = { git =
  "..." }` with no `rev`, relying on `Cargo.lock` for
  reproducibility. A `cargo update -p gpui` could silently
  pick up a breaking change. Pin to a specific rev with a
  comment + a process for deliberate bumps.
- [ ] **Cargo profile per Windows arch.** The thin-LTO crash
  workaround lives in `release.yml`'s `CARGO_PROFILE_RELEASE_LTO`
  env override. Cleaner home for it: a `profile.release.windows-
  arm64` table in `Cargo.toml` once Cargo supports per-target
  profile fields, or a custom `release-no-lto` profile invoked
  only for the affected matrix entry.

## Test coverage

- [ ] **Beyond parsers.** All 20 existing tests cover the CDP /
  LLDP / MNDP parsers. `cli`, `privilege`, `updater`, `settings`,
  and the capture-orchestration `race_first` / `capture_one`
  paths have zero coverage. Worth adding at least:
  - `cli::run` with a fake `capture::run` that returns a known
    `CaptureResult` (need a trait abstraction first)
  - `settings::Settings` round-trip (load → toggle → save → load)
  - `updater::check_for_update` against a fake `ureq` response
    (legacy-tag filter, prefix allowlist, channel policy)
- [ ] **A property test or two on the parsers.** Fuzz random
  byte sequences through `cdp::parse` / `lldp::parse` /
  `mndp::parse` and assert they never panic. We already do
  bounds-checks but a `proptest` harness would catch any
  arithmetic-overflow regression future-us introduces.

## Polish / discoverability

All landed in 4.1.0 (see CHANGELOG). The history
feature additionally supports opt-in disk persistence via a
toggle in the settings popover.

## Cross-distro packaging

- [ ] **openSUSE-friendly .rpm.** Our libpcap dep is
  `libpcap.so.1()(64bit)` — works on Fedora; on openSUSE the
  package is `libpcap1`. Verify the SONAME alone is enough or
  add a `(libpcap1 or libpcap)` rich-dep alternative.
- [ ] **Flatpak / Snap.** Sandboxed Linux distribution. Packet
  capture under either is tricky (Flatpak's permission model
  doesn't have a "raw network" portal yet); might require
  shipping outside the sandbox via `--filesystem=host-os` or
  similar.
