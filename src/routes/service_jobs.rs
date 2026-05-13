use crate::{auth::AuthUser, db, error::AppError, permissions::*, services::audit, state::AppState};
#[allow(unused_imports)]
use chrono;
use axum::{extract::{Path, Query, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery { pub search: Option<String>, pub status: Option<String> }

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let jobs = sqlx::query!(
        "SELECT j.id, j.job_number, j.title, j.status, j.priority, j.scheduled_start, j.is_billable,
         c.name as customer_name, l.name as location_name, e.first_name || ' ' || e.last_name as technician
         FROM service_jobs j
         LEFT JOIN customers c ON c.id=j.customer_id
         LEFT JOIN locations l ON l.id=j.location_id
         LEFT JOIN employees e ON e.id=j.assigned_employee_id
         WHERE j.job_number LIKE ? OR j.title LIKE ? OR c.name LIKE ?
         ORDER BY j.created_at DESC LIMIT 200",
        like, like, like
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = jobs.into_iter().map(|j| serde_json::json!({
        "id": j.id, "job_number": j.job_number, "title": j.title, "status": j.status,
        "priority": j.priority, "scheduled_start": j.scheduled_start,
        "is_billable": j.is_billable != 0, "customer_name": j.customer_name,
        "location_name": j.location_name, "technician": j.technician,
    })).collect();

    state.render("service_jobs/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        jobs => items,
        search => search,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_READ)?;

    let job = sqlx::query!(
        "SELECT j.*, c.name as customer_name, l.name as location_name
         FROM service_jobs j
         LEFT JOIN customers c ON c.id=j.customer_id
         LEFT JOIN locations l ON l.id=j.location_id
         WHERE j.id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let time_entries = sqlx::query!(
        "SELECT t.id, t.activity_type, t.description, t.started_at, t.ended_at,
         t.duration_minutes, t.kilometers, t.is_billable,
         e.first_name || ' ' || e.last_name as employee_name
         FROM time_entries t LEFT JOIN employees e ON e.id=t.employee_id
         WHERE t.service_job_id=? ORDER BY t.started_at", id
    ).fetch_all(&state.db).await?;

    state.render("service_jobs/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        job => serde_json::json!({
            "id": job.id, "job_number": job.job_number, "title": job.title,
            "description": job.description, "status": job.status, "priority": job.priority,
            "scheduled_start": job.scheduled_start, "scheduled_end": job.scheduled_end,
            "actual_start": job.actual_start, "actual_end": job.actual_end,
            "is_billable": job.is_billable != 0, "notes": job.notes,
            "customer_name": job.customer_name, "location_name": job.location_name,
            "customer_id": job.customer_id,
        }),
        time_entries => time_entries.into_iter().map(|t| serde_json::json!({
            "id": t.id, "activity_type": t.activity_type, "description": t.description,
            "started_at": t.started_at, "ended_at": t.ended_at,
            "duration_minutes": t.duration_minutes, "kilometers": t.kilometers,
            "is_billable": t.is_billable != 0, "employee_name": t.employee_name,
        })).collect::<Vec<_>>(),
    })
}

#[derive(Deserialize)]
pub struct ServiceJobForm {
    pub customer_id: i64,
    pub location_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub assigned_employee_id: Option<i64>,
    pub scheduled_start: Option<String>,
    pub scheduled_end: Option<String>,
    pub is_billable: Option<String>,
    pub notes: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let employees = sqlx::query!("SELECT id, first_name, last_name FROM employees WHERE is_active=1 ORDER BY last_name LIMIT 100")
        .fetch_all(&state.db).await?;

    state.render("service_jobs/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        job => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        employees => employees.into_iter().map(|e| serde_json::json!({"id": e.id, "name": format!("{} {}", e.first_name, e.last_name)})).collect::<Vec<_>>(),
        title => "Neuer Serviceeinsatz",
        action => "/service-jobs/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<ServiceJobForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;

    let number = db::next_number(&state.db, "service_job").await?;
    let priority = form.priority.as_deref().unwrap_or("normal");
    let is_billable = match form.is_billable.as_deref() { Some("on") | Some("1") => 1i64, _ => 1 };

    let id = sqlx::query!(
        "INSERT INTO service_jobs (job_number, customer_id, location_id, title, description,
         priority, status, assigned_employee_id, scheduled_start, scheduled_end, is_billable, notes, created_by)
         VALUES (?,?,?,?,?,?,'open',?,?,?,?,?,?)",
        number, form.customer_id, form.location_id, form.title, form.description,
        priority, form.assigned_employee_id, form.scheduled_start, form.scheduled_end,
        is_billable, form.notes, auth.id
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "service_job", Some(&id.to_string()),
        Some(&format!("Created service job: {}", number)), None, true).await;

    Ok(Redirect::to(&format!("/service-jobs/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;

    let job = sqlx::query!("SELECT * FROM service_jobs WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let employees = sqlx::query!("SELECT id, first_name, last_name FROM employees WHERE is_active=1 ORDER BY last_name LIMIT 100")
        .fetch_all(&state.db).await?;

    state.render("service_jobs/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        job => serde_json::json!({
            "id": job.id, "customer_id": job.customer_id, "location_id": job.location_id,
            "title": job.title, "description": job.description, "priority": job.priority,
            "status": job.status, "assigned_employee_id": job.assigned_employee_id,
            "scheduled_start": job.scheduled_start, "scheduled_end": job.scheduled_end,
            "is_billable": job.is_billable != 0, "notes": job.notes,
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        employees => employees.into_iter().map(|e| serde_json::json!({"id": e.id, "name": format!("{} {}", e.first_name, e.last_name)})).collect::<Vec<_>>(),
        title => "Serviceeinsatz bearbeiten",
        action => format!("/service-jobs/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<ServiceJobForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;

    let priority = form.priority.as_deref().unwrap_or("normal");
    let is_billable = match form.is_billable.as_deref() { Some("on") | Some("1") => 1i64, _ => 1 };

    sqlx::query!(
        "UPDATE service_jobs SET customer_id=?, location_id=?, title=?, description=?,
         priority=?, assigned_employee_id=?, scheduled_start=?, scheduled_end=?,
         is_billable=?, notes=?, updated_at=datetime('now') WHERE id=?",
        form.customer_id, form.location_id, form.title, form.description,
        priority, form.assigned_employee_id, form.scheduled_start, form.scheduled_end,
        is_billable, form.notes, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "service_job", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/service-jobs/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;
    sqlx::query!("UPDATE service_jobs SET status='cancelled', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    Ok(Redirect::to("/service-jobs"))
}

pub async fn start(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;
    sqlx::query!("UPDATE service_jobs SET status='in_progress', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "start", "service_job", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/service-jobs/{}", id)))
}

pub async fn complete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;
    sqlx::query!(
        "UPDATE service_jobs SET status='done', actual_end=datetime('now'), updated_at=datetime('now') WHERE id=?", id
    ).execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "complete", "service_job", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/service-jobs/{}", id)))
}

pub async fn time_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;
    let job = sqlx::query!("SELECT id, title FROM service_jobs WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;
    state.render("service_jobs/time_form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        job => serde_json::json!({ "id": job.id, "title": job.title }),
    })
}

#[derive(Deserialize)]
pub struct TimeEntryForm {
    pub duration_minutes: Option<i64>,
    pub started_at: Option<String>,
    pub description: Option<String>,
    pub activity_type: Option<String>,
}

pub async fn add_time(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<TimeEntryForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(SERVICE_JOBS_WRITE)?;
    let started_at = form.started_at.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string());
    let duration = form.duration_minutes.unwrap_or(0);
    let activity = form.activity_type.as_deref().unwrap_or("work");
    let description = form.description.as_deref().unwrap_or("");
    let employee_id: Option<i64> = sqlx::query_scalar!(
        "SELECT id FROM employees WHERE user_id=? LIMIT 1", auth.id
    ).fetch_optional(&state.db).await?.flatten();
    if let Some(emp_id) = employee_id {
        sqlx::query!(
            "INSERT INTO time_entries (service_job_id, employee_id, activity_type, description, started_at, duration_minutes, is_billable)
             VALUES (?, ?, ?, ?, ?, ?, 1)",
            id, emp_id, activity, description, started_at, duration
        ).execute(&state.db).await?;
    }
    Ok(Redirect::to(&format!("/service-jobs/{}", id)))
}
