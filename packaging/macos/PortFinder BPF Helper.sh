#!/bin/sh
# PortFinder BPF Helper — sets BPF device permissions so PortFinder
# (and any other capture client in the access_bpf group) can read
# /dev/bpf* without sudo. Runs at boot via LaunchDaemon
# `io.github.packetThrower.PortFinder.BPFHelper`.
#
# This script is installed to
#   /Library/Application Support/PortFinder/PortFinder BPF Helper
# at .pkg install time. macOS Background Items (System Settings →
# General → Login Items & Extensions) renders the LaunchDaemon's
# program filename, so "PortFinder BPF Helper" is what the user
# sees there.

BPF_GROUP="access_bpf"

# Get max BPF devices from kernel; fall back to a sensible default
# if sysctl can't be queried.
MAXDEV=$(sysctl -n debug.bpf_maxdevices 2>/dev/null)
if [ -z "$MAXDEV" ]; then
    MAXDEV=256
fi

# Pre-create BPF devices by reading from each one. Opening the
# device triggers creation in devfs so the subsequent chgrp/chmod
# applies to a populated set.
CUR_DEV=0
while [ "$CUR_DEV" -lt "$MAXDEV" ]; do
    cat /dev/bpf$CUR_DEV > /dev/null 2>&1
    CUR_DEV=$((CUR_DEV + 1))
done

# Set group and permissions on all BPF devices
chgrp $BPF_GROUP /dev/bpf* 2>/dev/null
chmod g+rw /dev/bpf* 2>/dev/null

syslog -s -l notice "PortFinder BPF Helper: BPF devices configured for group $BPF_GROUP"
