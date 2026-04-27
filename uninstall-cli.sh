#!/bin/sh
# uninstall-cli.sh — Remove the PortFinder CLI symlink (macOS).
#
# Removes /usr/local/bin/portfinder if it points back into a
# PortFinder.app bundle. Leaves the BPF helper alone — for that, use
# packaging/macos/uninstall-bpf.sh.
#
# Usage:
#   sudo ./uninstall-cli.sh

set -e

if [ "$(uname)" != "Darwin" ]; then
    echo "This script is macOS-only."
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root: sudo $0"
    exit 1
fi

SYMLINK="/usr/local/bin/portfinder"

if [ ! -L "$SYMLINK" ]; then
    echo "No symlink at $SYMLINK — nothing to do."
    exit 0
fi

TARGET=$(readlink "$SYMLINK")
case "$TARGET" in
    */PortFinder.app/Contents/MacOS/portfinder)
        rm -f "$SYMLINK"
        echo "Removed $SYMLINK"
        ;;
    *)
        echo "$SYMLINK points at $TARGET, not a PortFinder.app bundle."
        echo "Refusing to remove. Delete it manually if you really want to."
        exit 1
        ;;
esac
