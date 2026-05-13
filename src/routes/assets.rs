use crate::{auth::AuthUser, error::AppError, permissions::*, services::{audit, naming}, state::AppState};
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub customer_id: Option<i64>,
    pub location_id: Option<i64>,
    pub device_type: Option<String>,
    pub status: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);
    let device_type_filter = q.device_type.as_deref().unwrap_or("").to_string();
    let status_filter = q.status.as_deref().unwrap_or("").to_string();

    let assets = sqlx::query!(
        "SELECT a.id, a.hostname, a.device_type, a.role, a.manufacturer, a.model,
         a.management_ip, a.status, a.customer_id, a.location_id,
         c.name as customer_name, l.name as location_name, l.site_code
         FROM assets a
         LEFT JOIN customers c ON c.id = a.customer_id
         LEFT JOIN locations l ON l.id = a.location_id
         WHERE (a.hostname LIKE ? OR a.manufacturer LIKE ? OR a.model LIKE ? OR a.management_ip LIKE ?)
           AND (? = '' OR a.device_type = ?)
           AND (? = '' OR a.status = ?)
         ORDER BY a.hostname LIMIT 500",
        like, like, like, like,
        device_type_filter, device_type_filter,
        status_filter, status_filter
    )
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = assets.into_iter().map(|a| serde_json::json!({
        "id": a.id, "hostname": a.hostname, "device_type": a.device_type, "role": a.role,
        "manufacturer": a.manufacturer, "model": a.model, "management_ip": a.management_ip,
        "status": a.status, "customer_id": a.customer_id, "location_id": a.location_id,
        "customer_name": a.customer_name, "location_name": a.location_name, "site_code": a.site_code,
    })).collect();

    // Load device types from DB for filter dropdown
    let db_types = sqlx::query!(
        "SELECT code, label FROM asset_device_types WHERE is_active=1 ORDER BY sort_order, code"
    ).fetch_all(&state.db).await?;
    let device_types: Vec<serde_json::Value> = db_types.into_iter()
        .map(|t| serde_json::json!({"code": t.code, "label": t.label})).collect();

    state.render("assets/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        assets => items,
        search => search,
        device_type => device_type_filter,
        status => status_filter,
        device_types => device_types,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_READ)?;

    let asset = sqlx::query!(
        "SELECT a.*, c.name as customer_name, l.name as location_name, l.site_code
         FROM assets a
         LEFT JOIN customers c ON c.id = a.customer_id
         LEFT JOIN locations l ON l.id = a.location_id
         WHERE a.id = ?", id
    )
    .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("assets/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        asset => serde_json::json!({
            "id": asset.id, "hostname": asset.hostname, "device_type": asset.device_type,
            "role": asset.role, "manufacturer": asset.manufacturer, "model": asset.model,
            "serial_number": asset.serial_number, "mac_address": asset.mac_address,
            "management_ip": asset.management_ip, "firmware_version": asset.firmware_version,
            "status": asset.status, "description": asset.description,
            "warranty_until": asset.warranty_until, "maintenance_until": asset.maintenance_until,
            "last_check": asset.last_check,
            "customer_id": asset.customer_id, "customer_name": asset.customer_name,
            "location_id": asset.location_id, "location_name": asset.location_name,
            "site_code": asset.site_code,
            "unifi_device_id": asset.unifi_device_id, "unifi_site": asset.unifi_site,
            "unifi_adoption_status": asset.unifi_adoption_status,
            "unifi_online": asset.unifi_online != Some(0),
            "unifi_clients_current": asset.unifi_clients_current,
            "unifi_last_contact": asset.unifi_last_contact,
            "unifi_controller_url": asset.unifi_controller_url,
        }),
        device_types => naming::DEVICE_TYPES,
        device_roles => naming::DEVICE_ROLES,
    })
}

#[derive(Deserialize)]
pub struct AssetForm {
    pub hostname: String,
    pub customer_id: Option<i64>,
    pub location_id: Option<i64>,
    pub device_type: String,
    pub role: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub mac_address: Option<String>,
    pub management_ip: Option<String>,
    pub firmware_version: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub warranty_until: Option<String>,
    pub maintenance_until: Option<String>,
    pub unifi_device_id: Option<String>,
    pub unifi_site: Option<String>,
    pub unifi_controller_url: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let locations = sqlx::query!("SELECT id, name, site_code, customer_id FROM locations WHERE status='active' ORDER BY name LIMIT 500")
        .fetch_all(&state.db).await?;

    let db_types = sqlx::query!(
        "SELECT code, label FROM asset_device_types WHERE is_active=1 ORDER BY sort_order, code"
    ).fetch_all(&state.db).await?;
    let device_types: Vec<serde_json::Value> = db_types.into_iter()
        .map(|t| serde_json::json!({"code": t.code, "label": t.label})).collect();

    state.render("assets/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        asset => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name, "site_code": l.site_code, "customer_id": l.customer_id})).collect::<Vec<_>>(),
        device_types => device_types,
        device_roles => naming::DEVICE_ROLES,
        title => "Neues Asset",
        action => "/assets/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<AssetForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let hostname = form.hostname.trim().to_uppercase();

    if hostname.is_empty() {
        return Err(AppError::bad_request("Hostname ist erforderlich"));
    }

    if !naming::validate_hostname(&hostname) {
        return Err(AppError::bad_request("Ungültiges Hostname-Format. Nur A-Z, 0-9 und Bindestrich erlaubt."));
    }

    let customer_id = form.customer_id.ok_or_else(|| AppError::bad_request("Kunde ist erforderlich"))?;
    let status = form.status.as_deref().unwrap_or("active");

    let id = sqlx::query!(
        "INSERT INTO assets (hostname, customer_id, location_id, device_type, role, manufacturer, model,
         serial_number, mac_address, management_ip, firmware_version, status, description,
         warranty_until, maintenance_until, unifi_device_id, unifi_site, unifi_controller_url)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        hostname, customer_id, form.location_id, form.device_type, form.role,
        form.manufacturer, form.model, form.serial_number, form.mac_address,
        form.management_ip, form.firmware_version, status, form.description,
        form.warranty_until, form.maintenance_until,
        form.unifi_device_id, form.unifi_site, form.unifi_controller_url
    )
    .execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "asset", Some(&id.to_string()),
        Some(&format!("Created asset: {}", hostname)), None, true).await;

    Ok(Redirect::to(&format!("/assets/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let asset = sqlx::query!("SELECT * FROM assets WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let locations = sqlx::query!("SELECT id, name, site_code, customer_id FROM locations WHERE status='active' ORDER BY name LIMIT 500")
        .fetch_all(&state.db).await?;

    state.render("assets/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        asset => serde_json::json!({
            "id": asset.id, "hostname": asset.hostname, "customer_id": asset.customer_id,
            "location_id": asset.location_id, "device_type": asset.device_type, "role": asset.role,
            "manufacturer": asset.manufacturer, "model": asset.model, "serial_number": asset.serial_number,
            "mac_address": asset.mac_address, "management_ip": asset.management_ip,
            "firmware_version": asset.firmware_version, "status": asset.status,
            "description": asset.description, "warranty_until": asset.warranty_until,
            "maintenance_until": asset.maintenance_until,
            "unifi_device_id": asset.unifi_device_id, "unifi_site": asset.unifi_site,
            "unifi_controller_url": asset.unifi_controller_url,
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        locations => locations.into_iter().map(|l| serde_json::json!({"id": l.id, "name": l.name, "site_code": l.site_code, "customer_id": l.customer_id})).collect::<Vec<_>>(),
        device_types => {
            let db_types = sqlx::query!(
                "SELECT code, label FROM asset_device_types WHERE is_active=1 ORDER BY sort_order, code"
            ).fetch_all(&state.db).await?;
            db_types.into_iter().map(|t| serde_json::json!({"code": t.code, "label": t.label})).collect::<Vec<_>>()
        },
        device_roles => naming::DEVICE_ROLES,
        title => "Asset bearbeiten",
        action => format!("/assets/{}", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<AssetForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_WRITE)?;

    let _existing = sqlx::query!("SELECT id FROM assets WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customer_id = form.customer_id.ok_or_else(|| AppError::bad_request("Kunde ist erforderlich"))?;
    let hostname = form.hostname.trim().to_uppercase();
    let status = form.status.as_deref().unwrap_or("active");

    sqlx::query!(
        "UPDATE assets SET hostname=?, customer_id=?, location_id=?, device_type=?, role=?,
         manufacturer=?, model=?, serial_number=?, mac_address=?, management_ip=?,
         firmware_version=?, status=?, description=?, warranty_until=?, maintenance_until=?,
         unifi_device_id=?, unifi_site=?, unifi_controller_url=?,
         updated_at=datetime('now') WHERE id=?",
        hostname, customer_id, form.location_id, form.device_type, form.role,
        form.manufacturer, form.model, form.serial_number, form.mac_address,
        form.management_ip, form.firmware_version, status, form.description,
        form.warranty_until, form.maintenance_until,
        form.unifi_device_id, form.unifi_site, form.unifi_controller_url, id
    )
    .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "asset", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/assets/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(ASSETS_DELETE)?;

    sqlx::query!("UPDATE assets SET status='decommissioned', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "asset", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/assets"))
}
