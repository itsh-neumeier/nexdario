use crate::{config::Config, services};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub fn spawn_background_jobs(pool: SqlitePool, config: Arc<Config>) {
    // Session cleanup job — every hour
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(3600));
        loop {
            tick.tick().await;
            match crate::auth::delete_expired_sessions(&pool_clone).await {
                Ok(n) if n > 0 => tracing::info!("Cleaned {} expired sessions", n),
                Ok(_) => {}
                Err(e) => tracing::error!("Session cleanup error: {}", e),
            }
        }
    });

    // Automatic backup job
    if config.backup_enabled {
        let pool_clone = pool.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            let interval_secs = config_clone.backup_interval_hours * 3600;
            // First run after initial delay of 5 minutes
            tokio::time::sleep(Duration::from_secs(300)).await;

            let mut tick = interval(Duration::from_secs(interval_secs));
            loop {
                tick.tick().await;

                let db_url = &config_clone.database_url;
                let backup_dir = &config_clone.backup_dir;
                let encrypted = config_clone.backup_encryption_enabled;
                let key = if encrypted {
                    Some(config_clone.backup_encryption_key.as_str())
                } else {
                    None
                };

                match services::backup::create_backup(db_url, backup_dir, encrypted, key).await {
                    Ok(entry) => {
                        tracing::info!("Automatic backup created: {}", entry.filename);

                        // Record in DB
                        let filename = entry.filename.clone();
                        let size = entry.size_bytes as i64;
                        let is_enc = if entry.is_encrypted { 1i64 } else { 0i64 };
                        sqlx::query!(
                            "INSERT INTO backup_history (filename, file_size, backup_type, storage_location, status, is_encrypted)
                             VALUES (?, ?, 'automatic', 'local', 'completed', ?)",
                            filename, size, is_enc
                        )
                        .execute(&pool_clone)
                        .await
                        .ok();

                        // Clean old backups
                        let retention = config_clone.backup_retention_local_days;
                        services::backup::delete_old_backups(backup_dir, retention).await.ok();
                    }
                    Err(e) => tracing::error!("Automatic backup failed: {}", e),
                }
            }
        });
    }
}
