use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use core_proto::RemoteInput;
use futures_util::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub struct RemoteInputState {
    pub tx: broadcast::Sender<RemoteInput>,
}

pub async fn start_remote_input_server() -> Result<(), Box<dyn std::error::Error>> {
    // We create a broadcast channel so any connected WebSocket (like the TV Shell locally)
    // can receive all events sent by Mobile phones.
    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(RemoteInputState { tx: tx.clone() });

    let app = Router::new()
        .route("/ws/remote", get(ws_handler))
        .with_state(state);

    let port = 8002;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Remote Input WebSocket server listening on ws://{}", addr);

    // Spawn WebSocket Server
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            let _ = axum::serve(listener, app).await;
        }
    });

    // Spawn mDNS Broadcaster
    tokio::spawn(async move {
        println!("Starting mDNS Broadcaster for _vibecast._tcp.local...");
        let mdns = ServiceDaemon::new().expect("Failed to create mDNS daemon");
        let service_type = "_vibecast._tcp.local.";
        let instance_name = "VibeCast-TV";
        
        let ip_addr = "0.0.0.0"; // We advertise 0.0.0.0, the resolver will see the actual IP
        let host_name = "vibecast-tv.local.";

        let properties: HashMap<String, String> = HashMap::new();
        let my_service = ServiceInfo::new(
            service_type,
            instance_name,
            host_name,
            ip_addr,
            port,
            Some(properties),
        )
        .expect("Failed to create mDNS service info")
        .enable_addr_auto();

        mdns.register(my_service).expect("Failed to register mDNS service");
        
        // Block to keep mDNS alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RemoteInputState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<RemoteInputState>) {
    println!("Client connected to Remote Input WebSocket");
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to the broadcast channel so we can forward events FROM the stream if needed,
    // or if we are the shell, we consume them.
    let mut rx = state.tx.subscribe();

    // Task that forwards broadcasted RemoteInputs to this websocket client 
    // (This is how the local Shell receives events from Mobile phones)
    let mut forward_task = tokio::spawn(async move {
        while let Ok(input) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&input) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task that reads incoming messages from this websocket client 
    // and broadcasts them (This is how Mobile phones send events to the TV)
    let tx_clone = state.tx.clone();
    let mut read_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(input) = serde_json::from_str::<RemoteInput>(&text) {
                let _ = tx_clone.send(input);
            }
        }
    });

    // Wait for either task to finish (e.g. client disconnects)
    tokio::select! {
        _ = (&mut forward_task) => read_task.abort(),
        _ = (&mut read_task) => forward_task.abort(),
    };

    println!("Client disconnected from Remote Input WebSocket");
}
