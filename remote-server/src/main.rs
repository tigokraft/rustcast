mod retention_manager;
use retention_manager::RetentionManager;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    println!("Hello, world!");
    let manager = RetentionManager::new("sqlite::memory:").await?;
    manager.spawn();

    // Keep the main thread alive since the retention manager runs in a background task
    tokio::signal::ctrl_c().await.unwrap();
    println!("Shutting down remote server...");

    Ok(())
}
