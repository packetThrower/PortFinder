#!/bin/sh
# Post-install hook for PortFinder's .deb / .rpm / pacman packages.
# Sets CAP_NET_RAW + CAP_NET_ADMIN on the installed binary so the
# user doesn't need to relaunch with `sudo` to capture packets.
#
# Wrapped in command-existence checks so chroot / container
# installs (no setcap, no libcap2-bin) don't fail noisily — capture
# falls back to "needs root" and the in-app banner explains.

set -e

BIN="/usr/bin/PortFinder"
if [ -x "$BIN" ] && command -v setcap >/dev/null 2>&1; then
    setcap cap_net_raw,cap_net_admin=eip "$BIN" 2>/dev/null || true
fi

exit 0
