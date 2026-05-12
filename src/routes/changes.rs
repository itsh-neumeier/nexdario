use crate::{auth::AuthUser, db, error::AppError, permissions::*, services::audit, state::AppState};
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery { pub search: Option<String>, pub status: Option<String> }

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let changes = sqlx::query!(
        "SELECT ch.id, ch.change_number, ch.title, ch.category, ch.status, ch.risk_level,
         ch.scheduled_start, c.name as customer_name
         FROM changes ch LEFT JOIN customers c ON c.id=ch.customer_id
         WHERE ch.change_number LIKE ? OR ch.title LIKE ?
         ORDER BY ch.created_at DESC LIMIT 200",
        like, like
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = changes.into_iter().map(|c| serde_json::json!({
        "id": c.id, "change_number": c.change_number, "title": c.title,
        "category": c.category, "status": c.status, "risk_level": c.risk_level,
        "scheduled_start": c.scheduled_start, "customer_name": c.customer_name,
    })).collect();

    state.render("changes/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        changes => items,
        search => search,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_READ)?;

    let change = sqlx::query!(
        "SELECT ch.*, c.name as customer_name, l.name as location_name
         FROM changes ch
         LEFT JOIN customers c ON c.id=ch.customer_id
         LEFT JOIN locations l ON l.id=ch.location_id
         WHERE ch.id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("changes/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        change => serde_json::json!({
            "id": change.id, "change_number": change.change_number, "title": change.title,
            "category": change.category, "description": change.description, "status": change.status,
            "risk_level": change.risk_level, "impact": change.impact,
            "rollback_plan": change.rollback_plan, "test_plan": change.test_plan,
            "scheduled_start": change.scheduled_start, "scheduled_end": change.scheduled_end,
            "maintenance_window": change.maintenance_window,
            "customer_name": change.customer_name, "location_name": change.location_name,
        }),
        can_approve => auth.has_permission(CHANGES_APPROVE),
        can_close => auth.has_permission(CHANGES_CLOSE),
    })
}

#[derive(Deserialize)]
pub struct ChangeForm {
    pub customer_id: i64,
    pub location_id: Option<i64>,
    pub category: String,
    pub title: String,
    pub description: String,
    pub risk_level: Option<String>,
    pub impact: Option<String>,
    pub rollback_plan: Option<String>,
    pub test_plan: Option<String>,
    pub scheduled_start: Option<String>,
    pub scheduled_end: Option<String>,
    pub maintenance_window: Option<String>,
    pub assigned_employee_id: Option<i64>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let employees = sqlx::query!("SELECT id, first_name, last_name FROM employees WHERE is_active=1 ORDER BY last_name LIMIT 100")
        .fetch_all(&state.db).await?;

    let categories = vec![
        "Netzwerk", "Firewall", "WAN / Internet", "PPPoE / Providerzugang", "VPN",
        "Routing", "Switching", "VLAN", "WLAN", "UniFi", "Server", "Virtualisierung",
        "Storage", "Backup", "Monitoring", "Security", "Benutzer / Berechtigungen",
        "Cloud / Microsoft 365", "Telefonie / VoIP", "Dokumentation", "Firmware / Updates", "Hardwaretausch",
    ];

    state.render("changes/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        change => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        employees => employees.into_iter().map(|e| serde_json::json!({"id": e.id, "name": format!("{} {}", e.first_name, e.last_name)})).collect::<Vec<_>>(),
        categories => categories,
        title => "Neuer IT-Change",
        action => "/changes/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<ChangeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;

    let number = db::next_number(&state.db, "change").await?;
    let risk_level = form.risk_level.as_deref().unwrap_or("low");

    let id = sqlx::query!(
        "INSERT INTO changes (change_number, customer_id, location_id, category, title,
         description, status, risk_level, impact, rollback_plan, test_plan,
         scheduled_start, scheduled_end, maintenance_window, assigned_employee_id, created_by)
         VALUES (?,?,?,?,?,?,'draft',?,?,?,?,?,?,?,?,?)",
        number, form.customer_id, form.location_id, form.category, form.title,
        form.description, risk_level, form.impact, form.rollback_plan, form.test_plan,
        form.scheduled_start, form.scheduled_end, form.maintenance_window,
        form.assigned_employee_id, auth.id
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "change", Some(&id.to_string()),
        Some(&format!("Created change: {}", number)), None, true).await;

    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;

    let change = sqlx::query!("SELECT * FROM changes WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let employees = sqlx::query!("SELECT id, first_name, last_name FROM employees WHERE is_active=1 ORDER BY last_name LIMIT 100")
        .fetch_all(&state.db).await?;

    let categories = vec![
        "Netzwerk", "Firewall", "WAN / Internet", "PPPoE / Providerzugang",
        "VPN", "Routing", "Switching", "VLAN", "WLAN", "UniFi",
    ];

    state.render("changes/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        change => serde_json::json!({
            "id": change.id, "customer_id": change.customer_id, "location_id": change.location_id,
            "category": change.category, "title": change.title, "description": change.description,
            "risk_level": change.risk_level, "impact": change.impact,
            "rollback_plan": change.rollback_plan, "test_plan": change.test_plan,
            "scheduled_start": change.scheduled_start, "scheduled_end": change.scheduled_end,
            "maintenance_window": change.maintenance_window,
            "assigned_employee_id": change.assigned_employee_id,
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        employees => employees.into_iter().map(|e| serde_json::json!({"id": e.id, "name": format!("{} {}", e.first_name, e.last_name)})).collect::<Vec<_>>(),
        categories => categories,
        title => "IT-Change bearbeiten",
        action => format!("/changes/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<ChangeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;

    let risk_level = form.risk_level.as_deref().unwrap_or("low");

    sqlx::query!(
        "UPDATE changes SET customer_id=?, location_id=?, category=?, title=?, description=?,
         risk_level=?, impact=?, rollback_plan=?, test_plan=?,
         scheduled_start=?, scheduled_end=?, maintenance_window=?,
         assigned_employee_id=?, updated_at=datetime('now') WHERE id=?",
        form.customer_id, form.location_id, form.category, form.title, form.description,
        risk_level, form.impact, form.rollback_plan, form.test_plan,
        form.scheduled_start, form.scheduled_end, form.maintenance_window,
        form.assigned_employee_id, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "change", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn approve(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_APPROVE)?;

    sqlx::query!(
        "UPDATE changes SET status='approved', approved_by=?, approved_at=datetime('now'),
         updated_at=datetime('now') WHERE id=?",
        auth.id, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "approve", "change", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn close(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_CLOSE)?;

    sqlx::query!(
        "UPDATE changes SET status='closed', closed_by=?, closed_at=datetime('now'),
         updated_at=datetime('now') WHERE id=?",
        auth.id, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "close", "change", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;
    sqlx::query!("UPDATE changes SET status='cancelled', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    Ok(Redirect::to("/changes"))
}

pub async fn submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;
    sqlx::query!("UPDATE changes SET status='pending', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "submit", "change", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn reject(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_APPROVE)?;
    sqlx::query!("UPDATE changes SET status='rejected', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "reject", "change", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/changes/{}", id)))
}

pub async fn start(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CHANGES_WRITE)?;
    sqlx::query!("UPDATE changes SET status='in_progress', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "start", "change", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/changes/{}", id)))
}
