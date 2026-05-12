use crate::auth::AuthUser;
use sqlx::SqlitePool;

pub async fn log(
    pool: &SqlitePool,
    user: Option<&AuthUser>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    details: Option<&str>,
    ip: Option<&str>,
    success: bool,
) {
    let (user_id, username) = match user {
        Some(u) => (Some(u.id), Some(u.username.as_str())),
        None => (None, None),
    };

    let success_int = if success { 1i64 } else { 0i64 };

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_log (user_id, username, action, resource_type, resource_id, details, ip_address, success)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        user_id,
        username,
        action,
        resource_type,
        resource_id,
        details,
        ip,
        success_int
    )
    .execute(pool)
    .await
    {
        tracing::error!("Audit log failed: {}", e);
    }
}

pub async fn log_login(pool: &SqlitePool, user_id: i64, username: &str, ip: Option<&str>, success: bool) {
    let success_int = if success { 1i64 } else { 0i64 };
    let action = if success { "login" } else { "login_failed" };
    let id_str = user_id.to_string();

    if let Err(e) = sqlx::query!(
        "INSERT INTO audit_log (user_id, username, action, resource_type, resource_id, ip_address, success)
         VALUES (?, ?, ?, 'user', ?, ?, ?)",
        user_id,
        username,
        action,
        id_str,
        ip,
        success_int
    )
    .execute(pool)
    .await
    {
        tracing::error!("Audit log failed: {}", e);
    }
}
