use crate::{
    auth as auth_utils,
    error::AppError,
    services::audit,
    state::AppState,
};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub redirect_to: Option<String>,
}

pub async fn login_page(
    State(state): State<AppState>,
    OptUser(user): OptUser,
) -> impl IntoResponse {
    if user.is_some() {
        return Redirect::to("/").into_response();
    }

    state
        .render(
            "login.html",
            minijinja::context! {
                app_name => state.config.app_name,
                error => Option::<String>::None,
            },
        )
        .map(|h| h.into_response())
        .unwrap_or_else(|e| e.into_response())
}

pub async fn login(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Form(form): Form<LoginForm>,
) -> Response {
    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let ua = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Find user
    let user_row = sqlx::query!(
        "SELECT id, username, password_hash, is_active FROM users
         WHERE username = ? COLLATE NOCASE LIMIT 1",
        form.username
    )
    .fetch_optional(&state.db)
    .await;

    match user_row {
        Ok(Some(row)) if row.is_active != 0 => {
            if auth_utils::verify_password(&form.password, &row.password_hash) {
                // Create session
                let token = match auth_utils::create_session(
                    &state.db,
                    row.id,
                    ip.as_deref(),
                    ua.as_deref(),
                    8,
                )
                .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!("Session creation failed: {}", e);
                        return render_login_error(&state, "Interner Fehler beim Login");
                    }
                };

                // Update last_login_at
                sqlx::query!(
                    "UPDATE users SET last_login_at = datetime('now') WHERE id = ?",
                    row.id
                )
                .execute(&state.db)
                .await
                .ok();

                audit::log_login(&state.db, row.id, &row.username, ip.as_deref(), true).await;

                let redirect_to = form
                    .redirect_to
                    .as_deref()
                    .filter(|s| s.starts_with('/'))
                    .unwrap_or("/");

                let cookie = auth_utils::session_cookie(&token, false);

                Response::builder()
                    .status(StatusCode::SEE_OTHER)
                    .header(header::LOCATION, redirect_to)
                    .header(header::SET_COOKIE, cookie)
                    .body(axum::body::Body::empty())
                    .unwrap()
            } else {
                // Log failed attempt
                audit::log_login(&state.db, row.id, &row.username, ip.as_deref(), false).await;
                render_login_error(&state, "Ungültige Zugangsdaten")
            }
        }
        Ok(Some(_)) => render_login_error(&state, "Benutzerkonto deaktiviert"),
        Ok(None) => render_login_error(&state, "Ungültige Zugangsdaten"),
        Err(e) => {
            tracing::error!("Login DB error: {}", e);
            render_login_error(&state, "Datenbankfehler")
        }
    }
}

fn render_login_error(state: &AppState, error: &str) -> Response {
    state
        .render(
            "login.html",
            minijinja::context! {
                app_name => state.config.app_name,
                error => error,
            },
        )
        .map(|h| h.into_response())
        .unwrap_or_else(|e| e.into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';').find_map(|part| {
                part.trim().strip_prefix("nxd_session=").map(|t| t.to_string())
            })
        });

    if let Some(token) = token {
        auth_utils::delete_session(&state.db, &token).await.ok();
    }

    let clear_cookie = auth_utils::clear_session_cookie();

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, "/login")
        .header(header::SET_COOKIE, clear_cookie)
        .body(axum::body::Body::empty())
        .unwrap()
}

// Helper extractor wrapper
struct OptUser(Option<crate::auth::AuthUser>);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for OptUser
where
    AppState: axum::extract::FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let opt = crate::auth::OptionalAuthUser::from_request_parts(parts, state).await?;
        Ok(OptUser(opt.0))
    }
}
