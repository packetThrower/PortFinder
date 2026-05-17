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

# Lowercase CLI alias so `portfinder capture …` matches the binary
# name everyone types. The capitalised `PortFinder` binary is kept
# so the .desktop file's Exec= line, the GNOME app launcher entry,
# and the running process name all stay consistent with the macOS
# .app's CFBundleExecutable. The symlink is owned by this script,
# not by dpkg/rpm/pacman's file manifest — the matching prerm
# (`portfinder-preremove.sh`) deletes it on uninstall so apt purge
# / dnf remove / pacman -R don't leave a dangling link behind.
if [ -x "$BIN" ]; then
    ln -sf PortFinder /usr/bin/portfinder
fi

exit 0
