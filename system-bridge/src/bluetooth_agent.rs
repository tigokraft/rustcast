use bluer::agent::Agent;
use bluer::{Address, Session};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use tokio_stream::StreamExt;

#[derive(Serialize, Deserialize, Default)]
pub struct BtConfig {
    pub last_used_speaker: Option<Address>,
}

pub async fn start_bluetooth_agent() -> Result<(), Box<dyn std::error::Error>> {
    let session = Session::new().await?;
    let _adapter = session.default_adapter().await?;

    // Register a pairing agent that allows any device to pair without a PIN
    let agent = Agent {
        request_default: true,
        request_pin_code: None,
        display_pin_code: None,
        request_passkey: None,
        display_passkey: None,
        request_confirmation: Some(Box::new(|req| {
            println!("Agent: Auto-confirming pairing request for {}", req.device);
            Box::pin(async move { Ok(()) })
        })),
        request_authorization: Some(Box::new(|req| {
            println!("Agent: Auto-authorizing pairing request for {}", req.device);
            Box::pin(async move { Ok(()) })
        })),
        authorize_service: Some(Box::new(|req| {
            println!("Agent: Auto-authorizing service target for {}", req.device);
            Box::pin(async move { Ok(()) })
        })),
        ..Default::default()
    };

    println!("Registering NoInputNoOutput built-in Bluetooth Agent...");
    // Keep the agent handle alive
    let _agent_handle = session.register_agent(agent).await?;

    // Spawn the PipeWire sink switcher listener
    let session_clone = session.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_device_connections(&session_clone).await {
            eprintln!("PipeWire Listener Error: {}", e);
        }
    });

    auto_pair_loop(&session).await?;

    Ok(())
}

async fn auto_pair_loop(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = Path::new("/etc/vibecast/bt_config.json");
    
    // Ensure config directory exists
    if let Some(parent) = config_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    loop {
        if let Ok(config_str) = fs::read_to_string(config_path) {
            if let Ok(config) = serde_json::from_str::<BtConfig>(&config_str) {
                if let Some(speaker_addr) = config.last_used_speaker {
                    if let Ok(adapter) = session.default_adapter().await {
                        let _ = adapter.set_powered(true).await;
                        if let Ok(device) = adapter.device(speaker_addr) {
                            let is_connected = device.is_connected().await.unwrap_or(false);
                            if !is_connected {
                                println!("Reconnect loop: connecting to speaker {}", speaker_addr);
                                // The agent will handle the pairing automatically if it's not
                                let _ = device.connect().await;
                                
                                // Automatically trust it
                                if let Ok(false) = device.is_trusted().await {
                                    println!("Auto-trusting mobile remote device: {}", speaker_addr);
                                    let _ = device.set_trusted(true).await;
                                }
                            }
                        }
                    }
                }
            }
        }
        sleep(Duration::from_secs(10)).await;
    }
}

async fn listen_for_device_connections(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let adapter = session.default_adapter().await?;
    let mut adapter_events = adapter.events().await?;
    
    // Listen to existing devices
    for addr in adapter.device_addresses().await? {
        if let Ok(device) = adapter.device(addr) {
            spawn_device_listener(device);
        }
    }

    // Listen for new devices being added
    while let Some(evt) = adapter_events.next().await {
        if let bluer::AdapterEvent::DeviceAdded(addr) = evt {
            if let Ok(device) = adapter.device(addr) {
                spawn_device_listener(device);
            }
        }
    }
    
    Ok(())
}

fn spawn_device_listener(device: bluer::Device) {
    tokio::spawn(async move {
        if let Ok(mut device_events) = device.events().await {
            while let Some(evt) = device_events.next().await {
                if let bluer::DeviceEvent::PropertyChanged(bluer::DeviceProperty::Connected(true)) = evt {
                    println!("Bluetooth Audio Device connected: {}", device.address());
                    // Auto-switch PipeWire sink
                    println!("Switching PipeWire default sink to Bluetooth output...");
                    // Give pipewire/wireplumber a brief moment to register the new bluez node
                    sleep(Duration::from_secs(2)).await;
                    
                    // In a production environment, we'd use wpctl status to find the exact node ID of the Bluetooth sink
                    // For this environment demo as designated by arch architecture, we can call wpctl to set default 50
                    // Or more dynamically: wpctl set-default $(wpctl status | grep -i bluez | head -n1 | grep -o '[0-9]\+')
                    let _ = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg("wpctl set-default $(wpctl status | grep -i bluez | grep Sink | head -n1 | grep -o '[0-9]\\+')")
                        .spawn();
                }
            }
        }
    });
}
