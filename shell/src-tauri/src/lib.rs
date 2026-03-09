pub mod app_loader;
use std::sync::{Arc, Mutex};
use app_loader::Registry;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut registry = Registry::new();
    if let Err(e) = registry.scan_and_load("./apps") {
        eprintln!("Failed to scan apps dir: {}", e);
    }

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(registry)))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
