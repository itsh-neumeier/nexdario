use crate::{auth::AuthUser, error::AppError, permissions::*, services::audit, state::AppState};
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery { pub location_id: Option<i64> }

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_READ)?;

    let items: Vec<serde_json::Value> = if let Some(lid) = q.location_id {
        sqlx::query!(
            "SELECT w.id, w.name, w.provider, w.connection_type, w.role, w.status,
             w.bandwidth_down, w.bandwidth_up, w.static_ipv4, w.location_id, l.name as location_name
             FROM wan_connections w LEFT JOIN locations l ON l.id=w.location_id
             WHERE w.location_id=? ORDER BY w.role, w.name", lid
        ).fetch_all(&state.db).await?
        .into_iter().map(|w| serde_json::json!({
            "id": w.id, "name": w.name, "provider": w.provider, "connection_type": w.connection_type,
            "role": w.role, "status": w.status,
            "bandwidth_down": w.bandwidth_down, "bandwidth_up": w.bandwidth_up,
            "static_ipv4": w.static_ipv4,
            "location_id": w.location_id, "location_name": w.location_name,
        })).collect()
    } else {
        sqlx::query!(
            "SELECT w.id, w.name, w.provider, w.connection_type, w.role, w.status,
             w.bandwidth_down, w.bandwidth_up, w.static_ipv4, w.location_id, l.name as location_name
             FROM wan_connections w LEFT JOIN locations l ON l.id=w.location_id
             ORDER BY l.name, w.role, w.name LIMIT 200"
        ).fetch_all(&state.db).await?
        .into_iter().map(|w| serde_json::json!({
            "id": w.id, "name": w.name, "provider": w.provider, "connection_type": w.connection_type,
            "role": w.role, "status": w.status,
            "bandwidth_down": w.bandwidth_down, "bandwidth_up": w.bandwidth_up,
            "static_ipv4": w.static_ipv4,
            "location_id": w.location_id, "location_name": w.location_name,
        })).collect()
    };

    state.render("wan/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        wan_connections => items,
        location_id_filter => q.location_id,
    })
}

#[derive(Deserialize)]
pub struct WanForm {
    pub location_id: Option<i64>,
    pub name: String,
    pub provider: Option<String>,
    pub connection_type: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub circuit_id: Option<String>,
    pub customer_number: Option<String>,
    pub contract_number: Option<String>,
    pub bandwidth_down: Option<i64>,
    pub bandwidth_up: Option<i64>,
    pub static_ipv4: Option<String>,
    pub gateway: Option<String>,
    pub dns_primary: Option<String>,
    pub vlan_id: Option<i64>,
    pub pppoe_username: Option<String>,
    pub notes: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_WRITE)?;

    let locations = sqlx::query!("SELECT id, name, site_code FROM locations WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("wan/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        wan => Option::<serde_json::Value>::None,
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name, "site_code": l.site_code})).collect::<Vec<_>>(),
        preselect_location_id => q.location_id,
        title => "Neue WAN-Verbindung",
        action => "/wan/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<WanForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_WRITE)?;

    let location_id = form.location_id.ok_or_else(|| AppError::bad_request("Standort ist erforderlich"))?;
    let role = form.role.as_deref().unwrap_or("PRIMARY");
    let status = form.status.as_deref().unwrap_or("active");

    let id = sqlx::query!(
        "INSERT INTO wan_connections (location_id, name, provider, connection_type, role, status,
         circuit_id, customer_number, contract_number, bandwidth_down, bandwidth_up,
         static_ipv4, gateway, dns_primary, vlan_id, pppoe_username, notes)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        location_id, form.name, form.provider, form.connection_type, role, status,
        form.circuit_id, form.customer_number, form.contract_number,
        form.bandwidth_down, form.bandwidth_up, form.static_ipv4, form.gateway,
        form.dns_primary, form.vlan_id, form.pppoe_username, form.notes
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "wan_connection", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/locations/{}", location_id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_WRITE)?;

    let wan = sqlx::query!("SELECT * FROM wan_connections WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let locations = sqlx::query!("SELECT id, name, site_code FROM locations WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("wan/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        wan => serde_json::json!({
            "id": wan.id, "location_id": wan.location_id, "name": wan.name,
            "provider": wan.provider, "connection_type": wan.connection_type,
            "role": wan.role, "status": wan.status, "circuit_id": wan.circuit_id,
            "bandwidth_down": wan.bandwidth_down, "bandwidth_up": wan.bandwidth_up,
            "static_ipv4": wan.static_ipv4, "gateway": wan.gateway,
            "dns_primary": wan.dns_primary, "vlan_id": wan.vlan_id,
            "pppoe_username": wan.pppoe_username, "notes": wan.notes,
        }),
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name, "site_code": l.site_code})).collect::<Vec<_>>(),
        title => "WAN-Verbindung bearbeiten",
        action => format!("/wan/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<WanForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_WRITE)?;

    let existing = sqlx::query!("SELECT location_id FROM wan_connections WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let location_id = form.location_id.ok_or_else(|| AppError::bad_request("Standort ist erforderlich"))?;
    let role = form.role.as_deref().unwrap_or("PRIMARY");
    let status = form.status.as_deref().unwrap_or("active");

    sqlx::query!(
        "UPDATE wan_connections SET location_id=?, name=?, provider=?, connection_type=?, role=?,
         status=?, circuit_id=?, customer_number=?, contract_number=?, bandwidth_down=?, bandwidth_up=?,
         static_ipv4=?, gateway=?, dns_primary=?, vlan_id=?, pppoe_username=?, notes=?,
         updated_at=datetime('now') WHERE id=?",
        location_id, form.name, form.provider, form.connection_type, role, status,
        form.circuit_id, form.customer_number, form.contract_number,
        form.bandwidth_down, form.bandwidth_up, form.static_ipv4, form.gateway,
        form.dns_primary, form.vlan_id, form.pppoe_username, form.notes, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "wan_connection", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/locations/{}", existing.location_id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(WAN_DELETE)?;

    let existing = sqlx::query!("SELECT location_id FROM wan_connections WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    sqlx::query!("DELETE FROM wan_connections WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "wan_connection", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/locations/{}", existing.location_id)))
}
