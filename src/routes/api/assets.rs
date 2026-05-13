use crate::{auth::AuthUser, error::AppError, services::naming, state::AppState};
use axum::{extract::{Query, State}, response::IntoResponse, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SuggestQuery {
    pub location_id: Option<i64>,
    pub device_type: Option<String>,
    pub role: Option<String>,
}

pub async fn suggest_hostname(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<SuggestQuery>,
) -> Result<impl IntoResponse, AppError> {
    let device_type = q.device_type.as_deref().unwrap_or("SRV");
    let role = q.role.as_deref().unwrap_or("INFRA");

    let site_code = if let Some(loc_id) = q.location_id {
        sqlx::query_scalar!("SELECT site_code FROM locations WHERE id=?", loc_id)
            .fetch_optional(&state.db).await?
            .unwrap_or_else(|| "SITE".to_string())
    } else {
        "SITE".to_string()
    };

    let hostname = naming::generate_hostname(&state.db, &site_code, device_type, role).await
        .unwrap_or_else(|_| format!("{}-{}-{}-01", site_code, device_type, role));

    Ok(Json(serde_json::json!({ "hostname": hostname })))
}
