use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "nexdario",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn info(State(state): State<AppState>) -> impl IntoResponse {
    let user_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE is_active=1")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Json(json!({
        "app_name": state.config.app_name,
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok",
        "user_count": user_count,
    }))
}
