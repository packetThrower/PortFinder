use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInfo {
    pub name: String,
    pub description: String,
    pub addresses: String,
    pub has_ip: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub interface_name: String,
    pub protocol: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub switch_name: String,
    pub switch_ip: String,
    pub switch_port: String,
    pub native_vlan: String,
    pub voice_vlan: String,
    pub switch_model: String,
}

#[derive(Serialize)]
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

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    Err("get_interfaces: not yet implemented (Phase 4)".into())
}

#[tauri::command]
fn start_capture(_request: CaptureRequest) -> Result<Option<CaptureResult>, String> {
    Err("start_capture: not yet implemented (Phase 4)".into())
}

#[tauri::command]
fn stop_capture() -> Result<(), String> {
    Err("stop_capture: not yet implemented (Phase 4)".into())
}

#[tauri::command]
fn check_privileges() -> bool {
    false
}

#[tauri::command]
fn get_privilege_status() -> PrivilegeStatus {
    PrivilegeStatus {
        has_access: false,
        helper_installed: false,
        in_bpf_group: false,
        can_install: false,
        platform: std::env::consts::OS.to_string(),
        npcap_installed: false,
        npcap_non_admin: false,
    }
}

#[tauri::command]
fn install_bpf_helper() -> Result<(), String> {
    Err("install_bpf_helper: not yet implemented (Phase 5)".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_version,
            get_interfaces,
            start_capture,
            stop_capture,
            check_privileges,
            get_privilege_status,
            install_bpf_helper,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
