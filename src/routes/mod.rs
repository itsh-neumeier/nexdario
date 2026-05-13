use crate::state::AppState;
use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_http::services::ServeDir;

pub mod api;
pub mod asset_types;
pub mod assets;
pub mod auth;
pub mod backup;
pub mod changes;
pub mod contacts;
pub mod customers;
pub mod dashboard;
pub mod employees;
pub mod invoices;
pub mod locations;
pub mod quotes;
pub mod recovery;
pub mod roles;
pub mod secrets;
pub mod service_jobs;
pub mod users;
pub mod wan;

pub fn create_router(state: AppState) -> Router {
    let static_dir = state.config.static_dir.clone();

    Router::new()
        // Auth
        .route("/login", get(auth::login_page).post(auth::login))
        .route("/logout", post(auth::logout))
        // Dashboard
        .route("/", get(dashboard::index))
        // Customers — forms POST to /customers (create) and /customers/:id (update)
        .route("/customers", get(customers::list).post(customers::create))
        .route("/customers/new", get(customers::new_form))
        .route("/customers/:id", get(customers::detail).post(customers::update))
        .route("/customers/:id/edit", get(customers::edit_form))
        .route("/customers/:id/delete", post(customers::delete))
        // Contacts
        .route("/contacts", get(contacts::list).post(contacts::create))
        .route("/contacts/new", get(contacts::new_form))
        .route("/contacts/:id", get(contacts::detail).post(contacts::update))
        .route("/contacts/:id/edit", get(contacts::edit_form))
        .route("/contacts/:id/delete", post(contacts::delete))
        // Locations
        .route("/locations", get(locations::list).post(locations::create))
        .route("/locations/new", get(locations::new_form))
        .route("/locations/:id", get(locations::detail).post(locations::update))
        .route("/locations/:id/edit", get(locations::edit_form))
        .route("/locations/:id/delete", post(locations::delete))
        // Assets
        .route("/assets", get(assets::list).post(assets::create))
        .route("/assets/new", get(assets::new_form))
        .route("/assets/:id", get(assets::detail).post(assets::update))
        .route("/assets/:id/edit", get(assets::edit_form))
        .route("/assets/:id/delete", post(assets::delete))
        // Asset Types
        .route("/asset-types", get(asset_types::list).post(asset_types::create))
        .route("/asset-types/:id", post(asset_types::update))
        .route("/asset-types/:id/delete", post(asset_types::delete))
        // Users
        .route("/users", get(users::list).post(users::create))
        .route("/users/new", get(users::new_form))
        .route("/users/:id", get(users::detail).post(users::update))
        .route("/users/:id/edit", get(users::edit_form))
        .route("/users/:id/delete", post(users::delete))
        .route("/users/:id/roles", get(users::roles_form).post(users::update_roles))
        // Roles
        .route("/roles", get(roles::list).post(roles::create))
        .route("/roles/new", get(roles::new_form))
        .route("/roles/:id", get(roles::detail).post(roles::update))
        .route("/roles/:id/edit", get(roles::edit_form))
        .route("/roles/:id/delete", post(roles::delete))
        // WAN
        .route("/wan", get(wan::list).post(wan::create))
        .route("/wan/new", get(wan::new_form))
        .route("/wan/:id", post(wan::update))
        .route("/wan/:id/edit", get(wan::edit_form))
        .route("/wan/:id/delete", post(wan::delete))
        // Secrets
        .route("/secrets", get(secrets::list).post(secrets::create))
        .route("/secrets/new", get(secrets::new_form))
        .route("/secrets/:id", get(secrets::detail).post(secrets::update))
        .route("/secrets/:id/edit", get(secrets::edit_form))
        .route("/secrets/:id/delete", post(secrets::delete))
        .route("/secrets/:id/reveal", get(secrets::reveal).post(secrets::reveal))
        .route("/secrets/:id/access-token", get(secrets::access_token_form).post(secrets::create_access_token))
        .route("/secret-access/:token", get(secrets::use_access_token))
        // Quotes
        .route("/quotes", get(quotes::list).post(quotes::create))
        .route("/quotes/new", get(quotes::new_form))
        .route("/quotes/:id", get(quotes::detail).post(quotes::update))
        .route("/quotes/:id/edit", get(quotes::edit_form))
        .route("/quotes/:id/delete", post(quotes::delete))
        .route("/quotes/:id/send", post(quotes::send))
        .route("/quotes/:id/accept", post(quotes::accept))
        .route("/quotes/:id/reject", post(quotes::reject))
        // Service Jobs
        .route("/service-jobs", get(service_jobs::list).post(service_jobs::create))
        .route("/service-jobs/new", get(service_jobs::new_form))
        .route("/service-jobs/:id", get(service_jobs::detail).post(service_jobs::update))
        .route("/service-jobs/:id/edit", get(service_jobs::edit_form))
        .route("/service-jobs/:id/delete", post(service_jobs::delete))
        .route("/service-jobs/:id/start", post(service_jobs::start))
        .route("/service-jobs/:id/complete", post(service_jobs::complete))
        .route("/service-jobs/:id/time", get(service_jobs::time_form).post(service_jobs::add_time))
        // Employees
        .route("/employees", get(employees::list).post(employees::create))
        .route("/employees/new", get(employees::new_form))
        .route("/employees/:id", get(employees::detail).post(employees::update))
        .route("/employees/:id/edit", get(employees::edit_form))
        .route("/employees/:id/delete", post(employees::delete))
        // Invoices
        .route("/invoices", get(invoices::list).post(invoices::create))
        .route("/invoices/new", get(invoices::new_form))
        .route("/invoices/:id", get(invoices::detail).post(invoices::update))
        .route("/invoices/:id/edit", get(invoices::edit_form))
        .route("/invoices/:id/approve", post(invoices::approve))
        .route("/invoices/:id/send", post(invoices::send))
        .route("/invoices/:id/paid", post(invoices::mark_paid))
        .route("/invoices/:id/cancel", post(invoices::cancel))
        .route("/invoices/:id/xrechnung", get(invoices::export_xrechnung))
        // Changes
        .route("/changes", get(changes::list).post(changes::create))
        .route("/changes/new", get(changes::new_form))
        .route("/changes/:id", get(changes::detail).post(changes::update))
        .route("/changes/:id/edit", get(changes::edit_form))
        .route("/changes/:id/delete", post(changes::delete))
        .route("/changes/:id/submit", post(changes::submit))
        .route("/changes/:id/approve", post(changes::approve))
        .route("/changes/:id/reject", post(changes::reject))
        .route("/changes/:id/start", post(changes::start))
        .route("/changes/:id/close", post(changes::close))
        // Backup
        .route("/backup", get(backup::index))
        .route("/backup/create", post(backup::create))
        .route("/backup/download/:filename", get(backup::download))
        .route("/backup/delete/:filename", post(backup::delete_backup))
        // Recovery (public, token-protected)
        .route("/recovery", get(recovery::index).post(recovery::action))
        // API
        .nest("/api/v1", api::create_api_router())
        // Static files
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state)
}
