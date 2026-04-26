#!/bin/sh
# Allow non-root packet capture by granting CAP_NET_RAW to the binary.
# Tauri installs the executable to /usr/bin/portfinder by default.
setcap cap_net_raw+ep /usr/bin/portfinder 2>/dev/null || true
