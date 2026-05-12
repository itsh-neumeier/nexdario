use crate::{auth::AuthUser, error::AppError, permissions::*, services::{audit, backup as backup_svc}, state::AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Redirect},
};
use std::path::PathBuf;

pub async fn index(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(BACKUP_READ)?;

    let backups = backup_svc::list_backups(&state.config.backup_dir)
        .await
        .unwrap_or_default();

    let history = sqlx::query!(
        "SELECT filename, backup_type, storage_location, status, file_size, is_encrypted, created_at
         FROM backup_history ORDER BY created_at DESC LIMIT 50"
    ).fetch_all(&state.db).await?;

    let history_list: Vec<serde_json::Value> = history.into_iter().map(|h| serde_json::json!({
        "filename": h.filename, "backup_type": h.backup_type, "storage_location": h.storage_location,
        "status": h.status, "file_size": h.file_size, "is_encrypted": h.is_encrypted != 0,
        "created_at": h.created_at,
    })).collect();

    state.render("backup/index.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        backups => backups,
        history => history_list,
        backup_dir => &state.config.backup_dir,
        can_create => auth.has_permission(BACKUP_CREATE),
        can_download => auth.has_permission(BACKUP_DOWNLOAD),
        can_restore => auth.has_permission(BACKUP_RESTORE),
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(BACKUP_CREATE)?;

    let encrypted = state.config.backup_encryption_enabled;
    let key = if encrypted {
        Some(state.config.backup_encryption_key.as_str())
    } else {
        None
    };

    let entry = backup_svc::create_backup(
        &state.config.database_url,
        &state.config.backup_dir,
        encrypted,
        key,
    )
    .await
    .map_err(|e| AppError::internal(format!("Backup failed: {}", e)))?;

    let filename = entry.filename.clone();
    let size = entry.size_bytes as i64;
    let is_enc = if entry.is_encrypted { 1i64 } else { 0i64 };

    sqlx::query!(
        "INSERT INTO backup_history (filename, file_size, backup_type, storage_location, status, is_encrypted, created_by)
         VALUES (?, ?, 'manual', 'local', 'completed', ?, ?)",
        filename, size, is_enc, auth.id
    )
    .execute(&state.db)
    .await?;

    audit::log(&state.db, Some(&auth), "create", "backup", Some(&filename),
        Some(&format!("Manual backup created: {} ({} bytes)", filename, size)), None, true).await;

    Ok(Redirect::to("/backup"))
}

pub async fn download(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(BACKUP_DOWNLOAD)?;

    // Validate filename (no path traversal)
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(AppError::bad_request("Invalid filename"));
    }

    if !filename.starts_with("nexdario_backup_") {
        return Err(AppError::bad_request("Invalid backup filename"));
    }

    let path = PathBuf::from(&state.config.backup_dir).join(&filename);
    if !path.exists() {
        return Err(AppError::NotFound);
    }

    let data = tokio::fs::read(&path)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    audit::log(&state.db, Some(&auth), "download", "backup", Some(&filename), None, None, true).await;

    let content_type = if filename.ends_with(".enc") {
        "application/octet-stream"
    } else if filename.ends_with(".gz") {
        "application/gzip"
    } else {
        "application/octet-stream"
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename)),
        ],
        data,
    ))
}

pub async fn delete_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Only superadmin can delete backups
    if !auth.is_superadmin {
        return Err(AppError::Forbidden("Nur Superadmin kann Backups löschen".to_string()));
    }

    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(AppError::bad_request("Invalid filename"));
    }

    let path = PathBuf::from(&state.config.backup_dir).join(&filename);
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;
    }

    audit::log(&state.db, Some(&auth), "delete", "backup", Some(&filename), None, None, true).await;

    Ok(Redirect::to("/backup"))
}
