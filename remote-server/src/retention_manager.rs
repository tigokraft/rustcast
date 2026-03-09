use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::time::Duration;
use sysinfo::{DiskExt, System, SystemExt};
use tokio::time;

pub struct RetentionManager {
    db: Pool<Sqlite>,
    run_interval: Duration,
    max_age_days: i64,
}

impl RetentionManager {
    /// Initialize the RetentionManager with a given database URL
    pub async fn new(db_url: &str) -> std::result::Result<Self, sqlx::Error> {
        let db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;

        // Ensure the media table exists
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS media (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL UNIQUE,
                added_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&db)
        .await?;

        Ok(Self {
            db,
            run_interval: Duration::from_secs(60 * 60), // Every 60 minutes
            max_age_days: 60,
        })
    }

    /// Spawn the background retention worker task
    pub fn spawn(self) {
        tokio::spawn(async move {
            let mut interval = time::interval(self.run_interval);
            loop {
                interval.tick().await;
                if let Err(e) = self.run_retention_checks().await {
                    eprintln!("Retention check error: {}", e);
                }
            }
        });
    }

    async fn run_retention_checks(&self) -> std::result::Result<(), sqlx::Error> {
        println!("Running retention manager checks...");
        self.prune_old_media().await?;
        self.enforce_disk_limits().await?;
        Ok(())
    }

    /// Deletes movies where the timestamp is older than max_age_days
    async fn prune_old_media(&self) -> std::result::Result<(), sqlx::Error> {
        let cutoff_date = format!("-{} days", self.max_age_days);
        
        // Find paths to delete from disk (in a real app, we'd fs::remove_file these)
        let records = sqlx::query!(
            "SELECT id, file_path FROM media WHERE added_at < datetime('now', ?)",
            cutoff_date
        )
        .fetch_all(&self.db)
        .await?;

        for record in records {
            println!("Pruning expired media: {}", record.file_path);
            let _ = std::fs::remove_file(&record.file_path);
            
            sqlx::query!("DELETE FROM media WHERE id = ?", record.id)
                .execute(&self.db)
                .await?;
        }

        Ok(())
    }

    /// Deletes the oldest entries until disk utilization is under 70% if it was > 90%
    async fn enforce_disk_limits(&self) -> std::result::Result<(), sqlx::Error> {
        let mut sys = System::new_all();
        sys.refresh_disks_list();
        sys.refresh_disks();

        // Get the disk we likely care about (in this demo, the first root/storage disk)
        if let Some(disk) = sys.disks().first() {
            let total_space = disk.total_space() as f64;
            let available_space = disk.available_space() as f64;
            let used_space = total_space - available_space;
            let utilization = used_space / total_space;

            if utilization > 0.90 {
                println!("Disk utilization > 90% ({:.1}%). Pruning oldest entries...", utilization * 100.0);
                let target_utilization = 0.70;
                let bytes_to_free = ((utilization - target_utilization) * total_space) as u64;
                let mut freed_bytes = 0;

                // Delete oldest entries until we've freed enough space
                while freed_bytes < bytes_to_free {
                    // Get the single oldest entry
                    let oldest = sqlx::query!("SELECT id, file_path FROM media ORDER BY added_at ASC LIMIT 1")
                        .fetch_optional(&self.db)
                        .await?;

                    if let Some(record) = oldest {
                        if let Ok(metadata) = std::fs::metadata(&record.file_path) {
                            freed_bytes += metadata.len();
                        }
                        println!("Disk pressure prune: {}", record.file_path);
                        let _ = std::fs::remove_file(&record.file_path);

                        sqlx::query!("DELETE FROM media WHERE id = ?", record.id)
                            .execute(&self.db)
                            .await?;
                    } else {
                        // DB is empty, nothing more we can do
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
