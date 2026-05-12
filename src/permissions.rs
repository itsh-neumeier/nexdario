// Permission constants — format: module:action
pub const CUSTOMERS_READ: &str = "customers:read";
pub const CUSTOMERS_WRITE: &str = "customers:write";
pub const CUSTOMERS_DELETE: &str = "customers:delete";

pub const CONTACTS_READ: &str = "contacts:read";
pub const CONTACTS_WRITE: &str = "contacts:write";
pub const CONTACTS_DELETE: &str = "contacts:delete";
pub const CONTACTS_PRIVACY_MANAGE: &str = "contacts:privacy_manage";

pub const LOCATIONS_READ: &str = "locations:read";
pub const LOCATIONS_WRITE: &str = "locations:write";
pub const LOCATIONS_DELETE: &str = "locations:delete";

pub const ASSETS_READ: &str = "assets:read";
pub const ASSETS_WRITE: &str = "assets:write";
pub const ASSETS_DELETE: &str = "assets:delete";

pub const DOCUMENTS_READ: &str = "documents:read";
pub const DOCUMENTS_WRITE: &str = "documents:write";
pub const DOCUMENTS_DOWNLOAD: &str = "documents:download";
pub const DOCUMENTS_DELETE: &str = "documents:delete";

pub const SECRETS_PRESENCE_READ: &str = "secrets:presence_read";
pub const SECRETS_REVEAL: &str = "secrets:reveal";
pub const SECRETS_WRITE: &str = "secrets:write";

pub const SECRET_ACCESS_CREATE: &str = "secret_access:create";
pub const SECRET_ACCESS_REVOKE: &str = "secret_access:revoke";
pub const SECRET_ACCESS_READ: &str = "secret_access:read";

pub const WAN_READ: &str = "wan:read";
pub const WAN_WRITE: &str = "wan:write";
pub const WAN_DELETE: &str = "wan:delete";

pub const QUOTES_READ: &str = "quotes:read";
pub const QUOTES_WRITE: &str = "quotes:write";
pub const QUOTES_APPROVE: &str = "quotes:approve";

pub const ORDERS_READ: &str = "orders:read";
pub const ORDERS_WRITE: &str = "orders:write";

pub const SERVICE_JOBS_READ: &str = "service_jobs:read";
pub const SERVICE_JOBS_WRITE: &str = "service_jobs:write";

pub const DISPATCH_READ: &str = "dispatch:read";
pub const DISPATCH_WRITE: &str = "dispatch:write";

pub const TIME_ENTRIES_READ: &str = "time_entries:read";
pub const TIME_ENTRIES_WRITE: &str = "time_entries:write";
pub const TIME_ENTRIES_APPROVE: &str = "time_entries:approve";

pub const SERVICE_REPORTS_READ: &str = "service_reports:read";
pub const SERVICE_REPORTS_WRITE: &str = "service_reports:write";
pub const SERVICE_REPORTS_APPROVE: &str = "service_reports:approve";

pub const INVOICES_READ: &str = "invoices:read";
pub const INVOICES_WRITE: &str = "invoices:write";
pub const INVOICES_APPROVE: &str = "invoices:approve";
pub const INVOICES_CANCEL: &str = "invoices:cancel";
pub const XRECHNUNG_EXPORT: &str = "xrechnung:export";

pub const ACCOUNTING_READ: &str = "accounting:read";
pub const ACCOUNTING_WRITE: &str = "accounting:write";
pub const ACCOUNTING_EXPORT: &str = "accounting:export";
pub const DATEV_EXPORT: &str = "datev:export";

pub const PURCHASES_READ: &str = "purchases:read";
pub const PURCHASES_WRITE: &str = "purchases:write";
pub const OPEN_ITEMS_READ: &str = "open_items:read";
pub const OPEN_ITEMS_WRITE: &str = "open_items:write";

pub const CHANGES_READ: &str = "changes:read";
pub const CHANGES_WRITE: &str = "changes:write";
pub const CHANGES_APPROVE: &str = "changes:approve";
pub const CHANGES_CLOSE: &str = "changes:close";

pub const BACKUP_READ: &str = "backup:read";
pub const BACKUP_CREATE: &str = "backup:create";
pub const BACKUP_DOWNLOAD: &str = "backup:download";
pub const BACKUP_RESTORE: &str = "backup:restore";

pub const S3_READ: &str = "s3:read";
pub const S3_WRITE: &str = "s3:write";

pub const SMTP_READ: &str = "smtp:read";
pub const SMTP_WRITE: &str = "smtp:write";

pub const USERS_READ: &str = "users:read";
pub const USERS_WRITE: &str = "users:write";
pub const USERS_DELETE: &str = "users:delete";

pub const ROLES_READ: &str = "roles:read";
pub const ROLES_WRITE: &str = "roles:write";
pub const ROLES_DELETE: &str = "roles:delete";

pub const API_TOKENS_READ: &str = "api_tokens:read";
pub const API_TOKENS_WRITE: &str = "api_tokens:write";

pub const AUDIT_READ: &str = "audit:read";

pub const SYSTEM_READ: &str = "system:read";
pub const SYSTEM_WRITE: &str = "system:write";

pub const RECOVERY_USE: &str = "recovery:use";

pub const EMPLOYEES_READ: &str = "employees:read";
pub const EMPLOYEES_WRITE: &str = "employees:write";

// All permissions list for seeding
pub fn all_permissions() -> Vec<(&'static str, &'static str, &'static str)> {
    // (name, display_name, module)
    vec![
        (CUSTOMERS_READ, "Kunden lesen", "Kunden"),
        (CUSTOMERS_WRITE, "Kunden schreiben", "Kunden"),
        (CUSTOMERS_DELETE, "Kunden löschen", "Kunden"),
        (CONTACTS_READ, "Kontakte lesen", "Kontakte"),
        (CONTACTS_WRITE, "Kontakte schreiben", "Kontakte"),
        (CONTACTS_DELETE, "Kontakte löschen", "Kontakte"),
        (CONTACTS_PRIVACY_MANAGE, "Kontakt-Sichtbarkeit verwalten", "Kontakte"),
        (LOCATIONS_READ, "Standorte lesen", "Standorte"),
        (LOCATIONS_WRITE, "Standorte schreiben", "Standorte"),
        (LOCATIONS_DELETE, "Standorte löschen", "Standorte"),
        (ASSETS_READ, "Assets lesen", "Assets"),
        (ASSETS_WRITE, "Assets schreiben", "Assets"),
        (ASSETS_DELETE, "Assets löschen", "Assets"),
        (DOCUMENTS_READ, "Dokumente lesen", "Dokumente"),
        (DOCUMENTS_WRITE, "Dokumente hochladen", "Dokumente"),
        (DOCUMENTS_DOWNLOAD, "Dokumente herunterladen", "Dokumente"),
        (DOCUMENTS_DELETE, "Dokumente löschen", "Dokumente"),
        (SECRETS_PRESENCE_READ, "Secret-Präsenz sehen", "Secrets"),
        (SECRETS_REVEAL, "Secrets anzeigen", "Secrets"),
        (SECRETS_WRITE, "Secrets schreiben", "Secrets"),
        (SECRET_ACCESS_CREATE, "Secret-Freigabe erstellen", "Secrets"),
        (SECRET_ACCESS_REVOKE, "Secret-Freigabe widerrufen", "Secrets"),
        (SECRET_ACCESS_READ, "Secret-Freigaben lesen", "Secrets"),
        (WAN_READ, "WAN lesen", "WAN"),
        (WAN_WRITE, "WAN schreiben", "WAN"),
        (WAN_DELETE, "WAN löschen", "WAN"),
        (QUOTES_READ, "Angebote lesen", "Angebote"),
        (QUOTES_WRITE, "Angebote schreiben", "Angebote"),
        (QUOTES_APPROVE, "Angebote freigeben", "Angebote"),
        (ORDERS_READ, "Aufträge lesen", "Aufträge"),
        (ORDERS_WRITE, "Aufträge schreiben", "Aufträge"),
        (SERVICE_JOBS_READ, "Serviceeinsätze lesen", "Service"),
        (SERVICE_JOBS_WRITE, "Serviceeinsätze schreiben", "Service"),
        (DISPATCH_READ, "Disposition lesen", "Service"),
        (DISPATCH_WRITE, "Disposition schreiben", "Service"),
        (TIME_ENTRIES_READ, "Zeiten lesen", "Service"),
        (TIME_ENTRIES_WRITE, "Zeiten erfassen", "Service"),
        (TIME_ENTRIES_APPROVE, "Zeiten freigeben", "Service"),
        (SERVICE_REPORTS_READ, "Leistungsnachweise lesen", "Service"),
        (SERVICE_REPORTS_WRITE, "Leistungsnachweise schreiben", "Service"),
        (SERVICE_REPORTS_APPROVE, "Leistungsnachweise freigeben", "Service"),
        (INVOICES_READ, "Rechnungen lesen", "Rechnungen"),
        (INVOICES_WRITE, "Rechnungen schreiben", "Rechnungen"),
        (INVOICES_APPROVE, "Rechnungen freigeben", "Rechnungen"),
        (INVOICES_CANCEL, "Rechnungen stornieren", "Rechnungen"),
        (XRECHNUNG_EXPORT, "XRechnung exportieren", "Rechnungen"),
        (ACCOUNTING_READ, "Buchhaltung lesen", "Buchhaltung"),
        (ACCOUNTING_WRITE, "Buchhaltung schreiben", "Buchhaltung"),
        (ACCOUNTING_EXPORT, "Buchhaltung exportieren", "Buchhaltung"),
        (DATEV_EXPORT, "DATEV-Export", "Buchhaltung"),
        (PURCHASES_READ, "Eingangsrechnungen lesen", "Buchhaltung"),
        (PURCHASES_WRITE, "Eingangsrechnungen schreiben", "Buchhaltung"),
        (OPEN_ITEMS_READ, "Offene Posten lesen", "Buchhaltung"),
        (OPEN_ITEMS_WRITE, "Offene Posten schreiben", "Buchhaltung"),
        (CHANGES_READ, "Changes lesen", "Changes"),
        (CHANGES_WRITE, "Changes schreiben", "Changes"),
        (CHANGES_APPROVE, "Changes freigeben", "Changes"),
        (CHANGES_CLOSE, "Changes abschließen", "Changes"),
        (BACKUP_READ, "Backups lesen", "Backup"),
        (BACKUP_CREATE, "Backups erstellen", "Backup"),
        (BACKUP_DOWNLOAD, "Backups herunterladen", "Backup"),
        (BACKUP_RESTORE, "Restore ausführen", "Backup"),
        (S3_READ, "S3 lesen", "System"),
        (S3_WRITE, "S3 konfigurieren", "System"),
        (SMTP_READ, "SMTP lesen", "System"),
        (SMTP_WRITE, "SMTP konfigurieren", "System"),
        (USERS_READ, "Benutzer lesen", "Benutzer"),
        (USERS_WRITE, "Benutzer schreiben", "Benutzer"),
        (USERS_DELETE, "Benutzer löschen", "Benutzer"),
        (ROLES_READ, "Rollen lesen", "Rollen"),
        (ROLES_WRITE, "Rollen schreiben", "Rollen"),
        (ROLES_DELETE, "Rollen löschen", "Rollen"),
        (API_TOKENS_READ, "API-Tokens lesen", "API"),
        (API_TOKENS_WRITE, "API-Tokens verwalten", "API"),
        (AUDIT_READ, "Audit-Log lesen", "Audit"),
        (SYSTEM_READ, "System-Einstellungen lesen", "System"),
        (SYSTEM_WRITE, "System-Einstellungen schreiben", "System"),
        (RECOVERY_USE, "Recovery Mode nutzen", "System"),
        (EMPLOYEES_READ, "Mitarbeiter lesen", "Mitarbeiter"),
        (EMPLOYEES_WRITE, "Mitarbeiter schreiben", "Mitarbeiter"),
    ]
}

// Default permissions for each system role
pub fn superadmin_permissions() -> Vec<&'static str> {
    all_permissions().into_iter().map(|(name, _, _)| name).collect()
}

pub fn admin_permissions() -> Vec<&'static str> {
    vec![
        CUSTOMERS_READ, CUSTOMERS_WRITE,
        CONTACTS_READ, CONTACTS_WRITE, CONTACTS_PRIVACY_MANAGE,
        LOCATIONS_READ, LOCATIONS_WRITE,
        ASSETS_READ, ASSETS_WRITE,
        DOCUMENTS_READ, DOCUMENTS_WRITE, DOCUMENTS_DOWNLOAD, DOCUMENTS_DELETE,
        SECRETS_PRESENCE_READ, SECRETS_REVEAL, SECRETS_WRITE,
        SECRET_ACCESS_CREATE, SECRET_ACCESS_REVOKE, SECRET_ACCESS_READ,
        WAN_READ, WAN_WRITE,
        QUOTES_READ, QUOTES_WRITE, QUOTES_APPROVE,
        ORDERS_READ, ORDERS_WRITE,
        SERVICE_JOBS_READ, SERVICE_JOBS_WRITE,
        DISPATCH_READ, DISPATCH_WRITE,
        TIME_ENTRIES_READ, TIME_ENTRIES_WRITE, TIME_ENTRIES_APPROVE,
        SERVICE_REPORTS_READ, SERVICE_REPORTS_WRITE, SERVICE_REPORTS_APPROVE,
        INVOICES_READ, INVOICES_WRITE, INVOICES_APPROVE,
        ACCOUNTING_READ,
        PURCHASES_READ,
        OPEN_ITEMS_READ,
        CHANGES_READ, CHANGES_WRITE, CHANGES_APPROVE,
        BACKUP_READ, BACKUP_CREATE,
        SMTP_READ,
        USERS_READ, USERS_WRITE,
        ROLES_READ,
        API_TOKENS_READ,
        AUDIT_READ,
        SYSTEM_READ,
        EMPLOYEES_READ, EMPLOYEES_WRITE,
    ]
}

pub fn manager_permissions() -> Vec<&'static str> {
    vec![
        CUSTOMERS_READ, CUSTOMERS_WRITE,
        CONTACTS_READ, CONTACTS_WRITE,
        LOCATIONS_READ, LOCATIONS_WRITE,
        ASSETS_READ, ASSETS_WRITE,
        DOCUMENTS_READ, DOCUMENTS_WRITE, DOCUMENTS_DOWNLOAD,
        SECRETS_PRESENCE_READ,
        SECRET_ACCESS_CREATE, SECRET_ACCESS_READ,
        WAN_READ, WAN_WRITE,
        QUOTES_READ, QUOTES_WRITE,
        ORDERS_READ, ORDERS_WRITE,
        SERVICE_JOBS_READ, SERVICE_JOBS_WRITE,
        DISPATCH_READ, DISPATCH_WRITE,
        TIME_ENTRIES_READ, TIME_ENTRIES_APPROVE,
        SERVICE_REPORTS_READ, SERVICE_REPORTS_APPROVE,
        INVOICES_READ, INVOICES_WRITE,
        CHANGES_READ, CHANGES_WRITE,
        EMPLOYEES_READ,
    ]
}

pub fn service_permissions() -> Vec<&'static str> {
    vec![
        CUSTOMERS_READ,
        CONTACTS_READ,
        LOCATIONS_READ,
        ASSETS_READ,
        DOCUMENTS_READ,
        SERVICE_JOBS_READ, SERVICE_JOBS_WRITE,
        TIME_ENTRIES_WRITE,
        SERVICE_REPORTS_READ, SERVICE_REPORTS_WRITE,
        CHANGES_READ,
    ]
}
