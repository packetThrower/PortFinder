//go:build darwin

package privilege

import (
	"os"
	"os/exec"
	"os/user"
	"strings"
)

const (
	bpfGroup      = "access_bpf"
	daemonPlist   = "/Library/LaunchDaemons/coop.otec.portfinder.ChmodBPF.plist"
	wiresharkPlist = "/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist"
)

func hasPlatformPrivilege() bool {
	if isRoot() {
		return true
	}
	// Check if BPF devices are readable (ChmodBPF installed and running)
	f, err := os.Open("/dev/bpf0")
	if err == nil {
		f.Close()
		return true
	}
	return false
}

func fillPlatformStatus(status *PrivilegeStatus) {
	status.HelperInstalled = isBPFHelperInstalled()
	status.InBPFGroup = isUserInBPFGroup()
	status.CanInstall = true
}

func isBPFHelperInstalled() bool {
	// Check for either PortFinder's or Wireshark's ChmodBPF
	if _, err := os.Stat(daemonPlist); err == nil {
		return true
	}
	if _, err := os.Stat(wiresharkPlist); err == nil {
		return true
	}
	return false
}

func isUserInBPFGroup() bool {
	u, err := user.Current()
	if err != nil {
		return false
	}
	cmd := exec.Command("dseditgroup", "-o", "checkmember", "-m", u.Username, bpfGroup)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return false
	}
	return strings.Contains(string(output), "yes")
}
