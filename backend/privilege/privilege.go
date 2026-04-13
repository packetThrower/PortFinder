package privilege

import (
	"os/user"
	"runtime"
)

type PrivilegeStatus struct {
	HasAccess       bool   `json:"hasAccess"`
	HelperInstalled bool   `json:"helperInstalled"`
	InBPFGroup      bool   `json:"inBPFGroup"`
	CanInstall      bool   `json:"canInstall"`
	Platform        string `json:"platform"`
	NpcapInstalled  bool   `json:"npcapInstalled"`
	NpcapNonAdmin   bool   `json:"npcapNonAdmin"`
}

// HasCapturePrivilege checks if the current process has sufficient
// privileges to perform raw packet capture.
func HasCapturePrivilege() bool {
	return hasPlatformPrivilege()
}

// GetPrivilegeStatus returns detailed privilege information
// for the current platform.
func GetPrivilegeStatus() PrivilegeStatus {
	status := PrivilegeStatus{
		HasAccess: hasPlatformPrivilege(),
		Platform:  runtime.GOOS,
	}
	fillPlatformStatus(&status)
	return status
}

// InstallBPFHelper installs the BPF helper on macOS.
// No-op on other platforms.
func InstallBPFHelper() error {
	return installHelper()
}

func isRoot() bool {
	u, err := user.Current()
	if err != nil {
		return false
	}
	return u.Uid == "0"
}
