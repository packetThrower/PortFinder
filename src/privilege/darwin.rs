use super::PrivilegeStatus;
use std::path::Path;
use std::process::Command;

const BPF_GROUP: &str = "access_bpf";
// New plist label / path. Replaces the legacy `coop.otec.portfinder.ChmodBPF`
// from the Tauri-era build. The reverse-DNS form mirrors what packetThrower
// uses for Baudrun (io.github.packetThrower.Baudrun); the helper filename
// `PortFinder BPF Helper` is what macOS Background Items shows the user in
// System Settings → General → Login Items & Extensions.
const DAEMON_PLIST: &str =
    "/Library/LaunchDaemons/io.github.packetThrower.PortFinder.BPFHelper.plist";
// Legacy daemon plist from the 3.x Tauri era. We probe for it so an existing
// install registers as "helper installed" even before the user re-runs the
// new installer — the legacy ChmodBPF binary still grants /dev/bpf* access
// to the access_bpf group, so capture works either way until the new
// installer is run (which cleans up the legacy entry).
const LEGACY_DAEMON_PLIST: &str =
    "/Library/LaunchDaemons/coop.otec.portfinder.ChmodBPF.plist";
const WIRESHARK_PLIST: &str = "/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist";

pub fn has_platform_privilege() -> bool {
    if super::is_root() {
        return true;
    }
    // ChmodBPF makes /dev/bpf0 readable for members of the access_bpf group.
    std::fs::File::open("/dev/bpf0").is_ok()
}

pub fn fill_platform_status(status: &mut PrivilegeStatus) {
    status.helper_installed = is_bpf_helper_installed();
    status.in_bpf_group = is_user_in_bpf_group();
    status.can_install = true;
}

fn is_bpf_helper_installed() -> bool {
    Path::new(DAEMON_PLIST).exists()
        || Path::new(LEGACY_DAEMON_PLIST).exists()
        || Path::new(WIRESHARK_PLIST).exists()
}

fn is_user_in_bpf_group() -> bool {
    let Some(username) = nix::unistd::User::from_uid(nix::unistd::Uid::effective())
        .ok()
        .flatten()
        .map(|u| u.name)
    else {
        return false;
    };
    let Ok(output) = Command::new("dseditgroup")
        .args(["-o", "checkmember", "-m", &username, BPF_GROUP])
        .output()
    else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout).contains("yes")
}
