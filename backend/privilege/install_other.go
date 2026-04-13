//go:build !darwin

package privilege

import "fmt"

func installHelper() error {
	return fmt.Errorf("BPF helper installation is only supported on macOS")
}
