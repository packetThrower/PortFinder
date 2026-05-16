#!/bin/sh
# uninstall-bpf.sh — remove the PortFinder BPF helper and restore
# default permissions. Usage: sudo ./uninstall-bpf.sh
#
# Reverses everything the 4.x BPF installer + the legacy 3.x
# ChmodBPF flow does:
#   1. Unloads + removes the LaunchDaemon (both new + legacy
#      labels, in case the user is mid-upgrade)
#   2. Removes the helper binary
#   3. Removes the current console user from access_bpf
#   4. Restores default BPF device permissions (root-only)
#   5. Optionally deletes the access_bpf group (only if Wireshark
#      isn't relying on it)

set -e

DAEMON_PLIST="/Library/LaunchDaemons/io.github.packetThrower.PortFinder.BPFHelper.plist"
LEGACY_DAEMON_PLIST="/Library/LaunchDaemons/coop.otec.portfinder.ChmodBPF.plist"
INSTALL_DIR="/Library/Application Support/PortFinder"
BPF_GROUP="access_bpf"

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root: sudo $0"
    exit 1
fi

echo "Uninstalling PortFinder BPF helper..."

for plist in "$DAEMON_PLIST" "$LEGACY_DAEMON_PLIST"; do
    if [ -f "$plist" ]; then
        launchctl unload "$plist" 2>/dev/null || true
        rm -f "$plist"
        echo "  Removed LaunchDaemon: $(basename "$plist")"
    fi
done

# Remove the helper binary (both new + legacy filenames)
if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo "  Removed $INSTALL_DIR"
fi

# Remove current console user from access_bpf group
CONSOLE_USER=$(stat -f "%Su" /dev/console 2>/dev/null)
if [ -n "$CONSOLE_USER" ] && [ "$CONSOLE_USER" != "root" ]; then
    if dseditgroup -o checkmember -m "$CONSOLE_USER" "$BPF_GROUP" > /dev/null 2>&1; then
        dseditgroup -o edit -d "$CONSOLE_USER" -t user "$BPF_GROUP"
        echo "  Removed $CONSOLE_USER from $BPF_GROUP group"
    fi
fi

# Restore default BPF device permissions (root-only)
chown root:wheel /dev/bpf* 2>/dev/null || true
chmod 600 /dev/bpf* 2>/dev/null || true
echo "  Restored BPF devices to root-only access"

# Remove the CLI symlink if it points back at PortFinder.app
SYMLINK="/usr/local/bin/portfinder"
if [ -L "$SYMLINK" ]; then
    TARGET=$(readlink "$SYMLINK")
    case "$TARGET" in
        */PortFinder.app/Contents/MacOS/PortFinder)
            rm -f "$SYMLINK"
            echo "  Removed CLI symlink ($SYMLINK)"
            ;;
    esac
fi

# Delete the group if no members remain and Wireshark isn't using it
if ! [ -f "/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist" ]; then
    MEMBERS=$(dscl . -read /Groups/$BPF_GROUP GroupMembership 2>/dev/null | sed 's/GroupMembership: //')
    if [ -z "$MEMBERS" ] || [ "$MEMBERS" = "GroupMembership:" ]; then
        dseditgroup -o delete "$BPF_GROUP" 2>/dev/null || true
        echo "  Deleted $BPF_GROUP group (no remaining members)"
    else
        echo "  Kept $BPF_GROUP group (other members: $MEMBERS)"
    fi
else
    echo "  Kept $BPF_GROUP group (Wireshark ChmodBPF is installed)"
fi

echo ""
echo "BPF helper uninstalled. Packet capture now requires sudo."
