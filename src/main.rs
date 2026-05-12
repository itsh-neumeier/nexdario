use anyhow::Context;
use minijinja::{path_loader, Environment};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod config;
mod db;
mod error;
mod jobs;
mod permissions;
mod routes;
mod services;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nexdario=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env().context("Failed to load configuration")?;

    tracing::info!("Starting {} v{}", config.app_name, env!("CARGO_PKG_VERSION"));
    tracing::info!("Database: {}", config.database_url);
    tracing::info!("Listening on: {}", config.bind_addr());

    if config.recovery_mode {
        tracing::warn!("⚠️  RECOVERY MODE ACTIVE — Disable after use!");
    }

    // Ensure data directories exist
    tokio::fs::create_dir_all(&config.backup_dir).await.ok();
    tokio::fs::create_dir_all(&config.export_dir).await.ok();

    // Connect to database
    let pool = db::create_pool(&config.database_url)
        .await
        .context("Failed to create database pool")?;

    // Run migrations
    db::run_migrations(&pool)
        .await
        .context("Failed to run migrations")?;

    // Seed initial data (only if first run)
    db::seed_initial_data(&pool, &config)
        .await
        .context("Failed to seed initial data")?;

    // Setup templates
    let templates = setup_templates(&config.templates_dir)?;

    // Create app state
    let app_state = state::AppState::new(pool.clone(), config.clone(), templates);

    // Start background jobs
    jobs::spawn_background_jobs(pool, Arc::new(config.clone()));

    // Build router
    let router = routes::create_router(app_state)
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO)),
        );

    // Start server
    let bind_addr = config.bind_addr();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .context(format!("Failed to bind to {}", bind_addr))?;

    tracing::info!("Server started at http://{}", bind_addr);
    tracing::info!("Admin login: {}", config.admin_username);

    axum::serve(listener, router.into_make_service())
        .await
        .context("Server error")?;

    Ok(())
}

fn setup_templates(templates_dir: &str) -> anyhow::Result<Environment<'static>> {
    let mut env = Environment::new();
    env.set_loader(path_loader(templates_dir));

    // Add custom filters
    env.add_filter("format_money", |value: minijinja::Value| -> String {
        let n = value.as_f64().unwrap_or(0.0);
        format!("{:.2}", n).replace('.', ",")
    });

    env.add_filter("format_date", |value: minijinja::Value| -> String {
        let s = value.to_string();
        if s.len() >= 10 {
            let parts: Vec<&str> = s[..10].split('-').collect();
            if parts.len() == 3 {
                return format!("{}.{}.{}", parts[2], parts[1], parts[0]);
            }
        }
        s
    });

    env.add_filter("truncate", |value: minijinja::Value, length: usize| -> String {
        let s = value.to_string();
        if s.len() > length {
            format!("{}…", &s[..length.min(s.len())])
        } else {
            s
        }
    });

    Ok(env)
}
