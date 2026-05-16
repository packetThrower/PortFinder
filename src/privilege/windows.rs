use super::PrivilegeStatus;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
/// `BUILTIN\Administrators` SID (S-1-5-32-544) — the approach Microsoft
/// recommends, and unlike parsing `net session` stderr it's locale-
/// independent and avoids spawning a child process on every privilege
/// check.
fn is_admin() -> bool {
    use windows_sys::Win32::Security::{
        AllocateAndInitializeSid, CheckTokenMembership, FreeSid, SID_IDENTIFIER_AUTHORITY,
    };

    // Well-known SID values from the Windows ABI:
    //   SECURITY_NT_AUTHORITY        = { 0, 0, 0, 0, 0, 5 }
    //   SECURITY_BUILTIN_DOMAIN_RID  = 32
    //   DOMAIN_ALIAS_RID_ADMINS      = 544
    // Hardcoded here so we don't depend on a particular windows-sys path
    // (these constants have moved between modules across versions).
    const NT_AUTHORITY: [u8; 6] = [0, 0, 0, 0, 0, 5];
    const BUILTIN_DOMAIN_RID: u32 = 32;
    const ADMINS_RID: u32 = 544;

    let nt_authority = SID_IDENTIFIER_AUTHORITY {
        Value: NT_AUTHORITY,
    };
    let mut admin_group = std::ptr::null_mut();

    unsafe {
        if AllocateAndInitializeSid(
            &nt_authority,
            2,
            BUILTIN_DOMAIN_RID,
            ADMINS_RID,
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
    // Non-admin Npcap install creates an "Npcap" local group. Silence the
    // child's stdout/stderr so the CLI doesn't print
    //   System error 1376 has occurred.
    //   The specified local group does not exist.
    // when Npcap was installed in the (default) admin-only mode, and pass
    // CREATE_NO_WINDOW so the GUI build doesn't flash a console window.
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    Command::new("net")
        .args(["localgroup", "Npcap"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
