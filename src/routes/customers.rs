use crate::{
    auth::AuthUser,
    db,
    error::AppError,
    permissions::*,
    services::audit,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let status = q.status.as_deref().unwrap_or("active");
    let like = format!("%{}%", search);

    let customers = if status == "all" {
        sqlx::query!(
            "SELECT id, customer_number, name, customer_type, status, industry, phone, email, billing_city
             FROM customers WHERE (name LIKE ? OR customer_number LIKE ? OR email LIKE ?)
             ORDER BY name LIMIT 200",
            like, like, like
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query!(
            "SELECT id, customer_number, name, customer_type, status, industry, phone, email, billing_city
             FROM customers WHERE status = ? AND (name LIKE ? OR customer_number LIKE ? OR email LIKE ?)
             ORDER BY name LIMIT 200",
            status, like, like, like
        )
        .fetch_all(&state.db)
        .await?
    };

    let items: Vec<serde_json::Value> = customers
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "customer_number": c.customer_number,
                "name": c.name,
                "customer_type": c.customer_type,
                "status": c.status,
                "industry": c.industry,
                "phone": c.phone,
                "email": c.email,
                "billing_city": c.billing_city,
            })
        })
        .collect();

    state.render(
        "customers/list.html",
        minijinja::context! {
            app_name => &state.config.app_name,
            user => &auth,
            customers => items,
            search => search,
            status_filter => status,
        },
    )
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_READ)?;

    let customer = sqlx::query!(
        "SELECT * FROM customers WHERE id = ?",
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Load contacts for this customer
    let contacts = sqlx::query!(
        "SELECT id, first_name, last_name, display_name, position, phone, mobile, email, is_primary, status
         FROM contacts WHERE customer_id = ? ORDER BY is_primary DESC, last_name",
        id
    )
    .fetch_all(&state.db)
    .await?;

    // Load locations
    let locations = sqlx::query!(
        "SELECT id, site_code, name, city, status FROM locations WHERE customer_id = ? ORDER BY name",
        id
    )
    .fetch_all(&state.db)
    .await?;

    // Load recent quotes
    let quotes = sqlx::query!(
        "SELECT id, quote_number, title, status, total, created_at FROM quotes
         WHERE customer_id = ? ORDER BY created_at DESC LIMIT 5",
        id
    )
    .fetch_all(&state.db)
    .await?;

    // Load recent invoices
    let invoices = sqlx::query!(
        "SELECT id, invoice_number, status, total, invoice_date FROM invoices
         WHERE customer_id = ? ORDER BY created_at DESC LIMIT 5",
        id
    )
    .fetch_all(&state.db)
    .await?;

    let contact_list: Vec<serde_json::Value> = contacts.into_iter().map(|c| serde_json::json!({
        "id": c.id, "first_name": c.first_name, "last_name": c.last_name,
        "display_name": c.display_name, "position": c.position,
        "phone": c.phone, "mobile": c.mobile, "email": c.email,
        "is_primary": c.is_primary != 0, "status": c.status,
    })).collect();

    let location_list: Vec<serde_json::Value> = locations.into_iter().map(|l| serde_json::json!({
        "id": l.id, "site_code": l.site_code, "name": l.name, "city": l.city, "status": l.status,
    })).collect();

    let quote_list: Vec<serde_json::Value> = quotes.into_iter().map(|q| serde_json::json!({
        "id": q.id, "quote_number": q.quote_number, "title": q.title,
        "status": q.status, "total": q.total, "created_at": q.created_at,
    })).collect();

    let invoice_list: Vec<serde_json::Value> = invoices.into_iter().map(|i| serde_json::json!({
        "id": i.id, "invoice_number": i.invoice_number, "status": i.status,
        "total": i.total, "invoice_date": i.invoice_date,
    })).collect();

    state.render(
        "customers/detail.html",
        minijinja::context! {
            app_name => &state.config.app_name,
            user => &auth,
            customer => serde_json::json!({
                "id": customer.id,
                "customer_number": customer.customer_number,
                "name": customer.name,
                "customer_type": customer.customer_type,
                "status": customer.status,
                "industry": customer.industry,
                "website": customer.website,
                "phone": customer.phone,
                "email": customer.email,
                "billing_street": customer.billing_street,
                "billing_zip": customer.billing_zip,
                "billing_city": customer.billing_city,
                "billing_country": customer.billing_country,
                "vat_id": customer.vat_id,
                "payment_terms": customer.payment_terms,
                "notes": customer.notes,
                "debtor_account": customer.debtor_account,
            }),
            contacts => contact_list,
            locations => location_list,
            quotes => quote_list,
            invoices => invoice_list,
        },
    )
}

#[derive(Deserialize)]
pub struct CustomerForm {
    pub name: String,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    pub industry: Option<String>,
    pub website: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub billing_street: Option<String>,
    pub billing_zip: Option<String>,
    pub billing_city: Option<String>,
    pub billing_country: Option<String>,
    pub vat_id: Option<String>,
    pub tax_country: Option<String>,
    pub payment_terms: Option<String>,
    pub debtor_account: Option<String>,
    pub notes: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    state.render(
        "customers/form.html",
        minijinja::context! {
            app_name => &state.config.app_name,
            user => &auth,
            customer => Option::<serde_json::Value>::None,
            title => "Neuer Kunde",
            action => "/customers/new",
        },
    )
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<CustomerForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    if form.name.trim().is_empty() {
        return Err(AppError::bad_request("Name ist erforderlich"));
    }

    let number = db::next_number(&state.db, "customer").await?;
    let customer_type = form.customer_type.as_deref().unwrap_or("business");
    let status = form.status.as_deref().unwrap_or("active");
    let billing_country = form.billing_country.as_deref().unwrap_or("DE");
    let payment_terms = form.payment_terms.as_deref().unwrap_or("14");

    let id = sqlx::query!(
        "INSERT INTO customers (customer_number, name, customer_type, status, industry, website,
         phone, email, billing_street, billing_zip, billing_city, billing_country,
         vat_id, tax_country, payment_terms, debtor_account, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        number, form.name, customer_type, status,
        form.industry, form.website, form.phone, form.email,
        form.billing_street, form.billing_zip, form.billing_city, billing_country,
        form.vat_id, form.tax_country.as_deref().unwrap_or("DE"), payment_terms,
        form.debtor_account, form.notes
    )
    .execute(&state.db)
    .await?
    .last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "customer", Some(&id.to_string()),
        Some(&format!("Created customer: {}", form.name)), None, true).await;

    Ok(Redirect::to(&format!("/customers/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    let customer = sqlx::query!(
        "SELECT * FROM customers WHERE id = ?", id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    state.render(
        "customers/form.html",
        minijinja::context! {
            app_name => &state.config.app_name,
            user => &auth,
            customer => serde_json::json!({
                "id": customer.id,
                "customer_number": customer.customer_number,
                "name": customer.name,
                "customer_type": customer.customer_type,
                "status": customer.status,
                "industry": customer.industry,
                "website": customer.website,
                "phone": customer.phone,
                "email": customer.email,
                "billing_street": customer.billing_street,
                "billing_zip": customer.billing_zip,
                "billing_city": customer.billing_city,
                "billing_country": customer.billing_country,
                "vat_id": customer.vat_id,
                "tax_country": customer.tax_country,
                "payment_terms": customer.payment_terms,
                "debtor_account": customer.debtor_account,
                "notes": customer.notes,
            }),
            title => format!("Kunde bearbeiten: {}", customer.name),
            action => format!("/customers/{}/edit", id),
        },
    )
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<CustomerForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    // Check exists
    let _existing = sqlx::query!("SELECT id FROM customers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    if form.name.trim().is_empty() {
        return Err(AppError::bad_request("Name ist erforderlich"));
    }

    let customer_type = form.customer_type.as_deref().unwrap_or("business");
    let status = form.status.as_deref().unwrap_or("active");
    let billing_country = form.billing_country.as_deref().unwrap_or("DE");
    let payment_terms = form.payment_terms.as_deref().unwrap_or("14");

    sqlx::query!(
        "UPDATE customers SET name=?, customer_type=?, status=?, industry=?, website=?,
         phone=?, email=?, billing_street=?, billing_zip=?, billing_city=?, billing_country=?,
         vat_id=?, tax_country=?, payment_terms=?, debtor_account=?, notes=?,
         updated_at=datetime('now') WHERE id=?",
        form.name, customer_type, status, form.industry, form.website,
        form.phone, form.email, form.billing_street, form.billing_zip,
        form.billing_city, billing_country, form.vat_id,
        form.tax_country.as_deref().unwrap_or("DE"), payment_terms,
        form.debtor_account, form.notes, id
    )
    .execute(&state.db)
    .await?;

    audit::log(&state.db, Some(&auth), "update", "customer", Some(&id.to_string()),
        Some(&format!("Updated customer: {}", form.name)), None, true).await;

    Ok(Redirect::to(&format!("/customers/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(CUSTOMERS_DELETE)?;

    let customer = sqlx::query!("SELECT name FROM customers WHERE id = ?", id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    sqlx::query!("UPDATE customers SET status='deleted', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db)
        .await?;

    audit::log(&state.db, Some(&auth), "delete", "customer", Some(&id.to_string()),
        Some(&format!("Deleted customer: {}", customer.name)), None, true).await;

    Ok(Redirect::to("/customers"))
}
