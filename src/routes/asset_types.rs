use crate::{auth::AuthUser, error::AppError, permissions::*, state::AppState};
use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let types = sqlx::query!(
        "SELECT id, code, label, is_active, sort_order FROM asset_device_types ORDER BY sort_order, code"
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = types.into_iter().map(|t| serde_json::json!({
        "id": t.id, "code": t.code, "label": t.label,
        "is_active": t.is_active != 0, "sort_order": t.sort_order,
    })).collect();

    state.render("asset_types/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        types => items,
    })
}

#[derive(Deserialize)]
pub struct TypeForm {
    pub code: String,
    pub label: String,
    pub sort_order: Option<i64>,
    pub is_active: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<TypeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let code = form.code.trim().to_uppercase();
    if code.is_empty() { return Err(AppError::bad_request("Code erforderlich")); }

    let sort_order = form.sort_order.unwrap_or(100);
    sqlx::query!(
        "INSERT INTO asset_device_types (code, label, sort_order) VALUES (?,?,?)",
        code, form.label, sort_order
    ).execute(&state.db).await?;

    Ok(Redirect::to("/asset-types"))
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<TypeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let is_active = match form.is_active.as_deref() { Some("on") | Some("1") => 1i64, _ => 0 };
    let sort_order = form.sort_order.unwrap_or(100);

    sqlx::query!(
        "UPDATE asset_device_types SET label=?, sort_order=?, is_active=? WHERE id=?",
        form.label, sort_order, is_active, id
    ).execute(&state.db).await?;

    Ok(Redirect::to("/asset-types"))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    sqlx::query!("DELETE FROM asset_device_types WHERE id=?", id)
        .execute(&state.db).await?;

    Ok(Redirect::to("/asset-types"))
}
