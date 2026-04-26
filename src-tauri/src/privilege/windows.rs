use super::PrivilegeStatus;
use std::path::PathBuf;
use std::process::Command;

pub fn has_platform_privilege() -> bool {
    is_npcap_non_admin() || is_admin()
}

pub fn fill_platform_status(status: &mut PrivilegeStatus) {
    status.npcap_installed = is_npcap_installed();
    status.npcap_non_admin = is_npcap_non_admin();
    status.helper_installed = status.npcap_installed;
    status.can_install = false; // Npcap has its own installer
}

fn is_admin() -> bool {
    let Ok(output) = Command::new("net").arg("session").output() else {
        return false;
    };
    !String::from_utf8_lossy(&output.stderr).contains("Access is denied") && output.status.success()
}

fn is_npcap_installed() -> bool {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let npcap_dir = PathBuf::from(&system_root).join("System32").join("Npcap");
    if npcap_dir.exists() {
        return true;
    }
    npcap_dir.join("wpcap.dll").exists()
}

fn is_npcap_non_admin() -> bool {
    if !is_npcap_installed() {
        return false;
    }
    // Non-admin Npcap install creates an "Npcap" local group.
    Command::new("net")
        .args(["localgroup", "Npcap"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
