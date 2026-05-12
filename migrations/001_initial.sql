-- Nexdario Database Schema v1
-- Migration 001: Initial schema

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- Users
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_system INTEGER NOT NULL DEFAULT 0,
    last_login_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Roles
CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    description TEXT,
    rank INTEGER NOT NULL DEFAULT 100,
    is_system INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    allow_api_access INTEGER NOT NULL DEFAULT 0,
    mobile_access INTEGER NOT NULL DEFAULT 0,
    default_landing TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Permissions
CREATE TABLE IF NOT EXISTS permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    description TEXT,
    module TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Role Permissions
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

-- User Roles
CREATE TABLE IF NOT EXISTS user_roles (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_by INTEGER REFERENCES users(id),
    assigned_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, role_id)
);

-- User direct permissions
CREATE TABLE IF NOT EXISTS user_permissions (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission_id INTEGER NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    is_deny INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, permission_id)
);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL DEFAULT (datetime('now')),
    ip_address TEXT,
    user_agent TEXT
);

-- API Tokens
CREATE TABLE IF NOT EXISTS api_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    user_id INTEGER REFERENCES users(id),
    scopes TEXT NOT NULL DEFAULT '[]',
    is_read_only INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    expires_at TEXT,
    last_used_at TEXT,
    last_used_ip TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Customers
CREATE TABLE IF NOT EXISTS customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_number TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    customer_type TEXT NOT NULL DEFAULT 'business',
    status TEXT NOT NULL DEFAULT 'active',
    industry TEXT,
    website TEXT,
    phone TEXT,
    email TEXT,
    billing_street TEXT,
    billing_zip TEXT,
    billing_city TEXT,
    billing_country TEXT NOT NULL DEFAULT 'DE',
    delivery_street TEXT,
    delivery_zip TEXT,
    delivery_city TEXT,
    delivery_country TEXT,
    debtor_account TEXT,
    vat_id TEXT,
    tax_country TEXT NOT NULL DEFAULT 'DE',
    payment_terms TEXT NOT NULL DEFAULT '14',
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Contacts
CREATE TABLE IF NOT EXISTS contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER REFERENCES customers(id),
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    position TEXT,
    department TEXT,
    role TEXT,
    phone TEXT,
    mobile TEXT,
    email TEXT,
    email_alt TEXT,
    preferred_contact TEXT NOT NULL DEFAULT 'email',
    language TEXT NOT NULL DEFAULT 'de',
    description TEXT,
    notes TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_technical INTEGER NOT NULL DEFAULT 0,
    is_commercial INTEGER NOT NULL DEFAULT 0,
    is_emergency INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    name_visible_to_service INTEGER NOT NULL DEFAULT 0,
    phone_visible_to_service INTEGER NOT NULL DEFAULT 1,
    mobile_visible_to_service INTEGER NOT NULL DEFAULT 0,
    email_visible_to_service INTEGER NOT NULL DEFAULT 0,
    role_visible_to_service INTEGER NOT NULL DEFAULT 1,
    department_visible_to_service INTEGER NOT NULL DEFAULT 0,
    description_visible_to_service INTEGER NOT NULL DEFAULT 1,
    notes_visible_to_service INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Locations
CREATE TABLE IF NOT EXISTS locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    street TEXT,
    zip TEXT,
    city TEXT,
    country TEXT NOT NULL DEFAULT 'DE',
    plus_code TEXT,
    latitude REAL,
    longitude REAL,
    building TEXT,
    floor TEXT,
    room_notes TEXT,
    rack_notes TEXT,
    access_notes TEXT,
    opening_hours TEXT,
    parking_notes TEXT,
    technical_notes TEXT,
    internal_notes TEXT,
    service_notes TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Assets
CREATE TABLE IF NOT EXISTS assets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hostname TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    device_type TEXT NOT NULL,
    role TEXT,
    manufacturer TEXT,
    model TEXT,
    serial_number TEXT,
    mac_address TEXT,
    management_ip TEXT,
    firmware_version TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    description TEXT,
    warranty_until TEXT,
    maintenance_until TEXT,
    last_check TEXT,
    unifi_device_id TEXT,
    unifi_site TEXT,
    unifi_adoption_status TEXT,
    unifi_uplink_device TEXT,
    unifi_uplink_port TEXT,
    unifi_poe_consumption REAL,
    unifi_channel TEXT,
    unifi_tx_power TEXT,
    unifi_clients_current INTEGER,
    unifi_last_contact TEXT,
    unifi_online INTEGER,
    unifi_controller_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Documents
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    storage_type TEXT NOT NULL DEFAULT 'sqlite_blob',
    file_data BLOB,
    file_path TEXT,
    category TEXT,
    visibility TEXT NOT NULL DEFAULT 'internal',
    customer_id INTEGER REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    asset_id INTEGER REFERENCES assets(id),
    contact_id INTEGER REFERENCES contacts(id),
    uploaded_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- WAN Connections
CREATE TABLE IF NOT EXISTS wan_connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    location_id INTEGER NOT NULL REFERENCES locations(id),
    name TEXT NOT NULL,
    provider TEXT,
    connection_type TEXT,
    role TEXT NOT NULL DEFAULT 'PRIMARY',
    status TEXT NOT NULL DEFAULT 'active',
    circuit_id TEXT,
    customer_number TEXT,
    contract_number TEXT,
    support_contact TEXT,
    bandwidth_down INTEGER,
    bandwidth_up INTEGER,
    static_ipv4 TEXT,
    static_ipv6 TEXT,
    subnets TEXT,
    gateway TEXT,
    dns_primary TEXT,
    dns_secondary TEXT,
    vlan_id INTEGER,
    modem_data TEXT,
    pppoe_username TEXT,
    pppoe_password_encrypted TEXT,
    auth_method TEXT,
    failover_connection_id INTEGER REFERENCES wan_connections(id),
    monitoring_url TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Secrets
CREATE TABLE IF NOT EXISTS secrets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    secret_type TEXT NOT NULL,
    username TEXT,
    password_encrypted TEXT,
    url TEXT,
    description TEXT,
    customer_id INTEGER REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    asset_id INTEGER REFERENCES assets(id),
    is_active INTEGER NOT NULL DEFAULT 1,
    created_by INTEGER REFERENCES users(id),
    updated_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Secret Access Tokens
CREATE TABLE IF NOT EXISTS secret_access_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    token_hash TEXT NOT NULL UNIQUE,
    secret_id INTEGER NOT NULL REFERENCES secrets(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL,
    access_type TEXT NOT NULL DEFAULT 'one_time',
    is_active INTEGER NOT NULL DEFAULT 1,
    expires_at TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    max_uses INTEGER,
    last_used_at TEXT,
    last_access_ip TEXT,
    last_user_agent TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Location Documentation
CREATE TABLE IF NOT EXISTS location_docs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    category TEXT,
    visibility TEXT NOT NULL DEFAULT 'internal',
    is_pinned INTEGER NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_by INTEGER REFERENCES users(id),
    updated_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Employees
CREATE TABLE IF NOT EXISTS employees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    employee_number TEXT NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    mobile TEXT,
    position TEXT,
    department TEXT,
    qualifications TEXT,
    hourly_rate REAL,
    is_active INTEGER NOT NULL DEFAULT 1,
    user_id INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Catalog Items
CREATE TABLE IF NOT EXISTS catalog_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_number TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    item_type TEXT NOT NULL,
    category TEXT NOT NULL,
    unit TEXT NOT NULL DEFAULT 'Stk',
    purchase_price REAL,
    sales_price REAL NOT NULL DEFAULT 0,
    tax_rate REAL NOT NULL DEFAULT 19,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Quotes
CREATE TABLE IF NOT EXISTS quotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    quote_number TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    version INTEGER NOT NULL DEFAULT 1,
    valid_until TEXT,
    payment_terms TEXT,
    delivery_terms TEXT,
    notes TEXT,
    internal_notes TEXT,
    subtotal REAL NOT NULL DEFAULT 0,
    discount_amount REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    total REAL NOT NULL DEFAULT 0,
    created_by INTEGER REFERENCES users(id),
    approved_by INTEGER REFERENCES users(id),
    approved_at TEXT,
    sent_at TEXT,
    accepted_at TEXT,
    rejected_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Quote Items
CREATE TABLE IF NOT EXISTS quote_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    quote_id INTEGER NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    item_type TEXT NOT NULL DEFAULT 'item',
    catalog_item_id INTEGER REFERENCES catalog_items(id),
    name TEXT NOT NULL,
    description TEXT,
    quantity REAL NOT NULL DEFAULT 1,
    unit TEXT NOT NULL DEFAULT 'Stk',
    unit_price REAL NOT NULL DEFAULT 0,
    discount_percent REAL NOT NULL DEFAULT 0,
    tax_rate REAL NOT NULL DEFAULT 19,
    subtotal REAL NOT NULL DEFAULT 0,
    is_optional INTEGER NOT NULL DEFAULT 0,
    is_alternative INTEGER NOT NULL DEFAULT 0
);

-- Orders
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_number TEXT NOT NULL UNIQUE,
    quote_id INTEGER REFERENCES quotes(id),
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    order_date TEXT NOT NULL,
    notes TEXT,
    internal_notes TEXT,
    subtotal REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    total REAL NOT NULL DEFAULT 0,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Service Jobs
CREATE TABLE IF NOT EXISTS service_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_number TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    order_id INTEGER REFERENCES orders(id),
    asset_id INTEGER REFERENCES assets(id),
    assigned_employee_id INTEGER REFERENCES employees(id),
    title TEXT NOT NULL,
    description TEXT,
    priority TEXT NOT NULL DEFAULT 'normal',
    status TEXT NOT NULL DEFAULT 'open',
    scheduled_start TEXT,
    scheduled_end TEXT,
    actual_start TEXT,
    actual_end TEXT,
    is_billable INTEGER NOT NULL DEFAULT 1,
    notes TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Time Entries
CREATE TABLE IF NOT EXISTS time_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_job_id INTEGER REFERENCES service_jobs(id),
    employee_id INTEGER NOT NULL REFERENCES employees(id),
    activity_type TEXT NOT NULL DEFAULT 'work',
    description TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration_minutes INTEGER,
    travel_time_minutes INTEGER NOT NULL DEFAULT 0,
    kilometers REAL NOT NULL DEFAULT 0,
    is_billable INTEGER NOT NULL DEFAULT 1,
    hourly_rate REAL,
    is_approved INTEGER NOT NULL DEFAULT 0,
    approved_by INTEGER REFERENCES users(id),
    approved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Invoices
CREATE TABLE IF NOT EXISTS invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_number TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    order_id INTEGER REFERENCES orders(id),
    invoice_type TEXT NOT NULL DEFAULT 'standard',
    status TEXT NOT NULL DEFAULT 'draft',
    invoice_date TEXT NOT NULL,
    delivery_date TEXT,
    due_date TEXT,
    payment_terms TEXT,
    our_tax_number TEXT,
    our_vat_id TEXT,
    our_bank_name TEXT,
    our_iban TEXT,
    our_bic TEXT,
    our_company_name TEXT,
    our_street TEXT,
    our_zip TEXT,
    our_city TEXT,
    our_country TEXT,
    customer_name TEXT NOT NULL,
    customer_street TEXT,
    customer_zip TEXT,
    customer_city TEXT,
    customer_country TEXT,
    customer_vat_id TEXT,
    leitweg_id TEXT,
    buyer_reference TEXT,
    purchase_order_number TEXT,
    project_reference TEXT,
    contract_reference TEXT,
    subtotal REAL NOT NULL DEFAULT 0,
    discount_amount REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    total REAL NOT NULL DEFAULT 0,
    amount_paid REAL NOT NULL DEFAULT 0,
    notes TEXT,
    internal_notes TEXT,
    cancelled_invoice_id INTEGER REFERENCES invoices(id),
    created_by INTEGER REFERENCES users(id),
    approved_by INTEGER REFERENCES users(id),
    approved_at TEXT,
    sent_at TEXT,
    paid_at TEXT,
    cancelled_at TEXT,
    exported_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Invoice Items
CREATE TABLE IF NOT EXISTS invoice_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    item_type TEXT NOT NULL DEFAULT 'item',
    catalog_item_id INTEGER REFERENCES catalog_items(id),
    name TEXT NOT NULL,
    description TEXT,
    quantity REAL NOT NULL DEFAULT 1,
    unit TEXT NOT NULL DEFAULT 'Stk',
    unit_price REAL NOT NULL DEFAULT 0,
    discount_percent REAL NOT NULL DEFAULT 0,
    tax_rate REAL NOT NULL DEFAULT 19,
    subtotal REAL NOT NULL DEFAULT 0
);

-- Incoming Invoices
CREATE TABLE IF NOT EXISTS incoming_invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_number TEXT NOT NULL UNIQUE,
    vendor_invoice_number TEXT,
    vendor_name TEXT NOT NULL,
    creditor_account TEXT,
    invoice_date TEXT NOT NULL,
    service_date TEXT,
    due_date TEXT,
    net_amount REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    gross_amount REAL NOT NULL DEFAULT 0,
    payment_status TEXT NOT NULL DEFAULT 'open',
    paid_amount REAL NOT NULL DEFAULT 0,
    paid_at TEXT,
    expense_account TEXT,
    cost_center TEXT,
    cost_unit TEXT,
    customer_id INTEGER REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    order_id INTEGER REFERENCES orders(id),
    notes TEXT,
    export_status TEXT NOT NULL DEFAULT 'pending',
    exported_at TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- IT Changes
CREATE TABLE IF NOT EXISTS changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    change_number TEXT NOT NULL UNIQUE,
    customer_id INTEGER NOT NULL REFERENCES customers(id),
    location_id INTEGER REFERENCES locations(id),
    asset_id INTEGER REFERENCES assets(id),
    order_id INTEGER REFERENCES orders(id),
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    risk_level TEXT NOT NULL DEFAULT 'low',
    impact TEXT,
    rollback_plan TEXT,
    test_plan TEXT,
    scheduled_start TEXT,
    scheduled_end TEXT,
    actual_start TEXT,
    actual_end TEXT,
    maintenance_window TEXT,
    assigned_employee_id INTEGER REFERENCES employees(id),
    reviewed_by INTEGER REFERENCES users(id),
    approved_by INTEGER REFERENCES users(id),
    closed_by INTEGER REFERENCES users(id),
    reviewed_at TEXT,
    approved_at TEXT,
    closed_at TEXT,
    failure_reason TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Audit Log
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER REFERENCES users(id),
    username TEXT,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT,
    ip_address TEXT,
    user_agent TEXT,
    success INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- System Settings
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    updated_by INTEGER REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Number Sequences
CREATE TABLE IF NOT EXISTS number_sequences (
    name TEXT PRIMARY KEY,
    prefix TEXT NOT NULL,
    current_year INTEGER NOT NULL,
    last_number INTEGER NOT NULL DEFAULT 0,
    min_digits INTEGER NOT NULL DEFAULT 4,
    include_year INTEGER NOT NULL DEFAULT 1,
    separator TEXT NOT NULL DEFAULT '-'
);

-- Backup History
CREATE TABLE IF NOT EXISTS backup_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    file_size INTEGER,
    checksum TEXT,
    backup_type TEXT NOT NULL DEFAULT 'manual',
    storage_location TEXT NOT NULL DEFAULT 'local',
    status TEXT NOT NULL DEFAULT 'completed',
    is_encrypted INTEGER NOT NULL DEFAULT 0,
    s3_path TEXT,
    error_message TEXT,
    created_by INTEGER REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- SMTP Settings (single row)
CREATE TABLE IF NOT EXISTS smtp_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    host TEXT,
    port INTEGER NOT NULL DEFAULT 587,
    username TEXT,
    password_encrypted TEXT,
    from_email TEXT,
    from_name TEXT,
    security TEXT NOT NULL DEFAULT 'starttls',
    is_enabled INTEGER NOT NULL DEFAULT 0,
    updated_by INTEGER REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- S3 Settings (single row)
CREATE TABLE IF NOT EXISTS s3_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    endpoint TEXT,
    region TEXT NOT NULL DEFAULT 'eu-central-1',
    bucket TEXT,
    prefix TEXT NOT NULL DEFAULT 'nexdario/',
    access_key TEXT,
    secret_key_encrypted TEXT,
    path_style INTEGER NOT NULL DEFAULT 1,
    retention_days INTEGER NOT NULL DEFAULT 30,
    is_enabled INTEGER NOT NULL DEFAULT 0,
    updated_by INTEGER REFERENCES users(id),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indices
CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_user ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_customers_status ON customers(status);
CREATE INDEX IF NOT EXISTS idx_contacts_customer ON contacts(customer_id);
CREATE INDEX IF NOT EXISTS idx_locations_customer ON locations(customer_id);
CREATE INDEX IF NOT EXISTS idx_assets_customer ON assets(customer_id);
CREATE INDEX IF NOT EXISTS idx_assets_location ON assets(location_id);
CREATE INDEX IF NOT EXISTS idx_assets_hostname ON assets(hostname);
CREATE INDEX IF NOT EXISTS idx_service_jobs_customer ON service_jobs(customer_id);
CREATE INDEX IF NOT EXISTS idx_service_jobs_status ON service_jobs(status);
CREATE INDEX IF NOT EXISTS idx_invoices_customer ON invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_quotes_customer ON quotes(customer_id);
CREATE INDEX IF NOT EXISTS idx_changes_customer ON changes(customer_id);
CREATE INDEX IF NOT EXISTS idx_changes_status ON changes(status);
CREATE INDEX IF NOT EXISTS idx_api_tokens_hash ON api_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_secret_tokens_hash ON secret_access_tokens(token_hash);
