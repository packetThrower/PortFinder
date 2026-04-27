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

/// Returns true if the current process is running with administrator
/// privileges. Uses `CheckTokenMembership` against the well-known
/// `BUILTIN\Administrators` SID — the approach Microsoft recommends, and
/// unlike parsing `net session` stderr it's locale-independent and avoids
/// spawning a child process on every privilege check.
fn is_admin() -> bool {
    use windows_sys::Win32::Security::{
        AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SID_IDENTIFIER_AUTHORITY,
    };
    use windows_sys::Win32::System::SystemServices::{
        DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID, SECURITY_NT_AUTHORITY,
    };

    // SID for BUILTIN\Administrators: S-1-5-32-544
    let nt_authority = SID_IDENTIFIER_AUTHORITY {
        Value: SECURITY_NT_AUTHORITY,
    };
    let mut admin_group = std::ptr::null_mut();

    unsafe {
        if AllocateAndInitializeSid(
            &nt_authority,
            2,
            SECURITY_BUILTIN_DOMAIN_RID as u32,
            DOMAIN_ALIAS_RID_ADMINS as u32,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut admin_group,
        ) == 0
        {
            return false;
        }

        let mut is_member = 0;
        // Passing a null token means "use the current process's effective
        // token" — which is what we want.
        let ok = CheckTokenMembership(std::ptr::null_mut(), admin_group, &mut is_member);
        FreeSid(admin_group);

        ok != 0 && is_member != 0
    }
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
