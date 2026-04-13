//go:build darwin

package privilege

import "os"

func hasPlatformPrivilege() bool {
	if isRoot() {
		return true
	}
	// Check if BPF devices are readable (e.g., ChmodBPF from Wireshark)
	f, err := os.Open("/dev/bpf0")
	if err == nil {
		f.Close()
		return true
	}
	return false
}
