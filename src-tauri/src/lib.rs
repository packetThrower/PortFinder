mod privilege;

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
    privilege::has_capture_privilege()
}

#[tauri::command]
fn get_privilege_status() -> privilege::PrivilegeStatus {
    privilege::get_privilege_status()
}

#[tauri::command]
fn install_bpf_helper() -> Result<(), String> {
    privilege::install_bpf_helper()
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
