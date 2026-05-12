use crate::{auth::AuthUser, error::AppError, state::AppState};
use axum::{extract::State, response::IntoResponse};

pub async fn index(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    // Stats
    let customer_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM customers WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;

    let location_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM locations WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;

    let asset_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM assets WHERE status = 'active'")
            .fetch_one(&state.db)
            .await?;

    let open_jobs: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM service_jobs WHERE status NOT IN ('completed','cancelled')"
    )
    .fetch_one(&state.db)
    .await?;

    let open_changes: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM changes WHERE status NOT IN ('completed','closed','cancelled','rejected')"
    )
    .fetch_one(&state.db)
    .await?;

    let open_quotes: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM quotes WHERE status IN ('draft','sent')"
    )
    .fetch_one(&state.db)
    .await?;

    let open_invoices: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE status IN ('approved','sent','open','overdue')"
    )
    .fetch_one(&state.db)
    .await?;

    let overdue_invoices: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE status = 'overdue' OR
         (status IN ('approved','sent','open') AND due_date < date('now'))"
    )
    .fetch_one(&state.db)
    .await?;

    // Last backup
    let last_backup = sqlx::query_scalar!(
        "SELECT created_at FROM backup_history WHERE status = 'completed'
         ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await?;

    // Recent audit events
    let recent_events = sqlx::query!(
        "SELECT action, resource_type, resource_id, username, created_at
         FROM audit_log ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&state.db)
    .await?;

    let events: Vec<serde_json::Value> = recent_events
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "action": e.action,
                "resource_type": e.resource_type,
                "resource_id": e.resource_id,
                "username": e.username,
                "created_at": e.created_at,
            })
        })
        .collect();

    state.render(
        "dashboard.html",
        minijinja::context! {
            app_name => &state.config.app_name,
            user => &auth,
            stats => serde_json::json!({
                "customers": customer_count,
                "locations": location_count,
                "assets": asset_count,
                "open_jobs": open_jobs,
                "open_changes": open_changes,
                "open_quotes": open_quotes,
                "open_invoices": open_invoices,
                "overdue_invoices": overdue_invoices,
            }),
            last_backup => last_backup,
            recent_events => events,
        },
    )
}
