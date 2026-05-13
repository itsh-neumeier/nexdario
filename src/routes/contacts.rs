use crate::{auth::AuthUser, db, error::AppError, permissions::*, services::audit, state::AppState};
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
    auth.require_permission(CONTACTS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let items: Vec<serde_json::Value> = if let Some(cid) = q.customer_id {
        sqlx::query!(
            "SELECT c.id, c.first_name, c.last_name, c.display_name, c.position, c.phone, c.email,
             c.is_primary, c.status, cu.name as customer_name, c.customer_id
             FROM contacts c LEFT JOIN customers cu ON cu.id = c.customer_id
             WHERE c.customer_id = ? AND (c.display_name LIKE ? OR c.email LIKE ?)
             ORDER BY c.is_primary DESC, c.last_name LIMIT 200",
            cid, like, like
        )
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|c| serde_json::json!({
            "id": c.id, "first_name": c.first_name, "last_name": c.last_name,
            "display_name": c.display_name, "position": c.position,
            "phone": c.phone, "email": c.email,
            "is_primary": c.is_primary != 0, "status": c.status,
            "customer_name": c.customer_name, "customer_id": c.customer_id,
        }))
        .collect()
    } else {
        sqlx::query!(
            "SELECT c.id, c.first_name, c.last_name, c.display_name, c.position, c.phone, c.email,
             c.is_primary, c.status, cu.name as customer_name, c.customer_id
             FROM contacts c LEFT JOIN customers cu ON cu.id = c.customer_id
             WHERE c.display_name LIKE ? OR c.email LIKE ?
             ORDER BY c.last_name LIMIT 200",
            like, like
        )
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|c| serde_json::json!({
            "id": c.id, "first_name": c.first_name, "last_name": c.last_name,
            "display_name": c.display_name, "position": c.position,
            "phone": c.phone, "email": c.email,
            "is_primary": c.is_primary != 0, "status": c.status,
            "customer_name": c.customer_name, "customer_id": c.customer_id,
        }))
        .collect()
    };

    // Load customers for filter
    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let customer_opts: Vec<serde_json::Value> = customers.into_iter()
        .map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect();

    state.render("contacts/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        contacts => items,
        search => search,
        customer_id_filter => q.customer_id,
        customers => customer_opts,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_READ)?;

    let contact = sqlx::query!(
        "SELECT c.*, cu.name as customer_name FROM contacts c
         LEFT JOIN customers cu ON cu.id = c.customer_id WHERE c.id = ?", id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let is_manager_plus = auth.has_permission(CONTACTS_PRIVACY_MANAGE);

    state.render("contacts/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        contact => serde_json::json!({
            "id": contact.id,
            "first_name": contact.first_name,
            "last_name": contact.last_name,
            "display_name": contact.display_name,
            "position": contact.position,
            "department": contact.department,
            "role": contact.role,
            "phone": contact.phone,
            "mobile": contact.mobile,
            "email": contact.email,
            "email_alt": contact.email_alt,
            "preferred_contact": contact.preferred_contact,
            "language": contact.language,
            "description": contact.description,
            "notes": if is_manager_plus { contact.notes.clone() } else { None },
            "is_primary": contact.is_primary != 0,
            "is_technical": contact.is_technical != 0,
            "is_commercial": contact.is_commercial != 0,
            "is_emergency": contact.is_emergency != 0,
            "status": contact.status,
            "customer_name": contact.customer_name,
            "customer_id": contact.customer_id,
            "name_visible_to_service": contact.name_visible_to_service != 0,
            "phone_visible_to_service": contact.phone_visible_to_service != 0,
            "email_visible_to_service": contact.email_visible_to_service != 0,
        }),
        can_manage_privacy => is_manager_plus,
    })
}

#[derive(Deserialize)]
pub struct ContactForm {
    pub customer_id: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub position: Option<String>,
    pub department: Option<String>,
    pub role: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub email_alt: Option<String>,
    pub preferred_contact: Option<String>,
    pub language: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub is_primary: Option<String>,
    pub is_technical: Option<String>,
    pub is_commercial: Option<String>,
    pub is_emergency: Option<String>,
    pub status: Option<String>,
    pub name_visible_to_service: Option<String>,
    pub phone_visible_to_service: Option<String>,
    pub mobile_visible_to_service: Option<String>,
    pub email_visible_to_service: Option<String>,
    pub role_visible_to_service: Option<String>,
    pub department_visible_to_service: Option<String>,
    pub description_visible_to_service: Option<String>,
    pub notes_visible_to_service: Option<String>,
}

fn bool_field(v: &Option<String>) -> i64 {
    match v.as_deref() { Some("on") | Some("true") | Some("1") => 1, _ => 0 }
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let customer_opts: Vec<serde_json::Value> = customers.into_iter()
        .map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect();

    state.render("contacts/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        contact => Option::<serde_json::Value>::None,
        customers => customer_opts,
        preselect_customer_id => q.customer_id,
        title => "Neuer Kontakt",
        action => "/contacts/new",
        can_manage_privacy => auth.has_permission(CONTACTS_PRIVACY_MANAGE),
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<ContactForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_WRITE)?;

    if form.first_name.trim().is_empty() && form.last_name.trim().is_empty() {
        return Err(AppError::bad_request("Vor- oder Nachname erforderlich"));
    }

    let display_name = format!("{} {}", form.first_name.trim(), form.last_name.trim()).trim().to_string();
    let status = form.status.as_deref().unwrap_or("active");
    let preferred_contact = form.preferred_contact.as_deref().unwrap_or("email");
    let language = form.language.as_deref().unwrap_or("de");
    let is_primary = bool_field(&form.is_primary);
    let is_technical = bool_field(&form.is_technical);
    let is_commercial = bool_field(&form.is_commercial);
    let is_emergency = bool_field(&form.is_emergency);
    let name_vis = bool_field(&form.name_visible_to_service);
    let phone_vis = bool_field(&form.phone_visible_to_service);
    let mobile_vis = bool_field(&form.mobile_visible_to_service);
    let email_vis = bool_field(&form.email_visible_to_service);
    let role_vis = bool_field(&form.role_visible_to_service);
    let dept_vis = bool_field(&form.department_visible_to_service);
    let desc_vis = bool_field(&form.description_visible_to_service);
    let notes_vis = bool_field(&form.notes_visible_to_service);

    let id = sqlx::query!(
        "INSERT INTO contacts (customer_id, first_name, last_name, display_name, position, department,
         role, phone, mobile, email, email_alt, preferred_contact, language, description, notes,
         is_primary, is_technical, is_commercial, is_emergency, status,
         name_visible_to_service, phone_visible_to_service, mobile_visible_to_service,
         email_visible_to_service, role_visible_to_service, department_visible_to_service,
         description_visible_to_service, notes_visible_to_service)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        form.customer_id, form.first_name, form.last_name, display_name,
        form.position, form.department, form.role, form.phone, form.mobile,
        form.email, form.email_alt,
        preferred_contact, language,
        form.description, form.notes,
        is_primary, is_technical, is_commercial, is_emergency, status,
        name_vis, phone_vis, mobile_vis, email_vis, role_vis, dept_vis, desc_vis, notes_vis
    )
    .execute(&state.db)
    .await?
    .last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "contact", Some(&id.to_string()),
        None, None, true).await;

    Ok(Redirect::to(&format!("/contacts/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_WRITE)?;

    let contact = sqlx::query!("SELECT * FROM contacts WHERE id = ?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;
    let customer_opts: Vec<serde_json::Value> = customers.into_iter()
        .map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect();

    state.render("contacts/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        contact => serde_json::json!({
            "id": contact.id, "customer_id": contact.customer_id,
            "first_name": contact.first_name, "last_name": contact.last_name,
            "position": contact.position, "department": contact.department, "role": contact.role,
            "phone": contact.phone, "mobile": contact.mobile, "email": contact.email, "email_alt": contact.email_alt,
            "preferred_contact": contact.preferred_contact, "language": contact.language,
            "description": contact.description, "notes": contact.notes,
            "is_primary": contact.is_primary != 0, "is_technical": contact.is_technical != 0,
            "is_commercial": contact.is_commercial != 0, "is_emergency": contact.is_emergency != 0,
            "status": contact.status,
            "name_visible_to_service": contact.name_visible_to_service != 0,
            "phone_visible_to_service": contact.phone_visible_to_service != 0,
            "email_visible_to_service": contact.email_visible_to_service != 0,
        }),
        customers => customer_opts,
        title => "Kontakt bearbeiten",
        action => format!("/contacts/{}/edit", id),
        can_manage_privacy => auth.has_permission(CONTACTS_PRIVACY_MANAGE),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<ContactForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_WRITE)?;

    let _existing = sqlx::query!("SELECT id FROM contacts WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let display_name = format!("{} {}", form.first_name.trim(), form.last_name.trim()).trim().to_string();
    let status = form.status.as_deref().unwrap_or("active");
    let preferred_contact = form.preferred_contact.as_deref().unwrap_or("email");
    let language = form.language.as_deref().unwrap_or("de");
    let is_primary = bool_field(&form.is_primary);
    let is_technical = bool_field(&form.is_technical);
    let is_commercial = bool_field(&form.is_commercial);
    let is_emergency = bool_field(&form.is_emergency);
    let name_vis = bool_field(&form.name_visible_to_service);
    let phone_vis = bool_field(&form.phone_visible_to_service);
    let mobile_vis = bool_field(&form.mobile_visible_to_service);
    let email_vis = bool_field(&form.email_visible_to_service);
    let role_vis = bool_field(&form.role_visible_to_service);
    let dept_vis = bool_field(&form.department_visible_to_service);
    let desc_vis = bool_field(&form.description_visible_to_service);
    let notes_vis = bool_field(&form.notes_visible_to_service);

    sqlx::query!(
        "UPDATE contacts SET customer_id=?, first_name=?, last_name=?, display_name=?, position=?,
         department=?, role=?, phone=?, mobile=?, email=?, email_alt=?, preferred_contact=?,
         language=?, description=?, notes=?, is_primary=?, is_technical=?, is_commercial=?,
         is_emergency=?, status=?, name_visible_to_service=?, phone_visible_to_service=?,
         mobile_visible_to_service=?, email_visible_to_service=?, role_visible_to_service=?,
         department_visible_to_service=?, description_visible_to_service=?, notes_visible_to_service=?,
         updated_at=datetime('now') WHERE id=?",
        form.customer_id, form.first_name, form.last_name, display_name,
        form.position, form.department, form.role, form.phone, form.mobile,
        form.email, form.email_alt,
        preferred_contact, language,
        form.description, form.notes,
        is_primary, is_technical, is_commercial, is_emergency, status,
        name_vis, phone_vis, mobile_vis, email_vis, role_vis, dept_vis, desc_vis, notes_vis,
        id
    )
    .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "contact", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/contacts/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CONTACTS_DELETE)?;

    sqlx::query!("UPDATE contacts SET status='deleted', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "contact", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/contacts"))
}
