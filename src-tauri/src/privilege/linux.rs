use super::PrivilegeStatus;

pub fn has_platform_privilege() -> bool {
    if super::is_root() {
        return true;
    }
    // Check for any non-zero CAP_EFFECTIVE capabilities (CAP_NET_RAW etc.)
    let Ok(data) = std::fs::read_to_string("/proc/self/status") else {
        return false;
    };
    for line in data.lines() {
        if let Some(caps) = line.strip_prefix("CapEff:") {
            let caps = caps.trim();
            return !caps.is_empty() && caps != "0000000000000000";
        }
    }
    false
}

pub fn fill_platform_status(_status: &mut PrivilegeStatus) {
    // Linux has no helper to install — privileges come from setcap during
    // package install or from running as root.
}
