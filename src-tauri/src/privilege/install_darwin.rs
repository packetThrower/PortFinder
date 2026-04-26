use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

// Single shell script that:
//   1. Creates the access_bpf group (idempotent)
//   2. Adds the console user to the group
//   3. Installs the ChmodBPF script under /Library/Application Support/PortFinder
//   4. Installs and loads the LaunchDaemon plist
// Runs as root via `osascript ... with administrator privileges`.
const INSTALL_SCRIPT: &str = r#"#!/bin/sh
set -e

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

mkdir -p "$INSTALL_DIR"

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

launchctl unload "$DAEMON_PLIST" 2>/dev/null || true
launchctl load "$DAEMON_PLIST"

"$INSTALL_DIR/ChmodBPF"
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
