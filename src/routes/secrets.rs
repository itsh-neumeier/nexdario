use crate::{
    auth::{self as auth_utils, AuthUser},
    error::AppError,
    permissions::*,
    services::{audit, encryption::EncryptionService},
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery {
    pub customer_id: Option<i64>,
    pub location_id: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_PRESENCE_READ)?;

    let secrets = sqlx::query!(
        "SELECT s.id, s.name, s.secret_type, s.username, s.url, s.description,
         s.customer_id, s.location_id, s.asset_id, s.is_active,
         c.name as customer_name, l.name as location_name
         FROM secrets s
         LEFT JOIN customers c ON c.id=s.customer_id
         LEFT JOIN locations l ON l.id=s.location_id
         WHERE s.is_active=1
         ORDER BY s.name LIMIT 200"
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = secrets.into_iter().map(|s| serde_json::json!({
        "id": s.id, "name": s.name, "secret_type": s.secret_type, "username": s.username,
        "url": s.url, "description": s.description, "is_active": s.is_active != 0,
        "customer_id": s.customer_id, "location_id": s.location_id, "asset_id": s.asset_id,
        "customer_name": s.customer_name, "location_name": s.location_name,
        // Never include password in list
    })).collect();

    state.render("secrets/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secrets => items,
        can_reveal => auth.has_permission(SECRETS_REVEAL),
        can_write => auth.has_permission(SECRETS_WRITE),
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_PRESENCE_READ)?;

    let secret = sqlx::query!(
        "SELECT s.*, c.name as customer_name, l.name as location_name, a.hostname as asset_hostname
         FROM secrets s
         LEFT JOIN customers c ON c.id=s.customer_id
         LEFT JOIN locations l ON l.id=s.location_id
         LEFT JOIN assets a ON a.id=s.asset_id
         WHERE s.id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // Load access tokens (presence only)
    let tokens = sqlx::query!(
        "SELECT id, purpose, access_type, is_active, expires_at, usage_count, created_at
         FROM secret_access_tokens WHERE secret_id=? ORDER BY created_at DESC LIMIT 20", id
    ).fetch_all(&state.db).await?;

    state.render("secrets/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secret => serde_json::json!({
            "id": secret.id, "name": secret.name, "secret_type": secret.secret_type,
            "username": secret.username, "url": secret.url, "description": secret.description,
            "is_active": secret.is_active != 0,
            "customer_name": secret.customer_name, "location_name": secret.location_name,
            "asset_hostname": secret.asset_hostname,
            // NO password_encrypted in template context
        }),
        tokens => tokens.into_iter().map(|t| serde_json::json!({
            "id": t.id, "purpose": t.purpose, "access_type": t.access_type,
            "is_active": t.is_active != 0, "expires_at": t.expires_at,
            "usage_count": t.usage_count, "created_at": t.created_at,
        })).collect::<Vec<_>>(),
        can_reveal => auth.has_permission(SECRETS_REVEAL),
        can_create_token => auth.has_permission(SECRET_ACCESS_CREATE),
    })
}

#[derive(Deserialize)]
pub struct SecretForm {
    pub name: String,
    pub secret_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub customer_id: Option<i64>,
    pub location_id: Option<i64>,
    pub asset_id: Option<i64>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let locations = sqlx::query!("SELECT id, name FROM locations WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("secrets/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secret => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name})).collect::<Vec<_>>(),
        title => "Neues Secret",
        action => "/secrets/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<SecretForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_WRITE)?;

    let enc = EncryptionService::new(&state.config.data_encryption_key);
    let encrypted_pw = if let Some(pw) = form.password.as_deref().filter(|s| !s.is_empty()) {
        Some(enc.encrypt(pw).map_err(|e| AppError::internal(e.to_string()))?)
    } else {
        None
    };

    let id = sqlx::query!(
        "INSERT INTO secrets (name, secret_type, username, password_encrypted, url, description,
         customer_id, location_id, asset_id, created_by)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
        form.name, form.secret_type, form.username, encrypted_pw,
        form.url, form.description, form.customer_id, form.location_id, form.asset_id, auth.id
    ).execute(&state.db).await?.last_insert_rowid();

    // Audit: note that password is NOT logged
    audit::log(&state.db, Some(&auth), "create", "secret", Some(&id.to_string()),
        Some(&format!("Created secret: {}", form.name)), None, true).await;

    Ok(Redirect::to(&format!("/secrets/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_WRITE)?;

    let secret = sqlx::query!("SELECT * FROM secrets WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let locations = sqlx::query!("SELECT id, name FROM locations WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("secrets/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secret => serde_json::json!({
            "id": secret.id, "name": secret.name, "secret_type": secret.secret_type,
            "username": secret.username, "url": secret.url, "description": secret.description,
            "customer_id": secret.customer_id, "location_id": secret.location_id,
            // NO password in form
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name})).collect::<Vec<_>>(),
        title => "Secret bearbeiten",
        action => format!("/secrets/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<SecretForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_WRITE)?;

    let existing = sqlx::query!("SELECT id, password_encrypted FROM secrets WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let enc = EncryptionService::new(&state.config.data_encryption_key);

    // Only update password if a new one is provided
    let encrypted_pw = if let Some(pw) = form.password.as_deref().filter(|s| !s.is_empty()) {
        Some(enc.encrypt(pw).map_err(|e| AppError::internal(e.to_string()))?)
    } else {
        existing.password_encrypted
    };

    sqlx::query!(
        "UPDATE secrets SET name=?, secret_type=?, username=?, password_encrypted=?,
         url=?, description=?, customer_id=?, location_id=?, asset_id=?,
         updated_by=?, updated_at=datetime('now') WHERE id=?",
        form.name, form.secret_type, form.username, encrypted_pw,
        form.url, form.description, form.customer_id, form.location_id, form.asset_id,
        auth.id, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "secret", Some(&id.to_string()),
        Some(&format!("Updated secret: {}", form.name)), None, true).await;

    Ok(Redirect::to(&format!("/secrets/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_WRITE)?;

    sqlx::query!("UPDATE secrets SET is_active=0, updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "secret", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/secrets"))
}

pub async fn reveal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRETS_REVEAL)?;

    let secret = sqlx::query!("SELECT name, password_encrypted FROM secrets WHERE id=? AND is_active=1", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let password = if let Some(enc_pw) = &secret.password_encrypted {
        let enc = EncryptionService::new(&state.config.data_encryption_key);
        match enc.decrypt(enc_pw) {
            Ok(pw) => pw,
            Err(_) => return Err(AppError::internal("Decryption failed")),
        }
    } else {
        String::new()
    };

    audit::log(&state.db, Some(&auth), "reveal", "secret", Some(&id.to_string()),
        Some(&format!("Revealed secret: {}", secret.name)), None, true).await;

    state.render("secrets/reveal.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secret_name => secret.name,
        password => password,
        secret_id => id,
    })
}

pub async fn access_token_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRET_ACCESS_CREATE)?;

    let secret = sqlx::query!("SELECT id, name FROM secrets WHERE id=? AND is_active=1", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("secrets/access_token_form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        secret => serde_json::json!({ "id": secret.id, "name": secret.name }),
    })
}

#[derive(Deserialize)]
pub struct AccessTokenForm {
    pub purpose: String,
    pub access_type: String,
    pub expires_hours: Option<i64>,
}

pub async fn create_access_token(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<AccessTokenForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SECRET_ACCESS_CREATE)?;

    if form.purpose.trim().is_empty() {
        return Err(AppError::bad_request("Zweck ist erforderlich"));
    }

    let token = auth_utils::generate_api_token();
    let token_hash = auth_utils::hash_token(&token);

    let expires_at = form.expires_hours.map(|h| {
        chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(h))
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    });

    let max_uses: Option<i64> = match form.access_type.as_str() {
        "one_time" => Some(1),
        _ => None,
    };

    sqlx::query!(
        "INSERT INTO secret_access_tokens (token_hash, secret_id, purpose, access_type, expires_at, max_uses, created_by)
         VALUES (?,?,?,?,?,?,?)",
        token_hash, id, form.purpose, form.access_type, expires_at, max_uses, auth.id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "create_access_token", "secret", Some(&id.to_string()),
        Some(&format!("Created access token for purpose: {}", form.purpose)), None, true).await;

    state.render("secrets/token_created.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        token => token,
        secret_id => id,
        purpose => form.purpose,
        access_url => format!("{}/secret-access/{}", state.config.app_base_url, token),
    })
}

pub async fn use_access_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let token_hash = auth_utils::hash_token(&token);
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let token_row = sqlx::query!(
        "SELECT t.id, t.secret_id, t.purpose, t.access_type, t.is_active,
         t.expires_at, t.usage_count, t.max_uses
         FROM secret_access_tokens t
         WHERE t.token_hash=? AND t.is_active=1
         AND (t.expires_at IS NULL OR t.expires_at > ?)",
        token_hash, now
    ).fetch_optional(&state.db).await?;

    let token_row = match token_row {
        Some(t) => t,
        None => {
            return state.render("secrets/access_denied.html", minijinja::context! {
                app_name => &state.config.app_name,
                reason => "Token ungültig, abgelaufen oder widerrufen",
            });
        }
    };

    // Check usage limit
    if let Some(max) = token_row.max_uses {
        if token_row.usage_count >= max {
            return state.render("secrets/access_denied.html", minijinja::context! {
                app_name => &state.config.app_name,
                reason => "Token bereits verbraucht",
            });
        }
    }

    // Get the secret — decrypt password
    let secret = sqlx::query!("SELECT name, password_encrypted, username, url FROM secrets WHERE id=? AND is_active=1", token_row.secret_id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let password = if let Some(enc_pw) = &secret.password_encrypted {
        let enc = EncryptionService::new(&state.config.data_encryption_key);
        enc.decrypt(enc_pw).unwrap_or_default()
    } else {
        String::new()
    };

    // Update usage count and deactivate if one_time
    let new_count = token_row.usage_count + 1;
    let still_active = match token_row.access_type.as_str() {
        "one_time" => 0i64,
        _ => 1i64,
    };

    sqlx::query!(
        "UPDATE secret_access_tokens SET usage_count=?, is_active=?, last_used_at=datetime('now')
         WHERE id=?",
        new_count, still_active, token_row.id
    ).execute(&state.db).await?;

    // Audit without the password
    sqlx::query!(
        "INSERT INTO audit_log (action, resource_type, resource_id, details, success)
         VALUES ('use_access_token', 'secret', ?, ?, 1)",
        token_row.secret_id.to_string(),
        format!("Token used for: {}", token_row.purpose)
    ).execute(&state.db).await.ok();

    state.render("secrets/access_view.html", minijinja::context! {
        app_name => &state.config.app_name,
        secret_name => secret.name,
        username => secret.username,
        password => password,
        url => secret.url,
        purpose => token_row.purpose,
        is_one_time => token_row.access_type == "one_time",
    })
}
