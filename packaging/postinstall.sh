#!/bin/sh
# Allow non-root packet capture
setcap cap_net_raw+ep /usr/bin/portfinder 2>/dev/null || true
