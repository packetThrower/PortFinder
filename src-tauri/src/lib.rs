mod capture;
mod privilege;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub switch_name: String,
    pub switch_ip: String,
    pub switch_port: String,
    pub native_vlan: String,
    pub voice_vlan: String,
    pub switch_model: String,
}

/// Holds the cancellation token for the currently running capture (if any).
/// `start_capture` cancels the previous token and replaces it; `stop_capture`
/// cancels whatever's there.
#[derive(Default)]
struct CaptureState(Mutex<Option<CancellationToken>>);

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    capture::list_interfaces()
}

#[tauri::command]
async fn start_capture(
    request: CaptureRequest,
    state: tauri::State<'_, CaptureState>,
) -> Result<CaptureResult, String> {
    let cancel = CancellationToken::new();
    {
        let mut current = state.0.lock().await;
        if let Some(prev) = current.take() {
            prev.cancel();
        }
        *current = Some(cancel.clone());
    }

    let result = capture::run(request, cancel.clone()).await;

    // Clear the slot if it's still our token.
    let mut current = state.0.lock().await;
    if let Some(ref tok) = *current {
        if tok.is_cancelled() || std::ptr::eq(tok as *const _, &cancel as *const _) {
            *current = None;
        }
    }
    result
}

#[tauri::command]
async fn stop_capture(state: tauri::State<'_, CaptureState>) -> Result<(), String> {
    let mut current = state.0.lock().await;
    if let Some(tok) = current.take() {
        tok.cancel();
    }
    Ok(())
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
        .manage(CaptureState::default())
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
