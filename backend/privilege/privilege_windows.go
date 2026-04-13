//go:build windows

package privilege

import (
	"os/exec"
	"strings"
)

func hasPlatformPrivilege() bool {
	cmd := exec.Command("net", "session")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return false
	}
	return !strings.Contains(string(output), "Access is denied")
}

func fillPlatformStatus(status *PrivilegeStatus) {
	status.HelperInstalled = false
	status.InBPFGroup = false
	status.CanInstall = false
}
