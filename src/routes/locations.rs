use crate::{auth::AuthUser, db, error::AppError, permissions::*, services::{audit, naming}, state::AppState};
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub customer_id: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let locations = sqlx::query!(
        "SELECT l.id, l.site_code, l.name, l.city, l.zip, l.status, l.customer_id,
         c.name as customer_name
         FROM locations l LEFT JOIN customers c ON c.id = l.customer_id
         WHERE (l.name LIKE ? OR l.site_code LIKE ? OR l.city LIKE ?)
         ORDER BY l.name LIMIT 200",
        like, like, like
    )
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = locations.into_iter().map(|l| serde_json::json!({
        "id": l.id, "site_code": l.site_code, "name": l.name,
        "city": l.city, "zip": l.zip, "status": l.status,
        "customer_id": l.customer_id, "customer_name": l.customer_name,
    })).collect();

    state.render("locations/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        locations => items,
        search => search,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_READ)?;

    let loc = sqlx::query!(
        "SELECT l.*, c.name as customer_name FROM locations l
         LEFT JOIN customers c ON c.id = l.customer_id WHERE l.id = ?", id
    )
    .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    // Load assets
    let assets = sqlx::query!(
        "SELECT id, hostname, device_type, role, manufacturer, model, status, management_ip
         FROM assets WHERE location_id = ? ORDER BY hostname LIMIT 100",
        id
    ).fetch_all(&state.db).await?;

    // Load WAN connections
    let wan_connections = sqlx::query!(
        "SELECT id, name, provider, connection_type, role, status, bandwidth_down, bandwidth_up
         FROM wan_connections WHERE location_id = ? ORDER BY role, name",
        id
    ).fetch_all(&state.db).await?;

    // Load contacts for customer
    let contacts = sqlx::query!(
        "SELECT id, display_name, position, phone, email, is_primary
         FROM contacts WHERE customer_id = (SELECT customer_id FROM locations WHERE id = ?)
         AND status = 'active' ORDER BY is_primary DESC LIMIT 20",
        id
    ).fetch_all(&state.db).await?;

    // Load docs
    let docs = sqlx::query!(
        "SELECT id, title, category, visibility, is_pinned, updated_at
         FROM location_docs WHERE location_id = ? ORDER BY is_pinned DESC, sort_order, title",
        id
    ).fetch_all(&state.db).await?;

    // Load recent service jobs
    let jobs = sqlx::query!(
        "SELECT id, job_number, title, status, priority, scheduled_start
         FROM service_jobs WHERE location_id = ? ORDER BY created_at DESC LIMIT 5",
        id
    ).fetch_all(&state.db).await?;

    let asset_list: Vec<serde_json::Value> = assets.into_iter().map(|a| serde_json::json!({
        "id": a.id, "hostname": a.hostname, "device_type": a.device_type,
        "role": a.role, "manufacturer": a.manufacturer, "model": a.model,
        "status": a.status, "management_ip": a.management_ip,
    })).collect();

    let wan_list: Vec<serde_json::Value> = wan_connections.into_iter().map(|w| serde_json::json!({
        "id": w.id, "name": w.name, "provider": w.provider, "connection_type": w.connection_type,
        "role": w.role, "status": w.status,
        "bandwidth_down": w.bandwidth_down, "bandwidth_up": w.bandwidth_up,
    })).collect();

    let contact_list: Vec<serde_json::Value> = contacts.into_iter().map(|c| serde_json::json!({
        "id": c.id, "display_name": c.display_name, "position": c.position,
        "phone": c.phone, "email": c.email, "is_primary": c.is_primary != 0,
    })).collect();

    let doc_list: Vec<serde_json::Value> = docs.into_iter().map(|d| serde_json::json!({
        "id": d.id, "title": d.title, "category": d.category,
        "visibility": d.visibility, "is_pinned": d.is_pinned != 0, "updated_at": d.updated_at,
    })).collect();

    let job_list: Vec<serde_json::Value> = jobs.into_iter().map(|j| serde_json::json!({
        "id": j.id, "job_number": j.job_number, "title": j.title,
        "status": j.status, "priority": j.priority, "scheduled_start": j.scheduled_start,
    })).collect();

    state.render("locations/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        location => serde_json::json!({
            "id": loc.id, "site_code": loc.site_code, "name": loc.name,
            "customer_id": loc.customer_id, "customer_name": loc.customer_name,
            "street": loc.street, "house_number": loc.house_number,
            "zip": loc.zip, "city": loc.city, "country": loc.country,
            "building": loc.building, "floor": loc.floor,
            "room_notes": loc.room_notes, "rack_notes": loc.rack_notes,
            "access_notes": loc.access_notes, "opening_hours": loc.opening_hours,
            "parking_notes": loc.parking_notes, "technical_notes": loc.technical_notes,
            "service_notes": loc.service_notes, "status": loc.status,
            "network_range": loc.network_range,
            "vlan_ids": loc.vlan_ids,
            "dns_servers": loc.dns_servers,
        }),
        assets => asset_list,
        wan_connections => wan_list,
        contacts => contact_list,
        docs => doc_list,
        jobs => job_list,
    })
}

#[derive(Deserialize)]
pub struct LocationForm {
    pub name: String,
    pub customer_id: i64,
    pub street: Option<String>,
    pub house_number: Option<String>,
    pub zip: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub building: Option<String>,
    pub floor: Option<String>,
    pub room_notes: Option<String>,
    pub rack_notes: Option<String>,
    pub access_notes: Option<String>,
    pub opening_hours: Option<String>,
    pub parking_notes: Option<String>,
    pub technical_notes: Option<String>,
    pub internal_notes: Option<String>,
    pub service_notes: Option<String>,
    pub status: Option<String>,
    // Network fields
    pub network_range: Option<String>,
    pub vlan_ids: Option<String>,
    pub dns_servers: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let customer_opts: Vec<serde_json::Value> = customers.into_iter()
        .map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect();

    state.render("locations/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        location => Option::<serde_json::Value>::None,
        customers => customer_opts,
        preselect_customer_id => q.customer_id,
        title => "Neuer Standort",
        action => "/locations/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<LocationForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_WRITE)?;

    if form.name.trim().is_empty() {
        return Err(AppError::bad_request("Name ist erforderlich"));
    }

    // Generate site code
    let city = form.city.as_deref().unwrap_or("X");
    let zip = form.zip.as_deref().unwrap_or("");
    let street = form.street.as_deref().unwrap_or("");
    let country = form.country.as_deref().unwrap_or("DE");

    let site_code = naming::generate_site_code(&state.db, city, zip, street, "", country).await?;
    let status = form.status.as_deref().unwrap_or("active");

    let id = sqlx::query!(
        "INSERT INTO locations (site_code, name, customer_id, street, house_number, zip, city, country,
         building, floor, room_notes, rack_notes, access_notes, opening_hours, parking_notes,
         technical_notes, internal_notes, service_notes, status,
         network_range, vlan_ids, dns_servers)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        site_code, form.name, form.customer_id, form.street, form.house_number,
        form.zip, form.city, country,
        form.building, form.floor, form.room_notes, form.rack_notes, form.access_notes,
        form.opening_hours, form.parking_notes, form.technical_notes,
        form.internal_notes, form.service_notes, status,
        form.network_range, form.vlan_ids, form.dns_servers
    )
    .execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "location", Some(&id.to_string()),
        Some(&format!("Created location: {} ({})", form.name, site_code)), None, true).await;

    Ok(Redirect::to(&format!("/locations/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_WRITE)?;

    let loc = sqlx::query!("SELECT * FROM locations WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let customer_opts: Vec<serde_json::Value> = customers.into_iter()
        .map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect();

    state.render("locations/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        location => serde_json::json!({
            "id": loc.id, "site_code": loc.site_code, "name": loc.name,
            "customer_id": loc.customer_id, "street": loc.street, "house_number": loc.house_number,
            "zip": loc.zip, "city": loc.city, "country": loc.country,
            "building": loc.building, "floor": loc.floor,
            "room_notes": loc.room_notes, "rack_notes": loc.rack_notes,
            "access_notes": loc.access_notes, "opening_hours": loc.opening_hours,
            "parking_notes": loc.parking_notes, "technical_notes": loc.technical_notes,
            "service_notes": loc.service_notes, "status": loc.status,
            "network_range": loc.network_range, "vlan_ids": loc.vlan_ids, "dns_servers": loc.dns_servers,
        }),
        customers => customer_opts,
        title => "Standort bearbeiten",
        action => format!("/locations/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<LocationForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_WRITE)?;

    let _existing = sqlx::query!("SELECT id FROM locations WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let country = form.country.as_deref().unwrap_or("DE");
    let status = form.status.as_deref().unwrap_or("active");

    sqlx::query!(
        "UPDATE locations SET name=?, customer_id=?, street=?, house_number=?, zip=?, city=?, country=?,
         building=?, floor=?, room_notes=?, rack_notes=?, access_notes=?, opening_hours=?,
         parking_notes=?, technical_notes=?, internal_notes=?, service_notes=?, status=?,
         network_range=?, vlan_ids=?, dns_servers=?,
         updated_at=datetime('now') WHERE id=?",
        form.name, form.customer_id, form.street, form.house_number, form.zip, form.city, country,
        form.building, form.floor, form.room_notes, form.rack_notes, form.access_notes,
        form.opening_hours, form.parking_notes, form.technical_notes,
        form.internal_notes, form.service_notes, status,
        form.network_range, form.vlan_ids, form.dns_servers, id
    )
    .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "location", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/locations/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(LOCATIONS_DELETE)?;

    sqlx::query!("UPDATE locations SET status='deleted', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "location", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/locations"))
}
