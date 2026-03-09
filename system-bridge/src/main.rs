use tokio_cron_scheduler::{Job, JobScheduler};
use zbus::{connection, interface};

mod audio_manager;
mod bluetooth_agent;
mod system_manager;
mod wifi_manager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

    /// Triggers a system update manually (Check only)
    async fn check_updates(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Check System Updates");
        tokio::spawn(async {
            let _ = perform_system_update(true).await;
        });
        Ok("Checking for updates in background...".to_string())
    }

    /// Triggers a system update manually (Apply)
    async fn apply_updates(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Apply System Updates");
        tokio::spawn(async {
            let _ = perform_system_update(false).await;
        });
        Ok("Applying system updates in background...".to_string())
    }

    /// Suspends the system using logind
    async fn suspend(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Suspend System");
        tokio::spawn(async {
            let _ = tokio::process::Command::new("systemctl").arg("suspend").output().await;
        });
        Ok("System is suspending...".to_string())
    }

    /// Powers off the system using logind
    async fn power_off(&self) -> zbus::fdo::Result<String> {
        println!("IPC Request: Power Off System");
        tokio::spawn(async {
            let _ = tokio::process::Command::new("systemctl").arg("poweroff").output().await;
        });
        Ok("System is powering off...".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting VibeCast System Bridge Daemon...");

    let setup_mode = Arc::new(AtomicBool::new(false));

    // 1. Networking: nmrs Wi-Fi Manager and Hotspot fallback
    let sm_wifi = setup_mode.clone();
    tokio::spawn(async move {
        if let Err(e) = wifi_manager::start_wifi_manager(sm_wifi).await {
            eprintln!("Wifi Manager Error: {}", e);
        }
    });

    // 2. Bluetooth: Auto-connect to last used speaker via bluer
    let sm_bt = setup_mode.clone();
    tokio::spawn(async move {
        if let Err(e) = bluetooth_agent::start_bluetooth_agent(sm_bt).await {
            eprintln!("Bluetooth Agent Error: {}", e);
        }
    });

    // 3. System Manager: WebSockets config
    let sm_sys = setup_mode.clone();
    tokio::spawn(async move {
        if let Err(e) = system_manager::start_system_manager(sm_sys).await {
            eprintln!("System Manager Server Error: {}", e);
        }
    });

    // 4. Audio Manager: PipeWire `pactl subscribe` Listener
    tokio::spawn(async move {
        if let Err(e) = audio_manager::start_audio_manager().await {
            eprintln!("Audio Manager Error: {}", e);
        }
    });

    // 5. Arch Maintenance: Schedule weekly auto-update (Sunday at 3 AM)
    let sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async("0 0 3 * * Sun", |_uuid, _l| {
            Box::pin(async move {
                println!("Scheduled Task: Running Arch Maintenance Update...");
                let _ = perform_system_update(false).await;
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

/// Helper function to perform system updates using arch-toolkit or pacman directly
async fn perform_system_update(check_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    if check_only {
        println!("Running pacman -Sy to check for updates...");
        let status = tokio::process::Command::new("pacman")
            .args(["-Sy"])
            .status()
            .await?;
        
        if status.success() {
            println!("System Update Check completed successfully.");
        } else {
            eprintln!("System Update Check failed.");
        }
    } else {
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
    }
    Ok(())
}


