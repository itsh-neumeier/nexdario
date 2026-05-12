use crate::{auth::AuthUser, db, error::AppError, permissions::*, services::audit, state::AppState};
use axum::{extract::{Path, State}, response::{IntoResponse, Redirect}, Form};
use serde::Deserialize;

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_READ)?;

    let employees = sqlx::query!(
        "SELECT id, employee_number, first_name, last_name, email, position, department, is_active
         FROM employees ORDER BY last_name, first_name LIMIT 200"
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = employees.into_iter().map(|e| serde_json::json!({
        "id": e.id, "employee_number": e.employee_number,
        "first_name": e.first_name, "last_name": e.last_name,
        "email": e.email, "position": e.position, "department": e.department,
        "is_active": e.is_active != 0,
    })).collect();

    state.render("employees/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        employees => items,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_READ)?;

    let emp = sqlx::query!("SELECT * FROM employees WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("employees/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        employee => serde_json::json!({
            "id": emp.id, "employee_number": emp.employee_number,
            "first_name": emp.first_name, "last_name": emp.last_name,
            "email": emp.email, "phone": emp.phone, "mobile": emp.mobile,
            "position": emp.position, "department": emp.department,
            "qualifications": emp.qualifications, "is_active": emp.is_active != 0,
        }),
    })
}

#[derive(Deserialize)]
pub struct EmployeeForm {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub position: Option<String>,
    pub department: Option<String>,
    pub qualifications: Option<String>,
    pub hourly_rate: Option<f64>,
    pub is_active: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_WRITE)?;
    state.render("employees/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        employee => Option::<serde_json::Value>::None,
        title => "Neuer Mitarbeiter",
        action => "/employees/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<EmployeeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_WRITE)?;

    let number = db::next_number(&state.db, "employee").await?;

    let id = sqlx::query!(
        "INSERT INTO employees (employee_number, first_name, last_name, email, phone, mobile,
         position, department, qualifications, hourly_rate, is_active)
         VALUES (?,?,?,?,?,?,?,?,?,?,1)",
        number, form.first_name, form.last_name, form.email, form.phone, form.mobile,
        form.position, form.department, form.qualifications, form.hourly_rate
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "employee", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/employees/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_WRITE)?;

    let emp = sqlx::query!("SELECT * FROM employees WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    state.render("employees/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        employee => serde_json::json!({
            "id": emp.id, "first_name": emp.first_name, "last_name": emp.last_name,
            "email": emp.email, "phone": emp.phone, "mobile": emp.mobile,
            "position": emp.position, "department": emp.department,
            "qualifications": emp.qualifications, "hourly_rate": emp.hourly_rate,
            "is_active": emp.is_active != 0,
        }),
        title => "Mitarbeiter bearbeiten",
        action => format!("/employees/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<EmployeeForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_WRITE)?;

    let is_active = match form.is_active.as_deref() { Some("on") | Some("1") => 1i64, _ => 0 };

    sqlx::query!(
        "UPDATE employees SET first_name=?, last_name=?, email=?, phone=?, mobile=?,
         position=?, department=?, qualifications=?, hourly_rate=?, is_active=?,
         updated_at=datetime('now') WHERE id=?",
        form.first_name, form.last_name, form.email, form.phone, form.mobile,
        form.position, form.department, form.qualifications, form.hourly_rate, is_active, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "employee", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/employees/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(EMPLOYEES_WRITE)?;

    sqlx::query!("UPDATE employees SET is_active=0 WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "employee", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/employees"))
}
