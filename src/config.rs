use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub app_name: String,
    pub app_host: String,
    pub app_port: u16,
    pub app_base_url: String,
    pub database_url: String,
    pub data_dir: String,
    pub backup_dir: String,
    pub export_dir: String,
    pub templates_dir: String,
    pub static_dir: String,

    pub admin_username: String,
    pub admin_password: String,

    pub app_secret_key: String,
    pub data_encryption_key: String,

    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
    pub backup_retention_local_days: u64,
    pub backup_encryption_enabled: bool,
    pub backup_encryption_key: String,

    pub s3_enabled: bool,
    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: Option<String>,
    pub s3_prefix: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_path_style: bool,
    pub s3_retention_days: u64,

    pub recovery_mode: bool,
    pub recovery_token: Option<String>,

    pub demo_data: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            app_name: env::var("APP_NAME").unwrap_or_else(|_| "Nexdario".to_string()),
            app_host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            app_port: env::var("APP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            app_base_url: env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:/data/nexdario.sqlite".to_string()),
            data_dir: env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string()),
            backup_dir: env::var("BACKUP_DIR").unwrap_or_else(|_| "/data/backups".to_string()),
            export_dir: env::var("EXPORT_DIR").unwrap_or_else(|_| "/data/exports".to_string()),
            templates_dir: env::var("TEMPLATES_DIR").unwrap_or_else(|_| "templates".to_string()),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string()),

            admin_username: env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "changeme".to_string()),

            app_secret_key: env::var("APP_SECRET_KEY")
                .unwrap_or_else(|_| "please-change-this-secret-key-in-production".to_string()),
            data_encryption_key: env::var("DATA_ENCRYPTION_KEY")
                .unwrap_or_else(|_| "please-change-this-encryption-key-in-production".to_string()),

            backup_enabled: env::var("BACKUP_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
            backup_interval_hours: env::var("BACKUP_INTERVAL_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            backup_retention_local_days: env::var("BACKUP_RETENTION_LOCAL_DAYS")
                .unwrap_or_else(|_| "14".to_string())
                .parse()
                .unwrap_or(14),
            backup_encryption_enabled: env::var("BACKUP_ENCRYPTION_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
            backup_encryption_key: env::var("BACKUP_ENCRYPTION_KEY")
                .unwrap_or_else(|_| "please-change-this-backup-key".to_string()),

            s3_enabled: env::var("S3_ENABLED").map(|v| v == "true").unwrap_or(false),
            s3_endpoint: env::var("S3_ENDPOINT").ok().filter(|s| !s.is_empty()),
            s3_region: env::var("S3_REGION").unwrap_or_else(|_| "eu-central-1".to_string()),
            s3_bucket: env::var("S3_BUCKET").ok().filter(|s| !s.is_empty()),
            s3_prefix: env::var("S3_PREFIX").unwrap_or_else(|_| "nexdario/".to_string()),
            s3_access_key: env::var("S3_ACCESS_KEY").ok().filter(|s| !s.is_empty()),
            s3_secret_key: env::var("S3_SECRET_KEY").ok().filter(|s| !s.is_empty()),
            s3_path_style: env::var("S3_PATH_STYLE")
                .map(|v| v == "true")
                .unwrap_or(true),
            s3_retention_days: env::var("S3_RETENTION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),

            recovery_mode: env::var("RECOVERY_MODE")
                .map(|v| v == "true")
                .unwrap_or(false),
            recovery_token: env::var("RECOVERY_TOKEN").ok().filter(|s| !s.is_empty()),

            demo_data: env::var("DEMO_DATA").map(|v| v == "true").unwrap_or(false),
        })
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.app_host, self.app_port)
    }
}
