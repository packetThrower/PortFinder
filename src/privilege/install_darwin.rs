use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;

use security_framework::authorization::{Authorization, AuthorizationItemSetBuilder, Flags};
use tempfile::NamedTempFile;

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
    // Write the bootstrap script to a temp file the elevated shell
    // will read. The path must be unguessable + atomically created
    // because the elevated `/bin/sh` will open whatever path we
    // hand it, as root. Previously this code constructed the path
    // manually as `/tmp/portfinder-bpf-<pid>.sh` and called
    // `File::create` (no O_EXCL, follows symlinks); a local
    // unprivileged user could pre-symlink the predictable path
    // anywhere they liked and the elevated shell would happily run
    // that target as root.
    //
    // `tempfile::NamedTempFile` uses `mkstemp` underneath:
    //   - O_CREAT | O_EXCL — fails if the path already exists, so
    //     a pre-placed symlink can't win the race
    //   - random 6-byte suffix — path is unguessable to other
    //     local users
    //   - inherits umask, so the file ends up 0600 owned by us
    //
    // We don't need to `chmod 0755` to run it: the elevated
    // `/bin/sh` runs as root and bypasses owner/group/mode checks,
    // so 0600 is plenty. Keeping it owner-only also means no other
    // local user can read the script contents while it's on disk.
    let mut tmp = NamedTempFile::with_prefix("portfinder-bpf-")
        .map_err(|e| format!("failed to create temp script: {e}"))?;
    tmp.write_all(INSTALL_SCRIPT.as_bytes())
        .map_err(|e| format!("failed to write install script: {e}"))?;
    let mut perms = tmp
        .as_file()
        .metadata()
        .map_err(|e| format!("failed to stat temp script: {e}"))?
        .permissions();
    // Belt-and-braces — explicit 0600 in case the inherited umask
    // would have been laxer. fchmod via `set_permissions` on the
    // open file handle, no path-resolution race.
    perms.set_mode(0o600);
    tmp.as_file()
        .set_permissions(perms)
        .map_err(|e| format!("failed to chmod temp script: {e}"))?;

    let path_str = tmp.path().to_string_lossy().into_owned();
    let result = run_with_admin_privileges(&path_str);

    // `NamedTempFile`'s Drop unlinks the file. Explicit close
    // makes the cleanup happen here rather than later in this
    // function's stack unwind.
    drop(tmp);

    result
}

/// Request the `system.privilege.admin` right via macOS
/// AuthorizationServices and exec `/bin/sh <script>` as root. The
/// dialog box that pops up reads "PortFinder wants to make changes"
/// rather than the previous "osascript wants to make changes" because
/// the auth request now originates from inside this process. A
/// custom `prompt` string is wired into the auth environment so the
/// body text under the title spells out what the install does, not
/// the generic AuthorizationServices default.
///
/// Wraps the deprecated `AuthorizationExecuteWithPrivileges` API
/// (via security-framework). The replacement Apple recommends is
/// SMJobBless, which requires a Developer-ID-signed helper bundle
/// with `SMAuthorizedClients` matching our team identifier — blocked
/// on us getting an Apple Developer account. Until then, the
/// deprecated call still works on macOS 15 (Sequoia) and 26 (Tahoe).
fn run_with_admin_privileges(script_path: &str) -> Result<(), String> {
    // OSStatus code for "user clicked Cancel" in the auth dialog.
    // Hardcoded because security-framework-sys doesn't re-export
    // the named constant publicly.
    const ERR_AUTHORIZATION_CANCELED: i32 = -60006;

    let rights = AuthorizationItemSetBuilder::new()
        .add_right("system.privilege.admin")
        .map_err(|e| format!("auth: declaring required right failed: {e}"))?
        .build();

    // Build the auth environment. `prompt` overrides the body text
    // under the dialog title (title stays "PortFinder wants to make
    // changes", driven by the calling process's identity). `icon`
    // swaps the generic padlock/exec glyph for PortFinder's own
    // icon — kAuthorizationEnvironmentIcon wants a path to a
    // PNG/JPEG/TIFF (not .icns), so we point at icon.png in the
    // bundle's Resources/ in production and fall back to the
    // repo's resources/icons/icon.png in dev (`cargo run`).
    let mut env_builder = AuthorizationItemSetBuilder::new()
        .add_string(
            "prompt",
            "PortFinder needs to install the BPF helper for packet capture.",
        )
        .map_err(|e| format!("auth: setting prompt failed: {e}"))?;
    if let Some(icon_path) = find_dialog_icon() {
        env_builder = env_builder
            .add_string("icon", icon_path)
            .map_err(|e| format!("auth: setting icon failed: {e}"))?;
    }
    let env = env_builder.build();

    let auth = Authorization::new(
        Some(rights),
        Some(env),
        Flags::INTERACTION_ALLOWED | Flags::EXTEND_RIGHTS | Flags::PREAUTHORIZE,
    )
    .map_err(|e| {
        if e.code() == ERR_AUTHORIZATION_CANCELED {
            "authorization cancelled by user".to_string()
        } else {
            format!("authorization failed: {e}")
        }
    })?;

    // Use the piped variant so we can read the script's output to
    // EOF — that read blocks until the child exits, which is what we
    // want as a "wait for completion" signal. The non-piped variant
    // returns the moment the child is forked, which would let our
    // caller's `refresh_privileges()` run before the install finishes
    // and report stale "no helper installed" state.
    let pipe = auth
        .execute_with_privileges_piped("/bin/sh", [script_path], Flags::DEFAULTS)
        .map_err(|e| format!("BPF helper install failed: {e}"))?;

    let mut output = String::new();
    let _ = std::io::BufReader::new(pipe).read_to_string(&mut output);

    Ok(())
}

/// Locate the PNG that AuthorizationServices should display in the
/// password dialog. Returns the path as a string if a usable icon is
/// found, or None if neither candidate exists (in which case the
/// dialog falls back to the generic padlock).
///
/// Two candidates checked in order:
///   1. Production: `<bundle>/Contents/Resources/icon.png`,
///      derived from the running binary's path. cargo-packager
///      copies `resources/icons/icon.png` into that slot when it
///      builds the .app.
///   2. Dev (`cargo run`): the repo-root `resources/icons/icon.png`,
///      resolved via `CARGO_MANIFEST_DIR` so the path is correct no
///      matter where the binary runs from.
///
/// Only PNG / JPEG / TIFF work here — kAuthorizationEnvironmentIcon
/// doesn't understand .icns, so we deliberately skip the .icns even
/// though it sits right next to icon.png in the bundle.
fn find_dialog_icon() -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents_dir) = exe.parent().and_then(|p| p.parent()) {
            let bundle_icon = contents_dir.join("Resources/icon.png");
            if bundle_icon.exists() {
                return Some(bundle_icon.to_string_lossy().into_owned());
            }
        }
    }
    let dev_icon = format!("{}/resources/icons/icon.png", env!("CARGO_MANIFEST_DIR"));
    if std::path::Path::new(&dev_icon).exists() {
        return Some(dev_icon);
    }
    None
}
