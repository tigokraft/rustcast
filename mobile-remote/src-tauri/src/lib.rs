use mdns_sd::{ServiceDaemon, ServiceEvent};
use tauri::Manager;

#[tauri::command]
fn get_mock_tv_ip() -> String {
    // Fallback if mDNS fails during local development
    "127.0.0.1:8002".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");
                let service_type = "_vibecast._tcp.local.";
                let receiver = mdns.browse(service_type).expect("Failed to browse mDNS");
                
                println!("Searching for TV on network...");
                while let Ok(event) = receiver.recv() {
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            if let Some(ip) = info.get_addresses().iter().next() {
                                let target_uri = format!("{}:{}", ip, info.get_port());
                                println!("Found TV at {}", target_uri);
                                let _ = handle.emit("tv-discovered", target_uri);
                                break; // We found the TV, no need to keep browsing right now
                            }
                        }
                        _ => {}
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_mock_tv_ip])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
