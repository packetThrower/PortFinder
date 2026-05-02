use serde::Serialize;
use specta::Type;

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(target_os = "macos")]
mod install_darwin;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use darwin as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[derive(Serialize, Default, Type)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeStatus {
    pub has_access: bool,
    pub helper_installed: bool,
    pub in_bpf_group: bool,
    pub can_install: bool,
    pub platform: String,
    pub npcap_installed: bool,
    pub npcap_non_admin: bool,
}

pub fn has_capture_privilege() -> bool {
    platform::has_platform_privilege()
}

pub fn get_privilege_status() -> PrivilegeStatus {
    let mut status = PrivilegeStatus {
        has_access: platform::has_platform_privilege(),
        platform: std::env::consts::OS.to_string(),
        ..Default::default()
    };
    platform::fill_platform_status(&mut status);
    status
}

pub fn install_bpf_helper() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        install_darwin::install()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("install_bpf_helper is only available on macOS".into())
    }
}

#[cfg(unix)]
pub(crate) fn is_root() -> bool {
    nix::unistd::Uid::effective().is_root()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_get_privilege_status() {
        let status = get_privilege_status();
        // Whatever the platform, we should at least get a populated platform
        // string and a deterministic shape.
        assert!(!status.platform.is_empty());
        // Print so `cargo test -- --nocapture` shows the live result.
        println!("status: {}", serde_json::to_string_pretty(&status).unwrap());
    }
}
