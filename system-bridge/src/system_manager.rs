use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Serialize, Clone)]
pub struct SystemStatus {
    pub setup_mode_active: bool,
    pub ip_address: String,
    pub bt_status: String,
    pub signal_strength: u8,
}

pub async fn start_system_manager(setup_mode: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing SystemManager WebSocket Server...");

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(setup_mode);

    // Run on port 8001 so it doesn't conflict with wifi hotspot provision server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8001));
    println!("SystemManager WebSocket listening on ws://{}", addr);

    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            let _ = axum::serve(listener, app).await;
        }
    });

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(setup_mode): axum::extract::State<Arc<AtomicBool>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, setup_mode))
}

async fn handle_socket(mut socket: WebSocket, setup_mode: Arc<AtomicBool>) {
    println!("Client connected to SystemManager WebSocket!");
    
    // Broadcast loop every 5 seconds
    loop {
        // Here we would use nmrs to retrieve the real IP and Signal Strength
        // For now, we mock it.
        let status = SystemStatus {
            setup_mode_active: setup_mode.load(Ordering::Relaxed),
            ip_address: "192.168.1.10".to_string(), // Mock
            bt_status: "Active".to_string(),
            signal_strength: 85,
        };

        if let Ok(json_str) = serde_json::to_string(&status) {
            if socket.send(Message::Text(json_str.into())).await.is_err() {
                println!("SystemManager WebSocket client disconnected");
                break;
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}
