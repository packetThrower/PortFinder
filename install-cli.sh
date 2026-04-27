#!/bin/sh
# install-cli.sh — Symlink the PortFinder CLI onto your PATH (macOS).
#
# Creates /usr/local/bin/portfinder pointing at the binary inside
# PortFinder.app so `portfinder` works from any shell. Doesn't touch
# BPF / capture privileges — for that, install the BPF helper .pkg
# from the release page or click "Install BPF Access" inside the app.
#
# Usage:
#   sudo ./install-cli.sh

set -e

if [ "$(uname)" != "Darwin" ]; then
    echo "This script is macOS-only."
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root: sudo $0"
    exit 1
fi

CONSOLE_USER=$(stat -f "%Su" /dev/console 2>/dev/null)

APP_BIN=""
for candidate in \
    "/Applications/PortFinder.app/Contents/MacOS/portfinder" \
    "/Users/$CONSOLE_USER/Applications/PortFinder.app/Contents/MacOS/portfinder"; do
    if [ -x "$candidate" ]; then
        APP_BIN="$candidate"
        break
    fi
done

if [ -z "$APP_BIN" ]; then
    echo "PortFinder.app not found in /Applications or ~/Applications."
    echo "Move the app there first, then re-run this script."
    exit 1
fi

mkdir -p /usr/local/bin
ln -sf "$APP_BIN" /usr/local/bin/portfinder
echo "Linked CLI: /usr/local/bin/portfinder -> $APP_BIN"
echo ""
echo "Try: portfinder --help"
