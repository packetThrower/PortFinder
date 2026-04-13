//go:build windows

package privilege

import (
	"os/exec"
	"strings"
)

func hasPlatformPrivilege() bool {
	// On Windows with Npcap, non-admin users can often capture.
	// Check if we're running as admin by trying a known admin-only command.
	cmd := exec.Command("net", "session")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return false
	}
	// If "net session" succeeds without "Access is denied", we're admin
	return !strings.Contains(string(output), "Access is denied")
}
