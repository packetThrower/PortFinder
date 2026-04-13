package privilege

import "os/user"

// HasCapturePrivilege checks if the current process has sufficient
// privileges to perform raw packet capture.
func HasCapturePrivilege() bool {
	return hasPlatformPrivilege()
}

func isRoot() bool {
	u, err := user.Current()
	if err != nil {
		return false
	}
	return u.Uid == "0"
}
