mod retention_manager;
mod remote_input;
use retention_manager::RetentionManager;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    println!("Hello, world!");
    let manager = RetentionManager::new("sqlite::memory:").await?;
    manager.spawn();

    if let Err(e) = remote_input::start_remote_input_server().await {
        eprintln!("Failed to start remote_input server: {}", e);
    }

    // Keep the main thread alive since the retention manager runs in a background task
    tokio::signal::ctrl_c().await.unwrap();
    println!("Shutting down remote server...");

    Ok(())
}
