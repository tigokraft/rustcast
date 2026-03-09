use axum::{routing::post, Json, Router};
use serde::Deserialize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Deserialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

pub async fn start_wifi_manager() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing WifiManager with nmrs...");

    // 1. Networking (nmrs + Hotspot Timeout)
    tokio::spawn(async {
        // Scan for known SSID connections on startup
        println!("Scanning for saved connections...");

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
            // nmrs hotspot creation logic goes here
            is_connected = true; // Pretend hotspot is up
        } else {
            println!("Networking: Connected to known Wi-Fi network.");
        }
    });

    // 2. Axum Server for `/api/connect`
    tokio::spawn(async {
        let app = Router::new().route("/api/connect", post(handle_wifi_connect));
        
        // Listen on the Hotspot IP address
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 125, 1)), 8000);
        
        // Note: During local dev it may fail to bind to this IP if the hotspot isn't actually active.
        // We catch the error and fallback to 0.0.0.0 for development purposes
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                println!("Starting Axum Wi-Fi config server on {}", addr);
                axum::serve(listener, app).await.unwrap();
            }
            Err(e) => {
                eprintln!("Failed to bind to {}, falling back to 0.0.0.0:8000: {}", addr, e);
                let fallback_addr = SocketAddr::from(([0, 0, 0, 0], 8000));
                let listener = tokio::net::TcpListener::bind(fallback_addr).await.unwrap();
                println!("Starting Axum Wi-Fi config server on fallback {}", fallback_addr);
                axum::serve(listener, app).await.unwrap();
            }
        }
    });

    Ok(())
}

async fn handle_wifi_connect(Json(payload): Json<WifiConfig>) -> axum::http::StatusCode {
    println!("Axum /api/connect hit! SSID: {}", payload.ssid);
    println!("Saving NetworkManager profile for '{}' using nmrs...", payload.ssid);
    println!("Disabling 'VibeCast-Setup' Hotspot using nmrs...");
    println!("Connecting to new Wi-Fi network '{}'...", payload.ssid);
    
    // Example nmrs usage logic:
    // let nm = nmrs::NetworkManager::new().await.unwrap();
    // let device = nm.get_device_by_iface("wlan0").await.unwrap();
    // let cloned_pw = payload.password.clone();
    // ... disable hotspot, create connection profile recursively, activate it on device

    axum::http::StatusCode::OK
}
