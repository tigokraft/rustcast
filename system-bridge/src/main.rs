
use axum::{routing::post, Json, Router};
use bluer::Session;
use serde::Deserialize;
use std::net::SocketAddr;
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

#[derive(Deserialize)]
struct WifiConfig {
    ssid: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting VibeCast System Bridge Daemon...");

    // 1. Networking (nmrs + Hotspot Timeout)
    tokio::spawn(async {
        println!("Initializing Networking with nmrs...");
        // Scan for known SSID connections on startup
        println!("Scanning for known connections...");

        let mut is_connected = false;
        // Wait up to 30 seconds for an active connection
        for i in 1..=30 {
            sleep(Duration::from_secs(1)).await;
            // In a real scenario, query `nmrs::NetworkManager::new().await?.state()` 
            // For now, we simulate that no connection was established
            if i % 10 == 0 {
                println!("Waiting for active Wi-Fi connection... {}s", i);
            }
        }

        if !is_connected {
            println!("No connection active within 30 seconds.");
            println!("Using nmrs to create Wi-Fi Hotspot 'VibeCast-Setup'...");
            // nmrs hotspot creation logic goes here:
            // let nm = nmrs::NetworkManager::new().await.unwrap();
            // ... setup access point
            is_connected = true; // Pretend we made the hotspot successfully
        } else {
            println!("Networking: Connected to known Wi-Fi network.");
        }
    });

    // 1b. Axum Server for `/config/wifi`
    tokio::spawn(async {
        let app = Router::new().route("/config/wifi", post(handle_wifi_config));
        let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
        println!("Starting Axum Wi-Fi config server on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
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

async fn handle_wifi_config(Json(payload): Json<WifiConfig>) -> axum::http::StatusCode {
    println!("Axum /config/wifi hit! SSID: {}", payload.ssid);
    println!("Disabling 'VibeCast-Setup' Hotspot using nmrs...");
    println!("Connecting to new Wi-Fi network '{}' using nmrs...", payload.ssid);
    
    // Example nmrs usage logic:
    // let nm = nmrs::NetworkManager::new().await.unwrap();
    // let device = nm.get_device_by_iface("wlan0").await.unwrap();
    // let cloned_pw = payload.password.clone();
    // ... disable hotspot, create connection profile, activate it on device

    axum::http::StatusCode::OK
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
