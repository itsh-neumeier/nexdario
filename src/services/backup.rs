use crate::config::Config;
use anyhow::Context;
use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize)]
pub struct BackupEntry {
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String,
    pub checksum: Option<String>,
    pub is_encrypted: bool,
}

pub async fn create_backup(
    db_path: &str,
    backup_dir: &str,
    encrypted: bool,
    encryption_key: Option<&str>,
) -> anyhow::Result<BackupEntry> {
    tokio::fs::create_dir_all(backup_dir).await?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let base_name = format!("nexdario_backup_{}", timestamp);

    // Read source DB
    let db_path_clean = db_path
        .strip_prefix("sqlite:")
        .unwrap_or(db_path);

    let db_data = tokio::fs::read(db_path_clean)
        .await
        .context("Failed to read database file")?;

    // Compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&db_data)?;
    let compressed = encoder.finish()?;

    let (filename, final_data) = if encrypted {
        if let Some(key) = encryption_key {
            use aes_gcm::{aead::{Aead, AeadCore, KeyInit, OsRng}, Aes256Gcm, Key};
            use sha2::Sha256 as Sha256Enc;
            use sha2::Digest as DigestEnc;

            let mut hasher = Sha256Enc::new();
            hasher.update(key.as_bytes());
            let key_bytes: [u8; 32] = hasher.finalize().into();
            let aes_key = Key::<Aes256Gcm>::from_slice(&key_bytes);
            let cipher = Aes256Gcm::new(aes_key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

            let ciphertext = cipher
                .encrypt(&nonce, compressed.as_ref())
                .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

            let mut combined = Vec::with_capacity(nonce.len() + ciphertext.len());
            combined.extend_from_slice(&nonce);
            combined.extend_from_slice(&ciphertext);

            (format!("{}.sqlite.gz.enc", base_name), combined)
        } else {
            (format!("{}.sqlite.gz", base_name), compressed)
        }
    } else {
        (format!("{}.sqlite.gz", base_name), compressed)
    };

    let backup_path = PathBuf::from(backup_dir).join(&filename);
    tokio::fs::write(&backup_path, &final_data)
        .await
        .context("Failed to write backup file")?;

    // Calculate checksum
    let mut hasher = Sha256::new();
    hasher.update(&final_data);
    let checksum = hex::encode(hasher.finalize());

    let size_bytes = final_data.len() as u64;
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(BackupEntry {
        filename,
        size_bytes,
        created_at,
        checksum: Some(checksum),
        is_encrypted: encrypted && encryption_key.is_some(),
    })
}

pub async fn list_backups(backup_dir: &str) -> anyhow::Result<Vec<BackupEntry>> {
    let mut entries = Vec::new();

    let dir_path = Path::new(backup_dir);
    if !dir_path.exists() {
        return Ok(entries);
    }

    let mut read_dir = tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if !filename.starts_with("nexdario_backup_") {
            continue;
        }

        let metadata = tokio::fs::metadata(&path).await?;
        let size_bytes = metadata.len();

        let created_at = metadata
            .modified()
            .ok()
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                        .unwrap_or_default();
                    dt.to_rfc3339()
                })
            })
            .unwrap_or_default();

        let is_encrypted = filename.ends_with(".enc");

        entries.push(BackupEntry {
            filename,
            size_bytes,
            created_at,
            checksum: None,
            is_encrypted,
        });
    }

    // Sort newest first
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(entries)
}

pub async fn delete_old_backups(backup_dir: &str, retention_days: u64) -> anyhow::Result<u32> {
    let cutoff = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(retention_days as i64))
        .unwrap();

    let dir_path = Path::new(backup_dir);
    if !dir_path.exists() {
        return Ok(0);
    }

    let mut deleted = 0u32;
    let mut read_dir = tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !filename.starts_with("nexdario_backup_") {
            continue;
        }

        let metadata = tokio::fs::metadata(&path).await?;
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                let file_time =
                    chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0)
                        .unwrap_or_default();
                if file_time < cutoff {
                    tokio::fs::remove_file(&path).await.ok();
                    deleted += 1;
                }
            }
        }
    }

    Ok(deleted)
}
