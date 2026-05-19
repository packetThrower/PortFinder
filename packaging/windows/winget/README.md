# Winget manifest scaffold

Templates for getting PortFinder into the [Windows Package Manager
community repository](https://github.com/microsoft/winget-pkgs).

This directory is a **scaffold**, not an active install path —
winget doesn't read files from here. The actual install path is
a PR submitted upstream against `microsoft/winget-pkgs`. The
templates exist so the maintainer of that PR (us) doesn't have
to hand-write the YAML each release.

**Stable tags only.** winget needs a WiX `.msi` for both arches,
and `.github/workflows/release.yml` only emits MSIs on non-pre-
release tags (see the "Build .msi (cargo-wix, stable tags only)"
step). MSI ProductVersion rejects alphanumeric pre-release
identifiers, so betas ship NSIS-only.

## Files

| File | Purpose |
|---|---|
| `packetThrower.PortFinder.locale.en-US.yaml` | Default-locale manifest. Static across versions — copy as-is into each per-version dir, bump `PackageVersion`. |
| `packetThrower.PortFinder.yaml.template` | Version manifest. Substitute `${VERSION}`. |
| `packetThrower.PortFinder.installer.yaml.template` | Installer manifest. Substitute `${VERSION}`, `${RELEASE_DATE}`, `${SHA256_AMD64_MSI}`, `${SHA256_ARM64_MSI}`, `${PRODUCT_CODE_AMD64}`, `${PRODUCT_CODE_ARM64}`. |
| `rendered/<version>/` | Archived copy of the YAMLs submitted upstream for each version. Mirrors what's at `manifests/p/packetThrower/PortFinder/<version>/` in `microsoft/winget-pkgs`. |

## The submission, manually

This is the path if you don't want to install `wingetcreate`.
Targets the `winget-pkgs` fork's `manifests/p/packetThrower/PortFinder/<version>/` layout.

```bash
# From a fork of microsoft/winget-pkgs checked out locally:

VERSION=4.1.1
DATE=$(date -u +%Y-%m-%d)
DEST="manifests/p/packetThrower/PortFinder/$VERSION"
mkdir -p "$DEST"

# Pull the locale manifest in unchanged, then bump its PackageVersion.
cp /path/to/PortFinder/packaging/windows/winget/packetThrower.PortFinder.locale.en-US.yaml "$DEST/"
sed -i "s/^PackageVersion: .*/PackageVersion: $VERSION/" "$DEST/packetThrower.PortFinder.locale.en-US.yaml"

# Compute the SHA256s from the GitHub Release artifacts.
SHA_X64=$(curl -sL "https://github.com/packetThrower/PortFinder/releases/download/v$VERSION/PortFinder_${VERSION}_x64_en-US.msi" | sha256sum | awk '{print $1}')
SHA_ARM64=$(curl -sL "https://github.com/packetThrower/PortFinder/releases/download/v$VERSION/PortFinder_${VERSION}_arm64_en-US.msi" | sha256sum | awk '{print $1}')

# ProductCodes are per-build GUIDs that change each release.
# Extract from each MSI with `msiextract --version` or `lessmsi
# list` on Linux/macOS; on Windows use the WindowsInstaller COM
# API. wingetcreate auto-detects if going via that path (see
# below). The UpgradeCode is stable (defined in
# packaging/windows/wix/main.wxs); ProductCodes are not.
PRODUCT_CODE_X64='{REPLACE-WITH-X64-MSI-PRODUCT-GUID}'
PRODUCT_CODE_ARM64='{REPLACE-WITH-ARM64-MSI-PRODUCT-GUID}'

# Render the templates.
VERSION="$VERSION" RELEASE_DATE="$DATE" \
  SHA256_AMD64_MSI="$SHA_X64" SHA256_ARM64_MSI="$SHA_ARM64" \
  PRODUCT_CODE_AMD64="$PRODUCT_CODE_X64" PRODUCT_CODE_ARM64="$PRODUCT_CODE_ARM64" \
  envsubst < /path/to/PortFinder/packaging/windows/winget/packetThrower.PortFinder.yaml.template \
  > "$DEST/packetThrower.PortFinder.yaml"

VERSION="$VERSION" RELEASE_DATE="$DATE" \
  SHA256_AMD64_MSI="$SHA_X64" SHA256_ARM64_MSI="$SHA_ARM64" \
  PRODUCT_CODE_AMD64="$PRODUCT_CODE_X64" PRODUCT_CODE_ARM64="$PRODUCT_CODE_ARM64" \
  envsubst < /path/to/PortFinder/packaging/windows/winget/packetThrower.PortFinder.installer.yaml.template \
  > "$DEST/packetThrower.PortFinder.installer.yaml"

# Validate before pushing — same checks the upstream CI runs.
winget validate --manifest "$DEST"

# Open a PR against microsoft/winget-pkgs.
```

## The submission, via wingetcreate (preferred)

`wingetcreate` is Microsoft's CLI for the winget-pkgs repo. It
auto-detects everything we have to substitute by hand above —
SHA256, ProductCode, ARP entries, installer type, scope — by
downloading the installer and inspecting it. It also opens the
PR for us against `microsoft/winget-pkgs`.

```powershell
# Windows host. winget install Microsoft.WingetCreate (one-time).

$Version  = "4.1.1"
$MsiX64   = "https://github.com/packetThrower/PortFinder/releases/download/v$Version/PortFinder_${Version}_x64_en-US.msi"
$MsiArm64 = "https://github.com/packetThrower/PortFinder/releases/download/v$Version/PortFinder_${Version}_arm64_en-US.msi"

# First-time submission: scaffolds the three YAMLs interactively
# from the .msi inspection (auto-detects both ProductCodes), then
# prompts for the locale fields.
wingetcreate new "$MsiX64,$MsiArm64"

# Subsequent version bumps: reuses the existing locale manifest
# and only updates URLs + SHA256 + ProductCode.
wingetcreate update packetThrower.PortFinder `
  --version $Version `
  --urls "$MsiX64,$MsiArm64" `
  --submit `
  --token $env:GITHUB_TOKEN
```

The `--submit` flag opens the PR for you (forks
microsoft/winget-pkgs to your account, commits, pushes,
PRs). Without `--submit` it just writes the YAMLs to a
temp directory for review.

## CI hookup (future)

A natural follow-up is to call `wingetcreate update --submit` from
`.github/workflows/release.yml`'s Windows job once the release
artifacts are uploaded — same auto-bump pattern the Homebrew tap
and Scoop bucket already use. Needs a GitHub PAT in the secret
store with `public_repo` scope on a `packetThrower-bot` (or
similar) account that owns the `winget-pkgs` fork.

Defer until the first manual submission lands cleanly — the
moderator review on the first PR can take 1–7 days and may flag
metadata adjustments that would otherwise be baked into the
automated flow.

## Notes

- **PackageIdentifier casing** is `packetThrower.PortFinder`.
  winget accepts mixed-case publishers (`git/git`, `Microsoft/PowerShell`
  both exist); we match the GitHub org casing exactly.
- **Moniker `portfinder`** — competitive but should be available.
  Lets users run `winget install portfinder` instead of the full
  identifier.
- **Code signing** is not required for winget acceptance. Users
  will still see SmartScreen on first run (same UX as Scoop today
  and the manual installer). Notarisation is tracked in
  `TODO.md` under "Known open items from the 4.0 cycle".
- **Both arches ship WiX MSI now** (since the build switched from
  cargo-packager's bundled WiX 3.11 to cargo-wix + system WiX 3.14
  on the Windows runners). The half-machine / half-user registry
  shape of the prior NSIS-on-arm64 install is what tripped
  winget's Validation-Executable-Error on PR
  microsoft/winget-pkgs#376193; the matching shape of two MSIs
  with explicit `Scope: machine` clears that.
