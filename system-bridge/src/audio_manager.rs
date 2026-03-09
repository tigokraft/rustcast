use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub async fn start_audio_manager() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing AudioManager with pactl subscribe...");

    tokio::spawn(async move {
        // First ensure we are on the correct sink initially
        set_optimal_sink().await;

        let mut child = Command::new("pactl")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start pactl subscribe for audio monitoring");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let mut reader = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            // "Event 'new' on sink #4" or "Event 'remove' on sink #4"
            if line.contains("Event 'new' on sink") || line.contains("Event 'remove' on sink") {
                // Give PipeWire/WirePlumber a moment to settle routes before querying
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                set_optimal_sink().await;
            }
        }
    });

    Ok(())
}

async fn set_optimal_sink() {
    let output = Command::new("pactl")
        .args(&["list", "short", "sinks"])
        .output()
        .await
        .expect("Failed to run pactl");

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    
    let mut bluez_sink_id = None;
    let mut alsa_sink_id = None;

    // Example output from `pactl list short sinks`:
    // 40      alsa_output.pci-0000_00_1f.3.analog-stereo      PipeWire        s16le 2ch 48000Hz       SUSPENDED
    // 59      bluez_sink.XX_XX_XX_XX_XX_XX.a2dp_sink          PipeWire        s16le 2ch 48000Hz       SUSPENDED

    for line in stdout_str.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let id = parts[0];
            let name = parts[1];
            if name.contains("bluez_sink") {
                bluez_sink_id = Some(id.to_string());
            } else if name.contains("alsa_output") {
                alsa_sink_id = Some(id.to_string());
            }
        }
    }

    if let Some(id) = bluez_sink_id {
        println!("AudioManager: detected Bluetooth sink, routing audio to ID {}", id);
        let _ = Command::new("wpctl").args(&["set-default", &id]).output().await;
    } else if let Some(id) = alsa_sink_id {
        println!("AudioManager: no Bluetooth sink, routing audio to TV speakers ID {}", id);
        let _ = Command::new("wpctl").args(&["set-default", &id]).output().await;
    } else {
        println!("AudioManager: no valid sinks found to route audio");
    }
}
