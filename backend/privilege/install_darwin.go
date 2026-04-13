//go:build darwin

package privilege

/*
#cgo CFLAGS: -x objective-c
#cgo LDFLAGS: -framework Security
#include <Security/Authorization.h>
#include <Security/AuthorizationTags.h>
#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>

// AuthorizationExecuteWithPrivileges is deprecated but still functional
// on all current macOS versions. We load it dynamically to avoid
// build warnings while keeping the native auth dialog that shows
// the calling app name (PortFinder) instead of "osascript".
typedef OSStatus (*AuthExecFn)(AuthorizationRef, const char *,
    AuthorizationFlags, char *const *, FILE **);

int runPrivileged(const char *path, const char *arg) {
    AuthorizationRef authRef;
    OSStatus status;

    status = AuthorizationCreate(NULL, kAuthorizationEmptyEnvironment,
        kAuthorizationFlagDefaults, &authRef);
    if (status != errAuthorizationSuccess) {
        return (int)status;
    }

    AuthorizationItem items = {kAuthorizationRightExecute, 0, NULL, 0};
    AuthorizationRights rights = {1, &items};
    AuthorizationFlags flags = kAuthorizationFlagDefaults |
        kAuthorizationFlagInteractionAllowed |
        kAuthorizationFlagPreAuthorize |
        kAuthorizationFlagExtendRights;

    status = AuthorizationCopyRights(authRef, &rights, NULL, flags, NULL);
    if (status != errAuthorizationSuccess) {
        AuthorizationFree(authRef, kAuthorizationFlagDefaults);
        return (int)status;
    }

    void *lib = dlopen("/System/Library/Frameworks/Security.framework/Security", RTLD_LAZY);
    if (!lib) {
        AuthorizationFree(authRef, kAuthorizationFlagDefaults);
        return -1;
    }

    AuthExecFn execFn = (AuthExecFn)dlsym(lib, "AuthorizationExecuteWithPrivileges");
    if (!execFn) {
        dlclose(lib);
        AuthorizationFree(authRef, kAuthorizationFlagDefaults);
        return -2;
    }

    char *args[] = { (char *)arg, NULL };
    FILE *pipe = NULL;
    status = execFn(authRef, path, kAuthorizationFlagDefaults, args, &pipe);

    if (pipe) {
        char buf[256];
        while (fgets(buf, sizeof(buf), pipe)) {}
        fclose(pipe);
    }

    dlclose(lib);
    AuthorizationFree(authRef, kAuthorizationFlagDefaults);
    return (int)status;
}
*/
import "C"

import (
	"fmt"
	"os"
	"unsafe"
)

func installHelper() error {
	script := `#!/bin/sh
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

	// Write script to temp file
	tmpFile, err := os.CreateTemp("", "portfinder-bpf-*.sh")
	if err != nil {
		return fmt.Errorf("failed to create temp script: %w", err)
	}
	defer os.Remove(tmpFile.Name())

	if _, err := tmpFile.WriteString(script); err != nil {
		tmpFile.Close()
		return fmt.Errorf("failed to write install script: %w", err)
	}
	if err := tmpFile.Chmod(0755); err != nil {
		tmpFile.Close()
		return fmt.Errorf("failed to chmod install script: %w", err)
	}
	tmpFile.Close()

	cPath := C.CString("/bin/sh")
	defer C.free(unsafe.Pointer(cPath))

	cArg := C.CString(tmpFile.Name())
	defer C.free(unsafe.Pointer(cArg))

	status := C.runPrivileged(cPath, cArg)
	if status != 0 {
		if status == -60006 {
			return fmt.Errorf("authorization cancelled by user")
		}
		return fmt.Errorf("BPF helper installation failed (status: %d)", status)
	}

	return nil
}
