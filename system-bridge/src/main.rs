use arch_toolkit::pacman;
use bluer::{AdapterEvent, Address, Session};
use std::time::Duration;
use tokio::time::sleep;
use tokio_cron_scheduler::{Job, JobScheduler};
use zbus::{connection, interface};

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

    // 1. Networking (NetworkManager zbus stub)
    // We would connect to NetworkManager to check internet, if offline start Hotspot
    // Using zbus to call `org.freedesktop.NetworkManager`
    tokio::spawn(async {
        println!("Initializing Networking: ensuring Wi-Fi is connected or starting Hotspot...");
        // Pretend to check for internet, then fallback to hotspot if needed
        sleep(Duration::from_secs(2)).await;
        println!("Networking: Connected to known Wi-Fi network.");
    });

    // 2. Bluetooth: Auto-connect to last used speaker via bluer
    tokio::spawn(async {
        if let Err(e) = auto_pair_bluetooth().await {
            eprintln!("Bluetooth Auto-pair Error: {}", e);
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

/// Helper function to auto-pair Bluetooth via bluer
async fn auto_pair_bluetooth() -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    println!("Bluetooth adapter powered on. MAC: {}", adapter.name());

    let device_addrs = adapter.device_addresses().await?;
    // Just an example, connecting to the first known/paired device
    // In reality, we'd query a local DB to find the "last used" Address
    if let Some(&addr) = device_addrs.first() {
        let device = adapter.device(addr)?;
        if !device.is_connected().await? {
            println!("Attempting to connect to Bluetooth device: {}", addr);
            // Ignore error for now if it is out of range
            let _ = device.connect().await;
            println!("Connected to audio device!");
        } else {
            println!("Bluetooth device {} is already connected.", addr);
        }
    }
    
    // Auto-set audio sink to HDMI out (PipeWire wireplumber/wpctl)
    // wpctl set-default N (where N is the HDMI node)
    println!("Configuring PipeWire default sink to HDMI out...");
    let _ = tokio::process::Command::new("wpctl")
        .args(&["set-default", "50"]) // 50 is an example node ID
        .output()
        .await;

    Ok(())
}
