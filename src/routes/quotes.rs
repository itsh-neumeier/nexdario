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
    auth.require_permission(QUOTES_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let quotes = sqlx::query!(
        "SELECT q.id, q.quote_number, q.title, q.status, q.total, q.created_at, q.valid_until,
         c.name as customer_name
         FROM quotes q LEFT JOIN customers c ON c.id=q.customer_id
         WHERE q.quote_number LIKE ? OR q.title LIKE ? OR c.name LIKE ?
         ORDER BY q.created_at DESC LIMIT 200",
        like, like, like
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = quotes.into_iter().map(|q| serde_json::json!({
        "id": q.id, "quote_number": q.quote_number, "title": q.title, "status": q.status,
        "total": q.total, "created_at": q.created_at, "valid_until": q.valid_until,
        "customer_name": q.customer_name,
    })).collect();

    state.render("quotes/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        quotes => items,
        search => search,
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_READ)?;

    let quote = sqlx::query!(
        "SELECT q.*, c.name as customer_name FROM quotes q
         LEFT JOIN customers c ON c.id=q.customer_id WHERE q.id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let items = sqlx::query!(
        "SELECT * FROM quote_items WHERE quote_id=? ORDER BY position", id
    ).fetch_all(&state.db).await?;

    let item_list: Vec<serde_json::Value> = items.into_iter().map(|i| serde_json::json!({
        "id": i.id, "position": i.position, "name": i.name, "description": i.description,
        "quantity": i.quantity, "unit": i.unit, "unit_price": i.unit_price,
        "discount_percent": i.discount_percent, "tax_rate": i.tax_rate, "subtotal": i.subtotal,
        "is_optional": i.is_optional != 0, "item_type": i.item_type,
    })).collect();

    state.render("quotes/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        quote => serde_json::json!({
            "id": quote.id, "quote_number": quote.quote_number, "title": quote.title,
            "status": quote.status, "version": quote.version,
            "subtotal": quote.subtotal, "discount_amount": quote.discount_amount,
            "tax_amount": quote.tax_amount, "total": quote.total,
            "notes": quote.notes, "valid_until": quote.valid_until,
            "customer_id": quote.customer_id, "customer_name": quote.customer_name,
            "created_at": quote.created_at,
        }),
        items => item_list,
        can_approve => auth.has_permission(QUOTES_APPROVE),
        can_write => auth.has_permission(QUOTES_WRITE),
    })
}

#[derive(Deserialize)]
pub struct QuoteForm {
    pub customer_id: Option<i64>,
    pub location_id: Option<i64>,
    pub title: String,
    pub valid_until: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
    pub internal_notes: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("quotes/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        quote => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        title => "Neues Angebot",
        action => "/quotes/new",
    })
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<QuoteForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;

    let customer_id = form.customer_id.ok_or_else(|| AppError::bad_request("Kunde ist erforderlich"))?;
    let number = db::next_number(&state.db, "quote").await?;

    let id = sqlx::query!(
        "INSERT INTO quotes (quote_number, customer_id, location_id, title, status, payment_terms,
         notes, internal_notes, valid_until, created_by)
         VALUES (?,?,?,?,'draft',?,?,?,?,?)",
        number, customer_id, form.location_id, form.title,
        form.payment_terms, form.notes, form.internal_notes, form.valid_until, auth.id
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "quote", Some(&id.to_string()),
        Some(&format!("Created quote: {}", number)), None, true).await;

    Ok(Redirect::to(&format!("/quotes/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;

    let quote = sqlx::query!("SELECT * FROM quotes WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    if quote.status != "draft" && !auth.is_superadmin {
        return Err(AppError::bad_request("Nur Entwürfe können bearbeitet werden"));
    }

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("quotes/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        quote => serde_json::json!({
            "id": quote.id, "customer_id": quote.customer_id, "location_id": quote.location_id,
            "title": quote.title, "valid_until": quote.valid_until,
            "payment_terms": quote.payment_terms, "notes": quote.notes,
            "internal_notes": quote.internal_notes, "status": quote.status,
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        title => "Angebot bearbeiten",
        action => format!("/quotes/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<QuoteForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;

    let customer_id = form.customer_id.ok_or_else(|| AppError::bad_request("Kunde ist erforderlich"))?;
    sqlx::query!(
        "UPDATE quotes SET customer_id=?, location_id=?, title=?, valid_until=?,
         payment_terms=?, notes=?, internal_notes=?, updated_at=datetime('now') WHERE id=?",
        customer_id, form.location_id, form.title, form.valid_until,
        form.payment_terms, form.notes, form.internal_notes, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "quote", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/quotes/{}", id)))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;

    sqlx::query!("UPDATE quotes SET status='cancelled', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "delete", "quote", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to("/quotes"))
}

pub async fn send(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;
    sqlx::query!("UPDATE quotes SET status='sent', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "send", "quote", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/quotes/{}", id)))
}

pub async fn accept(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;
    sqlx::query!("UPDATE quotes SET status='accepted', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "accept", "quote", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/quotes/{}", id)))
}

pub async fn reject(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(QUOTES_WRITE)?;
    sqlx::query!("UPDATE quotes SET status='rejected', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "reject", "quote", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/quotes/{}", id)))
}
