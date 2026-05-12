use crate::{auth, config::Config, permissions};
use anyhow::Context;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("Failed to connect to SQLite database")?;

    // Set pragmas
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(&pool).await?;
    sqlx::query("PRAGMA cache_size=10000").execute(&pool).await?;
    sqlx::query("PRAGMA temp_store=MEMORY").execute(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;
    Ok(())
}

pub async fn seed_initial_data(pool: &SqlitePool, config: &Config) -> anyhow::Result<()> {
    // Seed permissions
    seed_permissions(pool).await?;

    // Seed system roles
    seed_system_roles(pool).await?;

    // Create admin user if not exists
    create_initial_admin(pool, config).await?;

    // Seed number sequences
    seed_number_sequences(pool).await?;

    // Seed system settings
    seed_system_settings(pool, config).await?;

    Ok(())
}

async fn seed_permissions(pool: &SqlitePool) -> anyhow::Result<()> {
    for (name, display_name, module) in permissions::all_permissions() {
        sqlx::query!(
            "INSERT OR IGNORE INTO permissions (name, display_name, module)
             VALUES (?, ?, ?)",
            name,
            display_name,
            module
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn seed_system_roles(pool: &SqlitePool) -> anyhow::Result<()> {
    let system_roles = vec![
        ("superadmin", "Superadministrator", "Vollständige Systemkontrolle", 1000i64),
        ("admin", "Administrator", "Systemadministration ohne Recovery", 800),
        ("manager", "Manager", "Projektleitung und kaufmännische Verwaltung", 500),
        ("service", "Service-Techniker", "Mobile Service-Oberfläche", 100),
    ];

    for (name, display_name, description, rank) in &system_roles {
        sqlx::query!(
            "INSERT OR IGNORE INTO roles (name, display_name, description, rank, is_system, is_active)
             VALUES (?, ?, ?, ?, 1, 1)",
            name,
            display_name,
            description,
            rank
        )
        .execute(pool)
        .await?;
    }

    // Assign default permissions to roles
    assign_role_permissions(pool, "superadmin", &permissions::superadmin_permissions()).await?;
    assign_role_permissions(pool, "admin", &permissions::admin_permissions()).await?;
    assign_role_permissions(pool, "manager", &permissions::manager_permissions()).await?;
    assign_role_permissions(pool, "service", &permissions::service_permissions()).await?;

    Ok(())
}

async fn assign_role_permissions(
    pool: &SqlitePool,
    role_name: &str,
    perms: &[&str],
) -> anyhow::Result<()> {
    let role_id: Option<i64> = sqlx::query_scalar!(
        "SELECT id FROM roles WHERE name = ?",
        role_name
    )
    .fetch_optional(pool)
    .await?;

    let role_id = match role_id {
        Some(id) => id,
        None => return Ok(()),
    };

    for perm in perms {
        let perm_id: Option<i64> = sqlx::query_scalar!(
            "SELECT id FROM permissions WHERE name = ?",
            perm
        )
        .fetch_optional(pool)
        .await?;

        if let Some(pid) = perm_id {
            sqlx::query!(
                "INSERT OR IGNORE INTO role_permissions (role_id, permission_id) VALUES (?, ?)",
                role_id,
                pid
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn create_initial_admin(pool: &SqlitePool, config: &Config) -> anyhow::Result<()> {
    // Check if any superadmin exists
    let superadmin_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users u
         JOIN user_roles ur ON ur.user_id = u.id
         JOIN roles r ON r.id = ur.role_id
         WHERE r.name = 'superadmin' AND u.is_active = 1"
    )
    .fetch_one(pool)
    .await?;

    if superadmin_count > 0 {
        tracing::info!("Superadmin already exists, skipping initial admin creation");
        return Ok(());
    }

    // Check if any user exists
    let user_count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if user_count > 0 {
        tracing::warn!(
            "Users exist but no superadmin found. Use recovery mode to repair access."
        );
        return Ok(());
    }

    tracing::info!("Creating initial admin user: {}", config.admin_username);

    let hash = auth::hash_password(&config.admin_password)
        .context("Failed to hash admin password")?;

    let display_name = format!("Administrator");

    let user_id = sqlx::query!(
        "INSERT INTO users (username, email, display_name, password_hash, is_active, is_system)
         VALUES (?, ?, ?, ?, 1, 1)",
        config.admin_username,
        format!("{}@localhost", config.admin_username),
        display_name,
        hash
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    // Assign superadmin role
    let role_id: i64 = sqlx::query_scalar!("SELECT id FROM roles WHERE name = 'superadmin'")
        .fetch_one(pool)
        .await?;

    sqlx::query!(
        "INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)",
        user_id,
        role_id
    )
    .execute(pool)
    .await?;

    tracing::info!("Initial admin user created successfully");
    Ok(())
}

async fn seed_number_sequences(pool: &SqlitePool) -> anyhow::Result<()> {
    let year = chrono::Utc::now().year();

    let sequences = vec![
        ("customer", "KND", year, 4i64),
        ("employee", "MIT", year, 4),
        ("quote", "ANG", year, 4),
        ("order", "AB", year, 4),
        ("service_job", "SE", year, 4),
        ("service_report", "LSN", year, 4),
        ("invoice", "RE", year, 4),
        ("credit_note", "GS", year, 4),
        ("change", "CHG", year, 4),
        ("incoming_invoice", "ER", year, 4),
        ("export", "EXP", year, 4),
        ("catalog_item", "ART", year, 4),
    ];

    for (name, prefix, current_year, min_digits) in sequences {
        sqlx::query!(
            "INSERT OR IGNORE INTO number_sequences (name, prefix, current_year, last_number, min_digits)
             VALUES (?, ?, ?, 0, ?)",
            name,
            prefix,
            current_year,
            min_digits
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn seed_system_settings(pool: &SqlitePool, config: &Config) -> anyhow::Result<()> {
    let settings = vec![
        ("app_name", config.app_name.as_str(), "Application name"),
        ("app_base_url", config.app_base_url.as_str(), "Base URL"),
        ("invoice_tax_number", "", "Steuernummer des Unternehmens"),
        ("invoice_vat_id", "", "USt-ID des Unternehmens"),
        ("invoice_company_name", "", "Firmenname für Rechnungen"),
        ("invoice_street", "", "Straße für Rechnungen"),
        ("invoice_zip", "", "PLZ für Rechnungen"),
        ("invoice_city", "", "Ort für Rechnungen"),
        ("invoice_country", "DE", "Land für Rechnungen"),
        ("invoice_bank_name", "", "Bankname"),
        ("invoice_iban", "", "IBAN"),
        ("invoice_bic", "", "BIC"),
        ("invoice_default_tax_rate", "19", "Standard-Steuersatz (%)"),
        ("invoice_payment_terms", "14", "Standard-Zahlungsziel (Tage)"),
        ("xrechnung_version", "3.0.2", "XRechnung Version"),
        ("backup_enabled", if config.backup_enabled { "true" } else { "false" }, "Automatische Backups"),
        ("backup_interval_hours", &config.backup_interval_hours.to_string(), "Backup-Intervall Stunden"),
        ("change_require_for_critical", "true", "IT-Change für kritische Aktionen"),
    ];

    for (key, value, description) in settings {
        sqlx::query!(
            "INSERT OR IGNORE INTO system_settings (key, value, description) VALUES (?, ?, ?)",
            key,
            value,
            description
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn next_number(pool: &SqlitePool, sequence_name: &str) -> anyhow::Result<String> {
    let current_year = chrono::Utc::now().year() as i64;

    // Use a transaction to safely increment
    let mut tx = pool.begin().await?;

    let seq = sqlx::query!(
        "SELECT prefix, current_year, last_number, min_digits, include_year, separator
         FROM number_sequences WHERE name = ?",
        sequence_name
    )
    .fetch_optional(&mut *tx)
    .await?;

    let seq = seq.ok_or_else(|| anyhow::anyhow!("Unknown sequence: {}", sequence_name))?;

    let last_number = if seq.current_year != current_year {
        // New year, reset counter
        sqlx::query!(
            "UPDATE number_sequences SET current_year = ?, last_number = 1 WHERE name = ?",
            current_year,
            sequence_name
        )
        .execute(&mut *tx)
        .await?;
        1i64
    } else {
        let next = seq.last_number + 1;
        sqlx::query!(
            "UPDATE number_sequences SET last_number = ? WHERE name = ?",
            next,
            sequence_name
        )
        .execute(&mut *tx)
        .await?;
        next
    };

    tx.commit().await?;

    let min_digits = seq.min_digits as usize;
    let num_str = format!("{:0>width$}", last_number, width = min_digits);

    let number = if seq.include_year != 0 {
        format!("{}{}{}{}{}", seq.prefix, seq.separator, current_year, seq.separator, num_str)
    } else {
        format!("{}{}{}", seq.prefix, seq.separator, num_str)
    };

    Ok(number)
}

use chrono::Datelike;
