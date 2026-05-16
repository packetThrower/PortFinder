use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

// Single shell script that:
//   1. Removes the legacy `coop.otec.portfinder.ChmodBPF` daemon + helper
//      script from the 3.x install if present, so we don't end up with
//      two helpers racing on /dev/bpf*.
//   2. Creates the access_bpf group (idempotent).
//   3. Adds the console user to the group.
//   4. Installs the helper script under
//      `/Library/Application Support/PortFinder/PortFinder BPF Helper`
//      — the filename is what macOS Background Items renders for the
//      LaunchDaemon, so "PortFinder BPF Helper" is what the user sees in
//      System Settings → General → Login Items & Extensions.
//   5. Installs and loads the LaunchDaemon plist under the
//      `io.github.packetThrower.PortFinder.BPFHelper` label (reverse-DNS
//      of packetthrower.github.io/PortFinder, matching Baudrun's pattern).
// Runs as root via `osascript ... with administrator privileges`.
const INSTALL_SCRIPT: &str = r#"#!/bin/sh
set -e

BPF_GROUP="access_bpf"
INSTALL_DIR="/Library/Application Support/PortFinder"
HELPER_BIN="$INSTALL_DIR/PortFinder BPF Helper"
DAEMON_LABEL="io.github.packetThrower.PortFinder.BPFHelper"
DAEMON_PLIST="/Library/LaunchDaemons/${DAEMON_LABEL}.plist"
LEGACY_DAEMON_PLIST="/Library/LaunchDaemons/coop.otec.portfinder.ChmodBPF.plist"
LEGACY_HELPER_BIN="$INSTALL_DIR/ChmodBPF"

# Drop the 3.x ChmodBPF daemon if it's still loaded so the new helper
# is the only one touching /dev/bpf*. Wireshark's own daemon
# (org.wireshark.ChmodBPF) is left alone — both daemons grant the same
# access_bpf group ACL so they coexist cleanly.
if [ -f "$LEGACY_DAEMON_PLIST" ]; then
    launchctl unload "$LEGACY_DAEMON_PLIST" 2>/dev/null || true
    rm -f "$LEGACY_DAEMON_PLIST"
fi
if [ -f "$LEGACY_HELPER_BIN" ]; then
    rm -f "$LEGACY_HELPER_BIN"
fi

# Create access_bpf group if it doesn't exist
if ! dseditgroup -o read "$BPF_GROUP" > /dev/null 2>&1; then
    dseditgroup -o create "$BPF_GROUP"
fi

# Add current console user to the group
CONSOLE_USER=$(stat -f "%Su" /dev/console 2>/dev/null)
if [ -n "$CONSOLE_USER" ] && [ "$CONSOLE_USER" != "root" ]; then
    dseditgroup -o edit -a "$CONSOLE_USER" -t user "$BPF_GROUP"
fi

mkdir -p "$INSTALL_DIR"

cat > "$HELPER_BIN" << 'SCRIPT'
#!/bin/sh
# PortFinder BPF Helper — sets BPF device permissions so PortFinder
# (and any other capture client in the access_bpf group) can read
# /dev/bpf* without sudo. Runs at boot via LaunchDaemon.
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
syslog -s -l notice "PortFinder BPF Helper: BPF devices configured for group $BPF_GROUP"
SCRIPT
chmod 755 "$HELPER_BIN"
chown root:wheel "$HELPER_BIN"

cat > "$DAEMON_PLIST" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${DAEMON_LABEL}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>Program</key>
    <string>${HELPER_BIN}</string>
</dict>
</plist>
PLIST
chown root:wheel "$DAEMON_PLIST"
chmod 644 "$DAEMON_PLIST"

launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
launchctl load "$DAEMON_PLIST"

"$HELPER_BIN"

# Drop a symlink so the CLI is callable as `portfinder` from any shell.
# Search /Applications and the user's Applications dir for the bundle —
# CFBundleExecutable is `PortFinder` (capitalised) to match what shows
# in the Dock / Cmd-Tab, while the symlink is the lowercased convention.
APP_BIN=""
for candidate in \
    "/Applications/PortFinder.app/Contents/MacOS/PortFinder" \
    "$HOME/Applications/PortFinder.app/Contents/MacOS/PortFinder"; do
    if [ -x "$candidate" ]; then
        APP_BIN="$candidate"
        break
    fi
done
if [ -n "$APP_BIN" ]; then
    mkdir -p /usr/local/bin
    ln -sf "$APP_BIN" /usr/local/bin/portfinder
fi
"#;

pub fn install() -> Result<(), String> {
    // Write the script to a temp file the elevated shell can read.
    let dir = std::env::temp_dir();
    let path = dir.join(format!("portfinder-bpf-{}.sh", std::process::id()));
    {
        let mut f = std::fs::File::create(&path)
            .map_err(|e| format!("failed to create temp script: {e}"))?;
        f.write_all(INSTALL_SCRIPT.as_bytes())
            .map_err(|e| format!("failed to write install script: {e}"))?;
        let mut perms = f
            .metadata()
            .map_err(|e| format!("failed to stat temp script: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("failed to chmod temp script: {e}"))?;
    }

    // Use osascript to elevate. The 'do shell script' command shows the
    // standard macOS authentication dialog for the calling app (in this
    // case osascript). The script runs as root and any non-zero exit
    // surfaces as a non-zero osascript exit too.
    let path_str = path.to_string_lossy().replace('\'', r"'\''");
    let applescript = format!(
        "do shell script \"/bin/sh '{}'\" with administrator privileges",
        path_str
    );
    let output = Command::new("osascript")
        .args(["-e", &applescript])
        .output()
        .map_err(|e| format!("failed to run osascript: {e}"))?;

    let _ = std::fs::remove_file(&path);

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("-128") || stderr.to_lowercase().contains("user canceled") {
        return Err("authorization cancelled by user".into());
    }
    Err(format!("BPF helper installation failed: {}", stderr.trim()))
}
