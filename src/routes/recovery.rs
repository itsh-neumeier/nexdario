use crate::{auth as auth_utils, error::AppError, state::AppState};
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RecoveryQuery {
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct RecoveryActionForm {
    pub token: String,
    pub action: String,
    pub new_password: Option<String>,
    pub username: Option<String>,
}

fn validate_recovery_token(state: &AppState, token: &str) -> bool {
    if !state.config.recovery_mode {
        return false;
    }
    match &state.config.recovery_token {
        Some(rt) => rt.len() >= 32 && rt == token,
        None => false,
    }
}

pub async fn index(
    State(state): State<AppState>,
    Query(q): Query<RecoveryQuery>,
) -> Result<impl IntoResponse, AppError> {
    if !state.config.recovery_mode {
        return Err(AppError::NotFound);
    }

    let token_valid = q.token.as_deref().map(|t| validate_recovery_token(&state, t)).unwrap_or(false);

    state.render("recovery/index.html", minijinja::context! {
        app_name => &state.config.app_name,
        recovery_mode => state.config.recovery_mode,
        token => q.token.as_deref().unwrap_or(""),
        token_valid => token_valid,
    })
}

pub async fn action(
    State(state): State<AppState>,
    Form(form): Form<RecoveryActionForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state.config.recovery_mode {
        return Err(AppError::NotFound);
    }

    if !validate_recovery_token(&state, &form.token) {
        return state.render("recovery/index.html", minijinja::context! {
            app_name => &state.config.app_name,
            recovery_mode => true,
            token => "",
            token_valid => false,
            error => "Ungültiger Recovery-Token",
        });
    }

    match form.action.as_str() {
        "reset_admin_password" => {
            let username = form.username.as_deref().unwrap_or("admin");
            let new_pw = form.new_password.as_deref()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::bad_request("Neues Passwort erforderlich"))?;

            if new_pw.len() < 8 {
                return Err(AppError::bad_request("Passwort muss mindestens 8 Zeichen haben"));
            }

            let hash = auth_utils::hash_password(new_pw)
                .map_err(|e| AppError::internal(e.to_string()))?;

            let rows = sqlx::query!(
                "UPDATE users SET password_hash=?, is_active=1 WHERE username=? COLLATE NOCASE",
                hash, username
            ).execute(&state.db).await?.rows_affected();

            if rows == 0 {
                // Create if not exists
                let role_id: Option<i64> = sqlx::query_scalar!(
                    "SELECT id FROM roles WHERE name='superadmin'"
                ).fetch_optional(&state.db).await?;

                let user_id = sqlx::query!(
                    "INSERT INTO users (username, email, display_name, password_hash, is_active, is_system)
                     VALUES (?,?,?,?,1,1)",
                    username, format!("{}@localhost", username), "Administrator", hash
                ).execute(&state.db).await?.last_insert_rowid();

                if let Some(rid) = role_id {
                    sqlx::query!(
                        "INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES (?,?)",
                        user_id, rid
                    ).execute(&state.db).await?;
                }

                sqlx::query!(
                    "INSERT INTO audit_log (action, resource_type, resource_id, details, success)
                     VALUES ('recovery_create_admin', 'user', ?, 'Recovery: Created admin user', 1)",
                    user_id.to_string()
                ).execute(&state.db).await.ok();
            } else {
                sqlx::query!(
                    "INSERT INTO audit_log (action, resource_type, details, success)
                     VALUES ('recovery_reset_password', 'user', 'Recovery: Password reset', 1)"
                ).execute(&state.db).await.ok();
            }

            state.render("recovery/success.html", minijinja::context! {
                app_name => &state.config.app_name,
                message => format!("Passwort für '{}' wurde zurückgesetzt. Bitte RECOVERY_MODE=false setzen.", username),
            })
        }
        "clear_sessions" => {
            sqlx::query!("DELETE FROM sessions").execute(&state.db).await?;

            sqlx::query!(
                "INSERT INTO audit_log (action, resource_type, details, success)
                 VALUES ('recovery_clear_sessions', 'session', 'Recovery: All sessions cleared', 1)"
            ).execute(&state.db).await.ok();

            state.render("recovery/success.html", minijinja::context! {
                app_name => &state.config.app_name,
                message => "Alle Sessions wurden gelöscht. Bitte RECOVERY_MODE=false setzen.",
            })
        }
        "disable_api_tokens" => {
            sqlx::query!("UPDATE api_tokens SET is_active=0").execute(&state.db).await?;

            state.render("recovery/success.html", minijinja::context! {
                app_name => &state.config.app_name,
                message => "Alle API-Tokens wurden deaktiviert.",
            })
        }
        _ => Err(AppError::bad_request("Unbekannte Aktion")),
    }
}
