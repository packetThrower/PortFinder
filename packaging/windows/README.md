# Windows packaging

`Packager.json` is a Windows-only cargo-packager config used by
the release workflow's NSIS / WiX build step. It's passed via
`cargo packager --config packaging/windows/Packager.json` and
overrides `[package.metadata.packager]` from `Cargo.toml`.

## Why a separate file

The Windows installer ships **two** binaries:

- `PortFinder.exe` — GUI, built with `windows_subsystem = "windows"`
  so File Explorer double-click doesn't pop up a black console.
- `portfinder-cli.exe` — CLI, no subsystem attribute so it defaults
  to `console`. PowerShell waits for it to exit and stdio routes
  to the parent shell the way users expect.

A Windows PE binary has a single subsystem byte set at link time
and a single `.exe` can be either GUI-friendly or CLI-friendly
but not both. macOS and Linux don't have this distinction so they
ship a single `PortFinder` binary that handles both GUI and CLI
via argv dispatch — keeping their `.app` / `.deb` / `.rpm` lean.
See `src/lib.rs` for the full background.

## Why not in Cargo.toml

cargo-packager's `binaries` field is global — there's no
per-format filter. Listing `portfinder-cli` in
`[[package.metadata.packager.binaries]]` would also bundle it
into the macOS `.app` and Linux `.deb`, adding ~2.4 MB of
redundant CLI binary to platforms that don't need it.

## Drift warning

cargo-packager's `--config` flag **replaces** the in-Cargo.toml
config rather than merging with it. Anything Windows actually
depends on (`productName`, `identifier`, `icons`,
`beforePackagingCommand`, …) has to be restated in this file.
When you change `[package.metadata.packager]` in `Cargo.toml`,
audit this file for parallel edits.

Fields that genuinely *are* macOS / Linux-only
(`[package.metadata.packager.deb]`, `[package.metadata.packager.macos]`,
the `files` table) don't need to be here — they were never read
on Windows.

## Adding a new top-level config field

1. Add the field in `Cargo.toml`'s `[package.metadata.packager]`
   in kebab-case (`some-new-field = "value"`).
2. Add it in this file in **camelCase** (`someNewField`) — that's
   the JSON / serde convention cargo-packager uses.
3. Local sanity-check command (run on macOS as a proxy for the
   Windows NSIS path):

   ```sh
   cargo packager --release -f app \
     --out-dir target/release \
     --binaries-dir target/release \
     --config "$(cat packaging/windows/Packager.json)"
   ```

   The resulting `target/release/PortFinder.app/Contents/MacOS/`
   should contain both `PortFinder` and `portfinder-cli`.

## cargo-packager 0.11.8 quirks

`--config` accepts either a file path or a raw JSON string.
The file-path form is broken in 0.11.8 — it errors with
`I/O Error: Not a directory (os error 20)`. Workaround: pipe
the file contents through (`"$(cat …)"` on bash,
`Get-Content -Raw` on PowerShell). The release workflow does
the latter; update both if the bug is fixed upstream.

The `--config` path also **skips cargo workspace metadata
entirely** — that's the path that normally auto-fills
`version` from `Cargo.toml`'s `[package].version`. Without an
explicit `version` in this JSON, cargo-packager errors out
with `empty string, expected a semver version`. We don't
hardcode the version here (it's per-release); the release
workflow injects it dynamically via PowerShell:

```powershell
$version = "${{ github.ref_name }}" -replace '^v', ''
$config_obj = Get-Content packaging/windows/Packager.json -Raw | ConvertFrom-Json
$config_obj | Add-Member -NotePropertyName version -NotePropertyValue $version -Force
$config = $config_obj | ConvertTo-Json -Depth 10
cargo packager … --config $config …
```

If you ever build this locally from a checked-out tag, do the
analogous injection or pass `--config` with a hand-crafted
JSON that includes `"version": "..."`.

## Why `--out-dir` and `--binaries-dir` are explicit

When `--config` is passed with raw JSON, cargo-packager skips
the cargo-metadata path that auto-detects `target/<profile>` as
the binaries directory. The flags compensate by pointing it at
`target/release` explicitly.
