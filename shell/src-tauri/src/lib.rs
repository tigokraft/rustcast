use app_loader::Registry;
use core_proto::RemoteInput;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
fn get_available_apps(state: tauri::State<Arc<Mutex<Registry>>>) -> Vec<AppInfo> {
    let registry = state.lock().unwrap();
    registry
        .apps
        .values()
        .map(|a| AppInfo {
            id: a.id.clone(),
            name: a.name.clone(),
        })
        .collect()
}

#[tauri::command]
fn get_active_app(state: tauri::State<Arc<Mutex<Registry>>>) -> Option<String> {
    let registry = state.lock().unwrap();
    registry.active_app.clone()
}

#[tauri::command]
fn set_active_app(state: tauri::State<Arc<Mutex<Registry>>>, app_id: String) -> String {
    let mut registry = state.lock().unwrap();
    registry.set_active(&app_id);
    if let Some(app) = registry.active_app() {
        use core_proto::VibeApp;
        app.render()
    } else {
        "{\"type\": \"container\", \"children\": []}".to_string()
    }
}

#[tauri::command]
fn send_remote_input(state: tauri::State<Arc<Mutex<Registry>>>, input: String) -> String {
    let mut registry = state.lock().unwrap();
    let remote_input = match input.as_str() {
        "Up" => RemoteInput::Up,
        "Down" => RemoteInput::Down,
        "Left" => RemoteInput::Left,
        "Right" => RemoteInput::Right,
        "Select" => RemoteInput::Select,
        "Back" => RemoteInput::Back,
        "PlayPause" => RemoteInput::PlayPause,
        "VolumeUp" => RemoteInput::VolumeUp,
        "VolumeDown" => RemoteInput::VolumeDown,
        _ => return "{\"type\": \"container\", \"children\": []}".to_string(),
    };

    if let Some(app) = registry.active_app_mut() {
        use core_proto::VibeApp;
        app.handle_input(remote_input);
        app.render()
    } else {
        "{\"type\": \"container\", \"children\": []}".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut registry = Registry::new();
    if let Err(e) = registry.scan_and_load("./apps") {
        eprintln!("Failed to scan apps dir: {}", e);
    }

    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                use notify::{Watcher, RecursiveMode};
                let (tx, rx) = std::sync::mpsc::channel();
                if let Ok(mut watcher) = notify::recommended_watcher(tx) {
                    let theme_path = std::path::Path::new("/home/exxo/code/RustCast/apps/theme.json");
                    if watcher.watch(theme_path, RecursiveMode::NonRecursive).is_ok() {
                        println!("Watch established for theme.json (Dynamic Loader)");
                    }

                    for res in rx {
                        if let Ok(event) = res {
                            if matches!(event.kind, notify::EventKind::Modify(_) | notify::EventKind::Create(_)) {
                                if let Ok(content) = std::fs::read_to_string(theme_path) {
                                    if let Ok(theme) = serde_json::from_str::<core_proto::VibeTheme>(&content) {
                                        use tauri::Emitter;
                                        let _ = handle.emit("theme-update", theme);
                                    }
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        })
        .manage(Arc::new(Mutex::new(registry)))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_available_apps,
            get_active_app,
            set_active_app,
            send_remote_input
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
