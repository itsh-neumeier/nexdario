use crate::{auth::AuthUser, error::AppError, permissions::*, services::audit, state::AppState};
use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;
use std::collections::HashMap;

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_READ)?;

    let roles = sqlx::query!(
        "SELECT r.id, r.name, r.display_name, r.description, r.rank, r.is_system, r.is_active,
         COUNT(DISTINCT ur.user_id) as user_count,
         COUNT(DISTINCT rp.permission_id) as perm_count
         FROM roles r
         LEFT JOIN user_roles ur ON ur.role_id = r.id
         LEFT JOIN role_permissions rp ON rp.role_id = r.id
         GROUP BY r.id ORDER BY r.rank DESC"
    )
    .fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = roles.into_iter().map(|r| serde_json::json!({
        "id": r.id, "name": r.name, "display_name": r.display_name, "description": r.description,
        "rank": r.rank, "is_system": r.is_system != 0, "is_active": r.is_active != 0,
        "user_count": r.user_count, "perm_count": r.perm_count,
    })).collect();

    state.render("roles/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        roles => items,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_READ)?;

    let role = sqlx::query!(
        "SELECT id, name, display_name, description, rank, is_system, is_active,
         allow_api_access, mobile_access, default_landing
         FROM roles WHERE id=?", id
    )
    .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let role_perms: Vec<String> = sqlx::query_scalar!(
        "SELECT p.name FROM permissions p JOIN role_permissions rp ON rp.permission_id=p.id
         WHERE rp.role_id=?", id
    ).fetch_all(&state.db).await?;

    // All permissions grouped by module
    let all_perms = sqlx::query!(
        "SELECT id, name, display_name, module FROM permissions ORDER BY module, name"
    ).fetch_all(&state.db).await?;

    let mut perms_by_module: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for p in all_perms {
        let has = role_perms.contains(&p.name);
        perms_by_module.entry(p.module.clone()).or_default().push(serde_json::json!({
            "id": p.id, "name": p.name, "display_name": p.display_name,
            "module": p.module, "has": has,
        }));
    }

    // Users with this role
    let users = sqlx::query!(
        "SELECT u.id, u.username, u.display_name FROM users u
         JOIN user_roles ur ON ur.user_id=u.id
         WHERE ur.role_id=? ORDER BY u.username LIMIT 50", id
    ).fetch_all(&state.db).await?;

    state.render("roles/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        role => serde_json::json!({
            "id": role.id, "name": role.name, "display_name": role.display_name,
            "description": role.description, "rank": role.rank,
            "is_system": role.is_system != 0, "is_active": role.is_active != 0,
            "allow_api_access": role.allow_api_access != 0, "mobile_access": role.mobile_access != 0,
        }),
        perms_by_module => perms_by_module,
        role_perm_names => role_perms,
        users => users.into_iter().map(|u| serde_json::json!({"id": u.id, "username": u.username, "display_name": u.display_name})).collect::<Vec<_>>(),
    })
}

#[derive(Deserialize)]
pub struct RoleForm {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub rank: i64,
    pub is_active: Option<String>,
    pub allow_api_access: Option<String>,
    pub mobile_access: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_WRITE)?;

    state.render("roles/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        role => Option::<serde_json::Value>::None,
        title => "Neue Rolle",
        action => "/roles/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<RoleForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_WRITE)?;

    // Validate rank — custom roles cannot exceed calling user's max rank
    let user_max_rank: i64 = sqlx::query_scalar!(
        "SELECT MAX(r.rank) FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=?",
        auth.id
    ).fetch_one(&state.db).await?.unwrap_or(0);

    if !auth.is_superadmin && form.rank >= user_max_rank {
        return Err(AppError::forbidden("Rang darf nicht >= eigener Rolle sein"));
    }

    let name = form.name.to_lowercase().replace(' ', "_");
    let is_active = match form.is_active.as_deref() { Some("on") | Some("1") => 1i64, _ => 1 };
    let allow_api = match form.allow_api_access.as_deref() { Some("on") | Some("1") => 1i64, _ => 0 };
    let mobile = match form.mobile_access.as_deref() { Some("on") | Some("1") => 1i64, _ => 0 };

    let id = sqlx::query!(
        "INSERT INTO roles (name, display_name, description, rank, is_system, is_active, allow_api_access, mobile_access)
         VALUES (?,?,?,?,0,?,?,?)",
        name, form.display_name, form.description, form.rank, is_active, allow_api, mobile
    )
    .execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "role", Some(&id.to_string()),
        Some(&format!("Created role: {}", name)), None, true).await;

    Ok(Redirect::to(&format!("/roles/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_WRITE)?;

    let role = sqlx::query!("SELECT * FROM roles WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // All permissions for assignment
    let all_perms = sqlx::query!(
        "SELECT id, name, display_name, module FROM permissions ORDER BY module, name"
    ).fetch_all(&state.db).await?;

    let role_perm_ids: Vec<i64> = sqlx::query_scalar!(
        "SELECT permission_id FROM role_permissions WHERE role_id=?", id
    ).fetch_all(&state.db).await?;

    let mut perms_by_module: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for p in all_perms {
        let has = role_perm_ids.contains(&p.id);
        perms_by_module.entry(p.module.clone()).or_default().push(serde_json::json!({
            "id": p.id, "name": p.name, "display_name": p.display_name, "has": has,
        }));
    }

    state.render("roles/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        role => serde_json::json!({
            "id": role.id, "name": role.name, "display_name": role.display_name,
            "description": role.description, "rank": role.rank,
            "is_system": role.is_system != 0, "is_active": role.is_active != 0,
            "allow_api_access": role.allow_api_access != 0, "mobile_access": role.mobile_access != 0,
        }),
        perms_by_module => perms_by_module,
        role_perm_ids => role_perm_ids,
        title => format!("Rolle bearbeiten: {}", role.display_name),
        action => format!("/roles/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_WRITE)?;

    let role = sqlx::query!("SELECT id, name, is_system, rank FROM roles WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // System roles: only superadmin can modify core fields
    if role.is_system != 0 && !auth.is_superadmin {
        return Err(AppError::forbidden("Systemrollen können nur von Superadmin geändert werden"));
    }

    let display_name = form.get("display_name").cloned().unwrap_or_default();
    let description = form.get("description").cloned();
    let is_active = match form.get("is_active").map(String::as_str) {
        Some("on") | Some("1") | Some("true") => 1i64,
        _ => if role.is_system != 0 { 1 } else { 0 }
    };
    let allow_api = match form.get("allow_api_access").map(String::as_str) { Some("on") | Some("1") => 1i64, _ => 0 };
    let mobile = match form.get("mobile_access").map(String::as_str) { Some("on") | Some("1") => 1i64, _ => 0 };

    // Rank only for custom roles
    let rank = if role.is_system == 0 {
        form.get("rank").and_then(|r| r.parse::<i64>().ok()).unwrap_or(role.rank)
    } else {
        role.rank
    };

    sqlx::query!(
        "UPDATE roles SET display_name=?, description=?, rank=?, is_active=?,
         allow_api_access=?, mobile_access=?, updated_at=datetime('now') WHERE id=?",
        display_name, description, rank, is_active, allow_api, mobile, id
    )
    .execute(&state.db).await?;

    // Update permissions
    let perm_ids: Vec<i64> = form.iter()
        .filter(|(k, _)| *k == "perm_ids")
        .filter_map(|(_, v)| v.parse::<i64>().ok())
        .collect();

    sqlx::query!("DELETE FROM role_permissions WHERE role_id=?", id)
        .execute(&state.db).await?;

    for perm_id in &perm_ids {
        sqlx::query!(
            "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?,?)",
            id, perm_id
        ).execute(&state.db).await?;
    }

    audit::log(&state.db, Some(&auth), "update", "role", Some(&id.to_string()),
        Some(&format!("Updated role: {} with {} permissions", role.name, perm_ids.len())), None, true).await;

    Ok(Redirect::to(&format!("/roles/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ROLES_DELETE)?;

    let role = sqlx::query!("SELECT name, is_system FROM roles WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    if role.is_system != 0 {
        return Err(AppError::bad_request("Systemrollen können nicht gelöscht werden"));
    }

    sqlx::query!("UPDATE roles SET is_active=0, updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "role", Some(&id.to_string()),
        Some(&format!("Deactivated role: {}", role.name)), None, true).await;

    Ok(Redirect::to("/roles"))
}
