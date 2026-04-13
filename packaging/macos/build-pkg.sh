#!/bin/sh
# build-pkg.sh - Build the PortFinder BPF helper .pkg installer
# Run from the project root: ./packaging/macos/build-pkg.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION=$(cat version.txt 2>/dev/null || echo "1.0")
OUTPUT_DIR="${SCRIPT_DIR}/../../dist"
PKG_NAME="PortFinder-BPF-${VERSION}.pkg"

mkdir -p "$OUTPUT_DIR"

# Create a temp scripts directory with all needed files
TMPDIR=$(mktemp -d)
cp "$SCRIPT_DIR/postinstall" "$TMPDIR/postinstall"
cp "$SCRIPT_DIR/ChmodBPF.sh" "$TMPDIR/ChmodBPF.sh"
cp "$SCRIPT_DIR/coop.otec.portfinder.ChmodBPF.plist" "$TMPDIR/coop.otec.portfinder.ChmodBPF.plist"
chmod +x "$TMPDIR/postinstall"

pkgbuild \
    --nopayload \
    --scripts "$TMPDIR" \
    --identifier coop.otec.portfinder.chmodbpf \
    --version "$VERSION" \
    "$OUTPUT_DIR/$PKG_NAME"

rm -rf "$TMPDIR"

echo "Built: $OUTPUT_DIR/$PKG_NAME"
