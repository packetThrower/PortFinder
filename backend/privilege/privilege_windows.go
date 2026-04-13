//go:build windows

package privilege

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

func hasPlatformPrivilege() bool {
	// Check if Npcap is installed with non-admin capture support
	if isNpcapNonAdmin() {
		return true
	}
	// Fall back to checking if running as admin
	return isAdmin()
}

func fillPlatformStatus(status *PrivilegeStatus) {
	status.NpcapInstalled = isNpcapInstalled()
	status.NpcapNonAdmin = isNpcapNonAdmin()
	status.HelperInstalled = status.NpcapInstalled
	status.CanInstall = false // Can't auto-install Npcap (separate installer)
}

func isAdmin() bool {
	cmd := exec.Command("net", "session")
	output, err := cmd.CombinedOutput()
	if err != nil {
		return false
	}
	return !strings.Contains(string(output), "Access is denied")
}

func isNpcapInstalled() bool {
	// Npcap installs to System32\Npcap
	systemRoot := os.Getenv("SystemRoot")
	if systemRoot == "" {
		systemRoot = `C:\Windows`
	}
	npcapDir := filepath.Join(systemRoot, "System32", "Npcap")
	if _, err := os.Stat(npcapDir); err == nil {
		return true
	}
	// Also check the registry-based install path
	npcapDll := filepath.Join(systemRoot, "System32", "Npcap", "wpcap.dll")
	if _, err := os.Stat(npcapDll); err == nil {
		return true
	}
	return false
}

func isNpcapNonAdmin() bool {
	if !isNpcapInstalled() {
		return false
	}
	// When Npcap is installed with non-admin access, it creates
	// a local group and makes capture devices accessible.
	// The simplest check: try to open a pcap handle without admin.
	// But we can also check if the Npcap loopback adapter service exists
	// in non-admin mode by checking the registry or group.
	//
	// Check if "Npcap" group exists (created by non-admin install)
	cmd := exec.Command("net", "localgroup", "Npcap")
	err := cmd.Run()
	return err == nil
}
