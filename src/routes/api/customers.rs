use crate::{auth::AuthUser, db, error::AppError, permissions::*, state::AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}

pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_permission(CUSTOMERS_READ)?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let search = params.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);
    let status = params.status.as_deref().unwrap_or("active");

    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM customers WHERE status=? AND (name LIKE ? OR customer_number LIKE ?)",
        status, like, like
    ).fetch_one(&state.db).await? as i64;

    let customers = sqlx::query!(
        "SELECT id, customer_number, name, customer_type, status, industry, phone, email,
         billing_city, billing_country
         FROM customers WHERE status=? AND (name LIKE ? OR customer_number LIKE ?)
         ORDER BY name LIMIT ? OFFSET ?",
        status, like, like, per_page, offset
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = customers.into_iter().map(|c| serde_json::json!({
        "id": c.id,
        "customer_number": c.customer_number,
        "name": c.name,
        "customer_type": c.customer_type,
        "status": c.status,
        "industry": c.industry,
        "phone": c.phone,
        "email": c.email,
        "billing_city": c.billing_city,
        "billing_country": c.billing_country,
    })).collect();

    Ok(Json(serde_json::json!({
        "data": items,
        "total": total,
        "page": page,
        "per_page": per_page,
    })))
}

pub async fn get_one(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_permission(CUSTOMERS_READ)?;

    let c = sqlx::query!(
        "SELECT id, customer_number, name, customer_type, status, industry, website,
         phone, email, billing_street, billing_zip, billing_city, billing_country,
         vat_id, payment_terms, notes, created_at, updated_at
         FROM customers WHERE id=?", id
    ).fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    Ok(Json(serde_json::json!({
        "id": c.id, "customer_number": c.customer_number, "name": c.name,
        "customer_type": c.customer_type, "status": c.status, "industry": c.industry,
        "website": c.website, "phone": c.phone, "email": c.email,
        "billing_street": c.billing_street, "billing_zip": c.billing_zip,
        "billing_city": c.billing_city, "billing_country": c.billing_country,
        "vat_id": c.vat_id, "payment_terms": c.payment_terms, "notes": c.notes,
        "created_at": c.created_at, "updated_at": c.updated_at,
    })))
}

#[derive(Deserialize)]
pub struct CreateCustomerBody {
    pub name: String,
    pub customer_type: Option<String>,
    pub industry: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub billing_city: Option<String>,
    pub billing_country: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateCustomerBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    if body.name.trim().is_empty() {
        return Err(AppError::bad_request("name is required"));
    }

    let number = db::next_number(&state.db, "customer").await?;
    let customer_type = body.customer_type.as_deref().unwrap_or("business");
    let billing_country = body.billing_country.as_deref().unwrap_or("DE");
    let payment_terms = body.payment_terms.as_deref().unwrap_or("14");

    let id = sqlx::query!(
        "INSERT INTO customers (customer_number, name, customer_type, status, industry, phone, email,
         billing_city, billing_country, payment_terms, notes)
         VALUES (?,?,?,'active',?,?,?,?,?,?,?)",
        number, body.name, customer_type, body.industry, body.phone, body.email,
        body.billing_city, billing_country, payment_terms, body.notes
    ).execute(&state.db).await?.last_insert_rowid();

    Ok(Json(serde_json::json!({
        "id": id,
        "customer_number": number,
        "name": body.name,
    })))
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_permission(CUSTOMERS_WRITE)?;

    let _existing = sqlx::query!("SELECT id FROM customers WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        sqlx::query!("UPDATE customers SET name=?, updated_at=datetime('now') WHERE id=?", name, id)
            .execute(&state.db).await?;
    }

    Ok(Json(serde_json::json!({"id": id, "updated": true})))
}

pub async fn delete(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_permission(CUSTOMERS_DELETE)?;

    sqlx::query!("UPDATE customers SET status='deleted', updated_at=datetime('now') WHERE id=?", id)
        .execute(&state.db).await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}
