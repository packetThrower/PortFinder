#!/bin/sh
# ChmodBPF - Set BPF device permissions for packet capture
# Installed by PortFinder to allow non-root packet capture on macOS.
# Runs at boot via LaunchDaemon.

BPF_GROUP="access_bpf"

# Get max BPF devices from kernel
MAXDEV=$(sysctl -n debug.bpf_maxdevices 2>/dev/null)
if [ -z "$MAXDEV" ]; then
    MAXDEV=256
fi

# Pre-create BPF devices by reading from them
CUR_DEV=0
while [ "$CUR_DEV" -lt "$MAXDEV" ]; do
    # Opening the device triggers creation in devfs
    cat /dev/bpf$CUR_DEV > /dev/null 2>&1
    CUR_DEV=$((CUR_DEV + 1))
done

# Set group and permissions on all BPF devices
chgrp $BPF_GROUP /dev/bpf* 2>/dev/null
chmod g+rw /dev/bpf* 2>/dev/null

syslog -s -l notice "PortFinder ChmodBPF: BPF devices configured for group $BPF_GROUP"
