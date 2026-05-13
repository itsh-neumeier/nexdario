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
    auth.require_permission(INVOICES_READ)?;

    let search = q.search.as_deref().unwrap_or("");
    let like = format!("%{}%", search);

    let invoices = sqlx::query!(
        "SELECT i.id, i.invoice_number, i.customer_name, i.status, i.invoice_type,
         i.total, i.amount_paid, i.invoice_date, i.due_date, i.customer_id
         FROM invoices i
         WHERE i.invoice_number LIKE ? OR i.customer_name LIKE ?
         ORDER BY i.invoice_date DESC LIMIT 200",
        like, like
    ).fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = invoices.into_iter().map(|i| serde_json::json!({
        "id": i.id, "invoice_number": i.invoice_number, "customer_name": i.customer_name,
        "status": i.status, "invoice_type": i.invoice_type,
        "total": i.total, "amount_paid": i.amount_paid, "invoice_date": i.invoice_date,
        "due_date": i.due_date, "balance": i.total - i.amount_paid,
    })).collect();

    state.render("invoices/list.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        invoices => items,
        search => search,
        can_write => auth.has_permission(INVOICES_WRITE),
        can_approve => auth.has_permission(INVOICES_APPROVE),
    })
}

pub async fn detail(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_READ)?;

    let invoice = sqlx::query!("SELECT * FROM invoices WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    let items = sqlx::query!("SELECT * FROM invoice_items WHERE invoice_id=? ORDER BY position", id)
        .fetch_all(&state.db).await?;

    let item_list: Vec<serde_json::Value> = items.into_iter().map(|i| serde_json::json!({
        "id": i.id, "position": i.position, "name": i.name, "description": i.description,
        "quantity": i.quantity, "unit": i.unit, "unit_price": i.unit_price,
        "tax_rate": i.tax_rate, "subtotal": i.subtotal,
    })).collect();

    state.render("invoices/detail.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        invoice => serde_json::json!({
            "id": invoice.id, "invoice_number": invoice.invoice_number,
            "customer_id": invoice.customer_id, "customer_name": invoice.customer_name,
            "customer_street": invoice.customer_street, "customer_zip": invoice.customer_zip,
            "customer_city": invoice.customer_city, "customer_country": invoice.customer_country,
            "customer_vat_id": invoice.customer_vat_id,
            "status": invoice.status, "invoice_type": invoice.invoice_type,
            "invoice_date": invoice.invoice_date, "delivery_date": invoice.delivery_date,
            "due_date": invoice.due_date, "payment_terms": invoice.payment_terms,
            "our_company_name": invoice.our_company_name, "our_street": invoice.our_street,
            "our_zip": invoice.our_zip, "our_city": invoice.our_city,
            "our_vat_id": invoice.our_vat_id, "our_iban": invoice.our_iban,
            "subtotal": invoice.subtotal, "discount_amount": invoice.discount_amount,
            "tax_amount": invoice.tax_amount, "total": invoice.total,
            "amount_paid": invoice.amount_paid, "balance": invoice.total - invoice.amount_paid,
            "notes": invoice.notes, "leitweg_id": invoice.leitweg_id,
        }),
        items => item_list,
        can_approve => auth.has_permission(INVOICES_APPROVE),
        can_cancel => auth.has_permission(INVOICES_CANCEL),
        can_xrechnung => auth.has_permission(XRECHNUNG_EXPORT),
    })
}

#[derive(Deserialize)]
pub struct InvoiceForm {
    pub customer_id: Option<i64>,
    pub invoice_date: String,
    pub delivery_date: Option<String>,
    pub due_date: Option<String>,
    pub invoice_type: Option<String>,
    pub payment_terms: Option<String>,
    pub notes: Option<String>,
    pub leitweg_id: Option<String>,
    pub buyer_reference: Option<String>,
}

pub async fn new_form(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_WRITE)?;

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    // Load company settings
    let company_name = get_setting(&state, "invoice_company_name").await.unwrap_or_default();
    let vat_id = get_setting(&state, "invoice_vat_id").await.unwrap_or_default();
    let iban = get_setting(&state, "invoice_iban").await.unwrap_or_default();

    state.render("invoices/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        invoice => Option::<serde_json::Value>::None,
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        company_name => company_name,
        vat_id => vat_id,
        iban => iban,
        today => chrono::Utc::now().format("%Y-%m-%d").to_string(),
        title => "Neue Rechnung",
        action => "/invoices/new",
    })
}

async fn get_setting(state: &AppState, key: &str) -> Option<String> {
    sqlx::query_scalar!("SELECT value FROM system_settings WHERE key=?", key)
        .fetch_optional(&state.db).await.ok().flatten()
}

pub async fn create(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<InvoiceForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_WRITE)?;

    let customer_id = form.customer_id.ok_or_else(|| AppError::bad_request("Kunde ist erforderlich"))?;
    // Get customer data for snapshot
    let customer = sqlx::query!(
        "SELECT name, billing_street, billing_zip, billing_city, billing_country, vat_id
         FROM customers WHERE id=?", customer_id
    ).fetch_optional(&state.db).await?.ok_or_else(|| AppError::bad_request("Kunde nicht gefunden"))?;

    let number = db::next_number(&state.db, "invoice").await?;
    let invoice_type = form.invoice_type.as_deref().unwrap_or("standard");

    // Calc due date from payment terms if not set
    let payment_terms_days: i64 = get_setting(&state, "invoice_payment_terms").await
        .and_then(|v| v.parse().ok()).unwrap_or(14);

    let due_date = form.due_date.clone().or_else(|| {
        chrono::NaiveDate::parse_from_str(&form.invoice_date, "%Y-%m-%d").ok()
            .and_then(|d| d.checked_add_signed(chrono::Duration::days(payment_terms_days)))
            .map(|d| d.format("%Y-%m-%d").to_string())
    });

    // Company data from settings
    let our_company = get_setting(&state, "invoice_company_name").await;
    let our_vat_id = get_setting(&state, "invoice_vat_id").await;
    let our_iban = get_setting(&state, "invoice_iban").await;
    let our_bic = get_setting(&state, "invoice_bic").await;
    let our_street = get_setting(&state, "invoice_street").await;
    let our_zip = get_setting(&state, "invoice_zip").await;
    let our_city = get_setting(&state, "invoice_city").await;

    let payment_terms_days_str = payment_terms_days.to_string();
    let payment_terms_val = form.payment_terms.as_deref().unwrap_or(&payment_terms_days_str);
    let billing_country = customer.billing_country.as_str();

    let id = sqlx::query!(
        "INSERT INTO invoices (invoice_number, customer_id, invoice_type, status, invoice_date,
         delivery_date, due_date, payment_terms, our_company_name, our_vat_id, our_iban, our_bic,
         our_street, our_zip, our_city,
         customer_name, customer_street, customer_zip, customer_city, customer_country,
         customer_vat_id, leitweg_id, buyer_reference, notes, created_by)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        number, customer_id, invoice_type, "draft", form.invoice_date,
        form.delivery_date, due_date, payment_terms_val,
        our_company, our_vat_id, our_iban, our_bic, our_street, our_zip, our_city,
        customer.name, customer.billing_street, customer.billing_zip,
        customer.billing_city, billing_country,
        customer.vat_id, form.leitweg_id, form.buyer_reference, form.notes, auth.id
    ).execute(&state.db).await?.last_insert_rowid();

    audit::log(&state.db, Some(&auth), "create", "invoice", Some(&id.to_string()),
        Some(&format!("Created invoice: {}", number)), None, true).await;

    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn edit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_WRITE)?;

    let invoice = sqlx::query!("SELECT * FROM invoices WHERE id=?", id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    if invoice.status != "draft" && !auth.is_superadmin {
        return Err(AppError::bad_request("Freigegebene Rechnungen können nicht mehr bearbeitet werden"));
    }

    let customers = sqlx::query!("SELECT id, name FROM customers WHERE status='active' ORDER BY name LIMIT 200")
        .fetch_all(&state.db).await?;

    state.render("invoices/form.html", minijinja::context! {
        app_name => &state.config.app_name,
        user => &auth,
        invoice => serde_json::json!({
            "id": invoice.id, "customer_id": invoice.customer_id, "invoice_type": invoice.invoice_type,
            "invoice_date": invoice.invoice_date, "delivery_date": invoice.delivery_date,
            "due_date": invoice.due_date, "payment_terms": invoice.payment_terms,
            "notes": invoice.notes, "leitweg_id": invoice.leitweg_id,
            "buyer_reference": invoice.buyer_reference,
        }),
        customers => customers.into_iter().map(|c| serde_json::json!({"id": c.id, "name": c.name})).collect::<Vec<_>>(),
        title => "Rechnung bearbeiten",
        action => format!("/invoices/{}/edit", id),
    })
}

pub async fn update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<InvoiceForm>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_WRITE)?;

    sqlx::query!(
        "UPDATE invoices SET invoice_date=?, delivery_date=?, due_date=?,
         payment_terms=?, notes=?, leitweg_id=?, buyer_reference=?,
         updated_at=datetime('now') WHERE id=? AND status='draft'",
        form.invoice_date, form.delivery_date, form.due_date,
        form.payment_terms, form.notes, form.leitweg_id, form.buyer_reference, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "update", "invoice", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn approve(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_APPROVE)?;

    sqlx::query!(
        "UPDATE invoices SET status='approved', approved_by=?, approved_at=datetime('now'),
         updated_at=datetime('now') WHERE id=? AND status='draft'",
        auth.id, id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "approve", "invoice", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn cancel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_CANCEL)?;

    sqlx::query!(
        "UPDATE invoices SET status='cancelled', cancelled_at=datetime('now'),
         updated_at=datetime('now') WHERE id=?",
        id
    ).execute(&state.db).await?;

    audit::log(&state.db, Some(&auth), "cancel", "invoice", Some(&id.to_string()), None, None, true).await;

    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn send(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_APPROVE)?;
    sqlx::query!(
        "UPDATE invoices SET status='sent', sent_at=datetime('now'), updated_at=datetime('now') WHERE id=?",
        id
    ).execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "send", "invoice", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn mark_paid(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(INVOICES_APPROVE)?;
    sqlx::query!(
        "UPDATE invoices SET status='paid', paid_at=datetime('now'), updated_at=datetime('now') WHERE id=?",
        id
    ).execute(&state.db).await?;
    audit::log(&state.db, Some(&auth), "mark_paid", "invoice", Some(&id.to_string()), None, None, true).await;
    Ok(Redirect::to(&format!("/invoices/{}", id)))
}

pub async fn export_xrechnung(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    auth.require_permission(XRECHNUNG_EXPORT)?;

    let invoice = sqlx::query("SELECT * FROM invoices WHERE id=?")
        .bind(id)
        .fetch_optional(&state.db).await?.ok_or(AppError::NotFound)?;

    {
        use sqlx::Row;
        let status: String = invoice.try_get("status").unwrap_or_default();
        if status == "draft" {
            return Err(AppError::bad_request("Entwürfe können nicht exportiert werden"));
        }
    }

    let items = sqlx::query("SELECT * FROM invoice_items WHERE invoice_id=? ORDER BY position")
        .bind(id)
        .fetch_all(&state.db).await?;

    let xml = build_xrechnung_xml(&invoice, &items)?;

    let id_str = id.to_string();
    audit::log(&state.db, Some(&auth), "export_xrechnung", "invoice", Some(&id_str), None, None, true).await;

    use axum::http::header;
    use sqlx::Row;
    let invoice_number: String = invoice.try_get("invoice_number").unwrap_or_default();
    let filename = format!("XRechnung_{}.xml", invoice_number);
    let content_disposition = format!("attachment; filename=\"{}\"", filename);
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        xml,
    ))
}

fn build_xrechnung_xml(invoice: &sqlx::sqlite::SqliteRow, _items: &[sqlx::sqlite::SqliteRow]) -> Result<String, AppError> {
    use sqlx::Row;
    let invoice_number: String = invoice.try_get("invoice_number").unwrap_or_default();
    let invoice_date: String = invoice.try_get("invoice_date").unwrap_or_default();
    let total: f64 = invoice.try_get("total").unwrap_or(0.0);
    let customer_name: String = invoice.try_get("customer_name").unwrap_or_default();

    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<ubl:Invoice xmlns:ubl="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
  xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
  xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_3.0</cbc:CustomizationID>
  <cbc:ProfileID>urn:fdc:peppol.eu:2017:poacc:billing:01:1.0</cbc:ProfileID>
  <cbc:ID>{}</cbc:ID>
  <cbc:IssueDate>{}</cbc:IssueDate>
  <cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>
  <cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName>
    </cac:Party>
  </cac:AccountingCustomerParty>
  <cac:LegalMonetaryTotal>
    <cbc:TaxInclusiveAmount currencyID="EUR">{:.2}</cbc:TaxInclusiveAmount>
    <cbc:PayableAmount currencyID="EUR">{:.2}</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
</ubl:Invoice>"#,
        invoice_number, invoice_date,
        xml_escape(&customer_name),
        total, total
    );

    Ok(xml)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
