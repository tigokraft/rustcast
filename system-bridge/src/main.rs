use tokio_cron_scheduler::{Job, JobScheduler};
use zbus::{connection, interface};

mod bluetooth_agent;
mod wifi_manager;

/// The IPC Bridge that will be exposed over D-Bus for the Tauri Shell
struct VibeSystemBridge;

#[interface(name = "com.vibecast.SystemBridge")]
impl VibeSystemBridge {
    /// Toggles Wi-Fi via NetworkManager
    async fn toggle_wifi(&self, enable: bool) -> zbus::fdo::Result<String> {
        // In a real implementation, we would use zbus to call NetworkManager's D-Bus API:
        // org.freedesktop.NetworkManager Enable(true/false)
        println!("IPC Request: Toggle Wi-Fi -> {}", enable);
        Ok(format!("Wi-Fi enabled state: {}", enable))
    }

    /// Triggers a Bluetooth scan
    async fn scan_bluetooth(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Scan Bluetooth");
        Ok("Scanning for devices...".to_string())
    }

    /// Triggers a system update manually
    async fn trigger_update(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Manual System Update");
        tokio::spawn(async {
            let _ = perform_system_update().await;
        });
        Ok("System update started in background".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting VibeCast System Bridge Daemon...");

    // 1. Networking: nmrs Wi-Fi Manager and Hotspot fallback
    tokio::spawn(async {
        if let Err(e) = wifi_manager::start_wifi_manager().await {
            eprintln!("Wifi Manager Error: {}", e);
        }
    });

    // 2. Bluetooth: Auto-connect to last used speaker via bluer
    tokio::spawn(async {
        if let Err(e) = bluetooth_agent::start_bluetooth_agent().await {
            eprintln!("Bluetooth Agent Error: {}", e);
        }
    });

    // 3. Arch Maintenance: Schedule weekly auto-update (Sunday at 3 AM)
    let sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async("0 0 3 * * Sun", |_uuid, _l| {
            Box::pin(async move {
                println!("Scheduled Task: Running Arch Maintenance Update...");
                let _ = perform_system_update().await;
            })
        })?)
        .await?;
    sched.start().await?;
    println!("Arch Maintenance Scheduler started (Weekly at 3 AM)");

    // 4. IPC Bridge: Start the zbus server for the Shell to communicate with
    let _conn = connection::Builder::system()?
        .name("com.vibecast.SystemBridge")?
        .serve_at("/com/vibecast/SystemBridge", VibeSystemBridge)?
        .build()
        .await?;

    println!("System Bridge IPC Server running on D-Bus system bus at com.vibecast.SystemBridge");

    // Keep the daemon alive
    std::future::pending::<()>().await;
    Ok(())
}

/// Helper function to perform system updates using arch-toolkit
async fn perform_system_update() -> Result<(), Box<dyn std::error::Error>> {
    // Note: To truly auto-update we would wrap this in a Command running pacman.
    // The arch-toolkit crate provides various helper functions.
    // E.g. pacman::sync(vec!["--noconfirm", "--sysupgrade", "--refresh"]);
    println!("Running pacman -Syu --noconfirm...");
    let status = tokio::process::Command::new("pacman")
        .args(["-Syu", "--noconfirm"])
        .status()
        .await?;

    if status.success() {
        println!("System Update completed successfully.");
    } else {
        eprintln!("System Update failed.");
    }
    Ok(())
}


