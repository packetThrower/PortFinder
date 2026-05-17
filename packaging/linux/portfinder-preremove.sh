#!/bin/sh
# Pre-remove hook for PortFinder's .deb / .rpm / pacman packages.
# Cleans up the lowercase `/usr/bin/portfinder` symlink that the
# matching postinst (`portfinder-postinstall.sh`) created. The
# symlink isn't tracked in the package's file manifest, so dpkg /
# rpm / pacman won't remove it on their own.
#
# `rm -f` is intentional — if the symlink was already deleted by
# the user (or never created on a chroot install that skipped the
# postinst), don't fail the uninstall.

set -e

rm -f /usr/bin/portfinder

exit 0
