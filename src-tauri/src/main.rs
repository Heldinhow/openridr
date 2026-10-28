#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use openridr_host::{Host, HostError};
use serde_json::Value;
use tauri::State;

#[tauri::command]
fn host_call(method: String, params: Value, host: State<'_, Host>) -> Result<Value, HostError> {
    host.dispatch(&method, params)
}

fn main() {
    tauri::Builder::default()
        .manage(Host::new())
        .invoke_handler(tauri::generate_handler![host_call])
        .run(tauri::generate_context!())
        .expect("OpenRidr failed to start");
}
