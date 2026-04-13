//go:build darwin

package privilege

import (
	"fmt"
	"os/exec"
)

// installHelper installs the BPF helper on macOS using osascript
// to prompt for admin privileges. This creates the access_bpf group,
// adds the current user, installs the LaunchDaemon, and runs it.
func installHelper() error {
	script := `
BPF_GROUP="access_bpf"
INSTALL_DIR="/Library/Application Support/PortFinder"
DAEMON_PLIST="/Library/LaunchDaemons/coop.otec.portfinder.ChmodBPF.plist"

# Create access_bpf group if it doesn't exist
if ! dseditgroup -o read "$BPF_GROUP" > /dev/null 2>&1; then
    dseditgroup -o create "$BPF_GROUP"
fi

# Add current console user to the group
CONSOLE_USER=$(stat -f "%Su" /dev/console 2>/dev/null)
if [ -n "$CONSOLE_USER" ] && [ "$CONSOLE_USER" != "root" ]; then
    dseditgroup -o edit -a "$CONSOLE_USER" -t user "$BPF_GROUP"
fi

# Create install directory
mkdir -p "$INSTALL_DIR"

# Write the ChmodBPF script
cat > "$INSTALL_DIR/ChmodBPF" << 'SCRIPT'
#!/bin/sh
BPF_GROUP="access_bpf"
MAXDEV=$(sysctl -n debug.bpf_maxdevices 2>/dev/null)
if [ -z "$MAXDEV" ]; then MAXDEV=256; fi
CUR_DEV=0
while [ "$CUR_DEV" -lt "$MAXDEV" ]; do
    cat /dev/bpf$CUR_DEV > /dev/null 2>&1
    CUR_DEV=$((CUR_DEV + 1))
done
chgrp $BPF_GROUP /dev/bpf* 2>/dev/null
chmod g+rw /dev/bpf* 2>/dev/null
SCRIPT
chmod 755 "$INSTALL_DIR/ChmodBPF"
chown root:wheel "$INSTALL_DIR/ChmodBPF"

# Write the LaunchDaemon plist
cat > "$DAEMON_PLIST" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>coop.otec.portfinder.ChmodBPF</string>
    <key>RunAtLoad</key>
    <true/>
    <key>Program</key>
    <string>/Library/Application Support/PortFinder/ChmodBPF</string>
</dict>
</plist>
PLIST
chown root:wheel "$DAEMON_PLIST"
chmod 644 "$DAEMON_PLIST"

# Load the daemon
launchctl unload "$DAEMON_PLIST" 2>/dev/null
launchctl load "$DAEMON_PLIST"

# Run immediately
"$INSTALL_DIR/ChmodBPF"
`

	cmd := exec.Command("osascript", "-e",
		fmt.Sprintf(`do shell script "%s" with administrator privileges`, escapeForAppleScript(script)))

	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("BPF helper installation failed: %w (output: %s)", err, string(output))
	}
	return nil
}

func escapeForAppleScript(s string) string {
	// Escape backslashes first, then double quotes
	result := ""
	for _, c := range s {
		switch c {
		case '\\':
			result += "\\\\"
		case '"':
			result += "\\\""
		default:
			result += string(c)
		}
	}
	return result
}
