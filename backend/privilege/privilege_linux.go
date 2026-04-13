//go:build linux

package privilege

import (
	"os"
	"strings"
)

func hasPlatformPrivilege() bool {
	if isRoot() {
		return true
	}
	// Check for CAP_NET_RAW capability
	data, err := os.ReadFile("/proc/self/status")
	if err != nil {
		return false
	}
	for _, line := range strings.Split(string(data), "\n") {
		if strings.HasPrefix(line, "CapEff:") {
			caps := strings.TrimSpace(strings.TrimPrefix(line, "CapEff:"))
			if len(caps) > 0 && caps != "0000000000000000" {
				return true
			}
		}
	}
	return false
}

func fillPlatformStatus(status *PrivilegeStatus) {
	status.HelperInstalled = false
	status.InBPFGroup = false
	status.CanInstall = false
}
