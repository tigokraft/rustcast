use bluer::agent::Agent;
use bluer::{Address, Session};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

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
