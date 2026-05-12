use crate::{error::AppError, state::AppState};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum::extract::FromRef;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub permissions: Vec<String>,
    pub roles: Vec<String>,
    pub is_superadmin: bool,
    #[serde(skip)]
    pub permissions_set: HashSet<String>,
}

impl AuthUser {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.is_superadmin || self.permissions_set.contains(perm)
    }

    pub fn require_permission(&self, perm: &str) -> Result<(), AppError> {
        if self.has_permission(perm) {
            Ok(())
        } else {
            Err(AppError::Forbidden(format!(
                "Fehlende Berechtigung: {}",
                perm
            )))
        }
    }
}

// Optional auth user — does not redirect if missing
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_session_token(parts);

        match token {
            Some(t) => match load_auth_user(&app_state.db, &t).await {
                Ok(Some(user)) => Ok(user),
                Ok(None) => Err(Redirect::to("/login").into_response()),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
            },
            None => Err(Redirect::to("/login").into_response()),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let token = extract_session_token(parts);

        match token {
            Some(t) => match load_auth_user(&app_state.db, &t).await {
                Ok(user) => Ok(OptionalAuthUser(user)),
                Err(_) => Ok(OptionalAuthUser(None)),
            },
            None => Ok(OptionalAuthUser(None)),
        }
    }
}

pub fn extract_session_token(parts: &Parts) -> Option<String> {
    let header = parts
        .headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    header
        .split(';')
        .filter_map(|s| {
            let s = s.trim();
            s.strip_prefix("nxd_session=").map(|v| v.to_string())
        })
        .next()
}

pub async fn load_auth_user(pool: &SqlitePool, token: &str) -> anyhow::Result<Option<AuthUser>> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let row = sqlx::query!(
        "SELECT s.user_id FROM sessions s
         WHERE s.id = ? AND s.expires_at > ? LIMIT 1",
        token,
        now
    )
    .fetch_optional(pool)
    .await?;

    let user_id = match row {
        Some(r) => r.user_id,
        None => return Ok(None),
    };

    // Update last_used_at
    sqlx::query!(
        "UPDATE sessions SET last_used_at = datetime('now') WHERE id = ?",
        token
    )
    .execute(pool)
    .await?;

    load_user_by_id(pool, user_id).await
}

pub async fn load_user_by_id(pool: &SqlitePool, user_id: i64) -> anyhow::Result<Option<AuthUser>> {
    let user = sqlx::query!(
        "SELECT id, username, email, display_name, is_active FROM users WHERE id = ? AND is_active = 1",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let user = match user {
        Some(u) => u,
        None => return Ok(None),
    };

    // Load roles
    let roles: Vec<String> = sqlx::query_scalar!(
        "SELECT r.name FROM roles r
         JOIN user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = ? AND r.is_active = 1",
        user_id
    )
    .fetch_all(pool)
    .await?;

    let is_superadmin = roles.contains(&"superadmin".to_string());

    // Load permissions from roles
    let role_perms: Vec<String> = sqlx::query_scalar!(
        "SELECT DISTINCT p.name FROM permissions p
         JOIN role_permissions rp ON rp.permission_id = p.id
         JOIN user_roles ur ON ur.role_id = rp.role_id
         WHERE ur.user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await?;

    // Load direct user permissions
    let direct_perms: Vec<(String, bool)> = sqlx::query!(
        "SELECT p.name, up.is_deny FROM permissions p
         JOIN user_permissions up ON up.permission_id = p.id
         WHERE up.user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.name, r.is_deny != 0))
    .collect();

    let mut permissions_set: HashSet<String> = role_perms.iter().cloned().collect();

    // Apply direct permissions (add or deny)
    for (perm, is_deny) in &direct_perms {
        if *is_deny {
            permissions_set.remove(perm);
        } else {
            permissions_set.insert(perm.clone());
        }
    }

    let permissions: Vec<String> = permissions_set.iter().cloned().collect();

    Ok(Some(AuthUser {
        id: user.id,
        username: user.username,
        email: user.email,
        display_name: user.display_name,
        permissions,
        permissions_set,
        roles,
        is_superadmin,
    }))
}

pub async fn load_user_by_api_token(pool: &SqlitePool, token: &str) -> anyhow::Result<Option<AuthUser>> {
    let hash = hash_token(token);
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let row = sqlx::query!(
        "SELECT user_id FROM api_tokens
         WHERE token_hash = ? AND is_active = 1
         AND (expires_at IS NULL OR expires_at > ?)",
        hash,
        now
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            if let Some(uid) = r.user_id {
                sqlx::query!(
                    "UPDATE api_tokens SET last_used_at = datetime('now') WHERE token_hash = ?",
                    hash
                )
                .execute(pool)
                .await?;
                load_user_by_id(pool, uid).await
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn generate_session_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

pub fn generate_api_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    format!("nxd_{}", hex::encode(bytes))
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn create_session(
    pool: &SqlitePool,
    user_id: i64,
    ip: Option<&str>,
    ua: Option<&str>,
    ttl_hours: i64,
) -> anyhow::Result<String> {
    let token = generate_session_token();
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(ttl_hours))
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    sqlx::query!(
        "INSERT INTO sessions (id, user_id, expires_at, ip_address, user_agent)
         VALUES (?, ?, ?, ?, ?)",
        token,
        user_id,
        expires_at,
        ip,
        ua
    )
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn delete_session(pool: &SqlitePool, token: &str) -> anyhow::Result<()> {
    sqlx::query!("DELETE FROM sessions WHERE id = ?", token)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_expired_sessions(pool: &SqlitePool) -> anyhow::Result<u64> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = sqlx::query!("DELETE FROM sessions WHERE expires_at < ?", now)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub fn session_cookie(token: &str, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "nxd_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=28800{}",
        token, secure_flag
    )
}

pub fn clear_session_cookie() -> String {
    "nxd_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT".to_string()
}
