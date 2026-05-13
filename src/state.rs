use crate::config::Config;
use axum::extract::FromRef;
use minijinja::Environment;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub templates: Arc<Environment<'static>>,
}

impl AppState {
    pub fn new(db: SqlitePool, config: Config, templates: Environment<'static>) -> Self {
        Self {
            db,
            config: Arc::new(config),
            templates: Arc::new(templates),
        }
    }

    pub fn render(
        &self,
        template_name: &str,
        ctx: impl serde::Serialize,
    ) -> Result<axum::response::Html<String>, crate::error::AppError> {
        let tmpl = self
            .templates
            .get_template(template_name)
            .map_err(|e| crate::error::AppError::internal(format!("Template error: {}", e)))?;

        let rendered = tmpl
            .render(ctx)
            .map_err(|e| crate::error::AppError::internal(format!("Render error: {}", e)))?;

        Ok(axum::response::Html(rendered))
    }
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(input: &AppState) -> Self {
        input.db.clone()
    }
}
