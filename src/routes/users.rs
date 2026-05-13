use crate::{auth::{self as auth_utils, AuthUser}, db, error::AppError, permissions::*, services::audit, state::AppState};
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery { pub search: Option<String> }

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let users = sqlx::query!(
        "SELECT u.id, u.username, u.email, u.display_name, u.is_active, u.last_login_at,
         GROUP_CONCAT(r.display_name, ', ') as roles
         FROM users u
         LEFT JOIN user_roles ur ON ur.user_id = u.id
         LEFT JOIN roles r ON r.id = ur.role_id
         WHERE u.username LIKE ? OR u.email LIKE ? OR u.display_name LIKE ?
         GROUP BY u.id ORDER BY u.username LIMIT 200",
        like, like, like
    )
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = users.into_iter().map(|u| serde_json::json!({
        "id": u.id, "username": u.username, "email": u.email, "display_name": u.display_name,
        "is_active": u.is_active != 0, "last_login_at": u.last_login_at, "roles": u.roles,
    })).collect();

    state.render("users/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        users => items,
        search => search,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_READ)?;

    let target_user = sqlx::query!(
        "SELECT id, username, email, display_name, is_active, is_system, last_login_at, created_at
         FROM users WHERE id=?", id
    )
    .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let user_roles = sqlx::query!(
        "SELECT r.id, r.name, r.display_name, r.rank FROM roles r
         JOIN user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = ? ORDER BY r.rank DESC",
        id
    ).fetch_all(&state.db).await?;

    let max_rank: i64 = auth.roles.iter()
        .filter_map(|r| if r == "superadmin" { Some(9999i64) } else { None })
        .next()
        .unwrap_or(500);
    let all_roles = sqlx::query!(
        "SELECT id, name, display_name, rank, is_system FROM roles
         WHERE is_active=1 AND rank < ?
         ORDER BY rank DESC",
        max_rank
    ).fetch_all(&state.db).await?;

    let role_list: Vec<serde_json::Value> = user_roles.into_iter().map(|r| serde_json::json!({
        "id": r.id, "name": r.name, "display_name": r.display_name, "rank": r.rank,
    })).collect();

    let all_role_list: Vec<serde_json::Value> = all_roles.into_iter().map(|r| serde_json::json!({
        "id": r.id, "name": r.name, "display_name": r.display_name, "rank": r.rank, "is_system": r.is_system != 0,
    })).collect();

    state.render("users/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        target_user => serde_json::json!({
            "id": target_user.id, "username": target_user.username, "email": target_user.email,
            "display_name": target_user.display_name, "is_active": target_user.is_active != 0,
            "is_system": target_user.is_system != 0, "last_login_at": target_user.last_login_at,
            "created_at": target_user.created_at,
        }),
        user_roles => role_list,
        all_roles => all_role_list,
    })
}

#[derive(Deserialize)]
pub struct UserForm {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password: Option<String>,
    pub is_active: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;

    state.render("users/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        target_user => Option::<serde_json::Value>::None,
        title => "Neuer Benutzer",
        action => "/users/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<UserForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;

    if form.username.trim().is_empty() || form.email.trim().is_empty() {
        return Err(AppError::bad_request("Benutzername und E-Mail sind erforderlich"));
    }

    let password = form.password.as_deref().filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad_request("Passwort ist erforderlich für neue Benutzer"))?;

    let hash = auth_utils::hash_password(password)
        .map_err(|e| AppError::internal(e.to_string()))?;

    let username = form.username.trim().to_string();
    let email = form.email.trim().to_string();
    let id = sqlx::query!(
        "INSERT INTO users (username, email, display_name, password_hash, is_active)
         VALUES (?, ?, ?, ?, 1)",
        username, email, form.display_name, hash
    )
    .execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "user", Some(&id.to_string()),
        Some(&format!("Created user: {}", form.username)), None, true).await;

    Ok(Redirect::to(&format!("/users/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;

    let target = sqlx::query!("SELECT id, username, email, display_name, is_active FROM users WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("users/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        target_user => serde_json::json!({
            "id": target.id, "username": target.username, "email": target.email,
            "display_name": target.display_name, "is_active": target.is_active != 0,
        }),
        title => "Benutzer bearbeiten",
        action => format!("/users/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<UserForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;

    let target = sqlx::query!("SELECT id, username, is_system FROM users WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // Check if trying to change own activation status
    let is_active = match form.is_active.as_deref() { Some("on") | Some("1") | Some("true") => 1i64, _ => 0 };

    // Protect last superadmin
    if is_active == 0 {
        let is_superadmin: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM user_roles ur JOIN roles r ON r.id=ur.role_id
             WHERE ur.user_id=? AND r.name='superadmin'", id
        ).fetch_one(&state.db).await? as i64;

        if is_superadmin > 0 {
            let superadmin_count: i64 = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM users u JOIN user_roles ur ON ur.user_id=u.id
                 JOIN roles r ON r.id=ur.role_id WHERE r.name='superadmin' AND u.is_active=1"
            ).fetch_one(&state.db).await? as i64;

            if superadmin_count <= 1 {
                return Err(AppError::bad_request("Der letzte aktive Superadmin kann nicht deaktiviert werden."));
            }
        }
    }

    // Update password if provided
    if let Some(pw) = form.password.as_deref().filter(|s| !s.is_empty()) {
        let hash = auth_utils::hash_password(pw).map_err(|e| AppError::internal(e.to_string()))?;
        sqlx::query!("UPDATE users SET password_hash=? WHERE id=?", hash, id)
            .execute(&state.db).await?;
    }

    let email_trimmed = form.email.trim().to_string();
    sqlx::query!(
        "UPDATE users SET email=?, display_name=?, is_active=?, updated_at=datetime('now') WHERE id=?",
        email_trimmed, form.display_name, is_active, id
    )
    .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "user", Some(&id.to_string()),
        Some(&format!("Updated user: {}", form.username)), None, true).await;

    Ok(Redirect::to(&format!("/users/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_DELETE)?;

    if auth.id == id {
        return Err(AppError::bad_request("Eigenen Account nicht löschbar"));
    }

    // Deactivate instead of hard delete
    sqlx::query!("UPDATE users SET is_active=0 WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "deactivate", "user", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/users"))
}

#[derive(Deserialize)]
pub struct UpdateRolesForm {
    pub role_ids: Option<Vec<i64>>,
}

pub async fn roles_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;
    auth.require_permission(ROLES_WRITE)?;

    let target_user = sqlx::query!(
        "SELECT id, username, display_name FROM users WHERE id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let user_role_ids: Vec<i64> = sqlx::query_scalar!(
        "SELECT role_id FROM user_roles WHERE user_id=?", id
    ).fetch_all(&state.db).await?;

    let all_roles = sqlx::query!(
        "SELECT id, name, display_name, description, rank, is_system FROM roles WHERE is_active=1 ORDER BY rank DESC"
    ).fetch_all(&state.db).await?;

    let all_role_list: Vec<serde_json::Value> = all_roles.into_iter().map(|r| serde_json::json!({
        "id": r.id, "name": r.name, "display_name": r.display_name,
        "description": r.description, "rank": r.rank, "is_system": r.is_system != 0,
    })).collect();

    state.render("users/roles.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        target_user => serde_json::json!({
            "id": target_user.id, "username": target_user.username, "display_name": target_user.display_name,
        }),
        user_role_ids => user_role_ids,
        all_roles => all_role_list,
    })
}

pub async fn update_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(USERS_WRITE)?;
    auth.require_permission(ROLES_WRITE)?;

    // Parse role_ids from form (multivalue field)
    let role_ids: Vec<i64> = form.iter()
        .filter(|(k, _)| *k == "role_ids")
        .filter_map(|(_, v)| v.parse::<i64>().ok())
        .collect();

    // Remove existing roles
    sqlx::query!("DELETE FROM user_roles WHERE user_id=?", id)
        .execute(&state.db).await?;

    // Add new roles
    for role_id in &role_ids {
        sqlx::query!(
            "INSERT OR IGNORE INTO user_roles (user_id, role_id, assigned_by) VALUES (?,?,?)",
            id, role_id, auth.id
        )
        .execute(&state.db).await?;
    }

    audit::log(&state.db, Some(&auth), "update_roles", "user", Some(&id.to_string()),
        Some(&format!("Updated roles: {:?}", role_ids)), None, true).await;

    Ok(Redirect::to(&format!("/users/{}", id)))
}
