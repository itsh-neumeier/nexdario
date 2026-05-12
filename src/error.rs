use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Not found")]
    NotFound,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Sqlx(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            AppError::Internal(m) => {
                tracing::error!("Internal error: {}", m);
                (StatusCode::INTERNAL_SERVER_ERROR, m.clone())
            }
            AppError::Anyhow(e) => {
                tracing::error!("Error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Html(format!(
            r#"<!DOCTYPE html>
<html lang="de">
<head><meta charset="UTF-8"><title>Fehler {}</title>
<style>body{{font-family:system-ui;max-width:600px;margin:4rem auto;padding:1rem}}
.error{{background:#fee2e2;border:1px solid #fca5a5;border-radius:8px;padding:1.5rem}}
h1{{color:#dc2626}}a{{color:#2563eb}}</style></head>
<body><div class="error"><h1>Fehler {}</h1><p>{}</p></div>
<p><a href="/">Zurück zur Startseite</a></p></body></html>"#,
            status.as_u16(),
            status.as_u16(),
            message
        ));

        (status, body).into_response()
    }
}

impl AppError {
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }
}
