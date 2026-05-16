#!/bin/sh
# build-pkg.sh — build the PortFinder BPF helper .pkg installer.
# Run from the project root: ./packaging/macos/build-pkg.sh
#
# Produces `dist/PortFinder-BPF-<version>.pkg`. The release workflow
# uploads this alongside the .dmg artifacts. End users who launch
# PortFinder.app and click "Install BPF Helper" get the same end
# state via privilege::install_darwin::install(); the standalone
# .pkg is for sysadmins deploying via MDM or for the user who'd
# rather double-click an installer than authorise an osascript
# prompt from inside the app.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Pull the version from Cargo.toml's [package] section. awk over
# sed because the same script ports cleanly to Linux without a
# BSD-vs-GNU branch.
VERSION=$(awk '
  /^\[/ { in_pkg = ($0 == "[package]") }
  in_pkg && /^version = / {
    gsub(/^version = "/, ""); gsub(/"$/, "");
    print; exit
  }
' "${SCRIPT_DIR}/../../Cargo.toml" 2>/dev/null)
if [ -z "$VERSION" ]; then
    VERSION="1.0"
fi
OUTPUT_DIR="${SCRIPT_DIR}/../../dist"
PKG_NAME="PortFinder-BPF-${VERSION}.pkg"

mkdir -p "$OUTPUT_DIR"

# Stage the scripts (the postinstall hook + the helper + plist it
# installs) in a temp dir for pkgbuild.
TMPDIR=$(mktemp -d)
cp "$SCRIPT_DIR/postinstall" "$TMPDIR/postinstall"
cp "$SCRIPT_DIR/PortFinder BPF Helper.sh" "$TMPDIR/PortFinder BPF Helper.sh"
cp "$SCRIPT_DIR/io.github.packetThrower.PortFinder.BPFHelper.plist" \
   "$TMPDIR/io.github.packetThrower.PortFinder.BPFHelper.plist"
chmod +x "$TMPDIR/postinstall"

# --nopayload: the .pkg is purely a script hook (no files copied
# directly via Installer.app's payload machinery — postinstall
# stages everything itself). Identifier mirrors the daemon label.
pkgbuild \
    --nopayload \
    --scripts "$TMPDIR" \
    --identifier io.github.packetThrower.PortFinder.BPFHelper \
    --version "$VERSION" \
    "$OUTPUT_DIR/$PKG_NAME"

rm -rf "$TMPDIR"

echo "Built: $OUTPUT_DIR/$PKG_NAME"
