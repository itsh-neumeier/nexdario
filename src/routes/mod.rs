use crate::state::AppState;
use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_http::services::ServeDir;

pub mod api;
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
        // Customers
        .route("/customers", get(customers::list))
        .route("/customers/new", get(customers::new_form).post(customers::create))
        .route("/customers/:id", get(customers::detail))
        .route("/customers/:id/edit", get(customers::edit_form).post(customers::update))
        .route("/customers/:id/delete", post(customers::delete))
        // Contacts
        .route("/contacts", get(contacts::list))
        .route("/contacts/new", get(contacts::new_form).post(contacts::create))
        .route("/contacts/:id", get(contacts::detail))
        .route("/contacts/:id/edit", get(contacts::edit_form).post(contacts::update))
        .route("/contacts/:id/delete", post(contacts::delete))
        // Locations
        .route("/locations", get(locations::list))
        .route("/locations/new", get(locations::new_form).post(locations::create))
        .route("/locations/:id", get(locations::detail))
        .route("/locations/:id/edit", get(locations::edit_form).post(locations::update))
        .route("/locations/:id/delete", post(locations::delete))
        // Assets
        .route("/assets", get(assets::list))
        .route("/assets/new", get(assets::new_form).post(assets::create))
        .route("/assets/:id", get(assets::detail))
        .route("/assets/:id/edit", get(assets::edit_form).post(assets::update))
        .route("/assets/:id/delete", post(assets::delete))
        // Users
        .route("/users", get(users::list))
        .route("/users/new", get(users::new_form).post(users::create))
        .route("/users/:id", get(users::detail))
        .route("/users/:id/edit", get(users::edit_form).post(users::update))
        .route("/users/:id/delete", post(users::delete))
        .route("/users/:id/roles", get(users::roles_form).post(users::update_roles))
        // Roles
        .route("/roles", get(roles::list))
        .route("/roles/new", get(roles::new_form).post(roles::create))
        .route("/roles/:id", get(roles::detail))
        .route("/roles/:id/edit", get(roles::edit_form).post(roles::update))
        .route("/roles/:id/delete", post(roles::delete))
        // WAN
        .route("/wan", get(wan::list))
        .route("/wan/new", get(wan::new_form).post(wan::create))
        .route("/wan/:id/edit", get(wan::edit_form).post(wan::update))
        .route("/wan/:id/delete", post(wan::delete))
        // Secrets
        .route("/secrets", get(secrets::list))
        .route("/secrets/new", get(secrets::new_form).post(secrets::create))
        .route("/secrets/:id", get(secrets::detail))
        .route("/secrets/:id/edit", get(secrets::edit_form).post(secrets::update))
        .route("/secrets/:id/delete", post(secrets::delete))
        .route("/secrets/:id/reveal", get(secrets::reveal).post(secrets::reveal))
        .route("/secrets/:id/access-token", get(secrets::access_token_form).post(secrets::create_access_token))
        .route("/secret-access/:token", get(secrets::use_access_token))
        // Quotes
        .route("/quotes", get(quotes::list))
        .route("/quotes/new", get(quotes::new_form).post(quotes::create))
        .route("/quotes/:id", get(quotes::detail))
        .route("/quotes/:id/edit", get(quotes::edit_form).post(quotes::update))
        .route("/quotes/:id/delete", post(quotes::delete))
        .route("/quotes/:id/send", post(quotes::send))
        .route("/quotes/:id/accept", post(quotes::accept))
        .route("/quotes/:id/reject", post(quotes::reject))
        // Service Jobs
        .route("/service-jobs", get(service_jobs::list))
        .route("/service-jobs/new", get(service_jobs::new_form).post(service_jobs::create))
        .route("/service-jobs/:id", get(service_jobs::detail))
        .route("/service-jobs/:id/edit", get(service_jobs::edit_form).post(service_jobs::update))
        .route("/service-jobs/:id/delete", post(service_jobs::delete))
        .route("/service-jobs/:id/start", post(service_jobs::start))
        .route("/service-jobs/:id/complete", post(service_jobs::complete))
        .route("/service-jobs/:id/time", get(service_jobs::time_form).post(service_jobs::add_time))
        // Employees
        .route("/employees", get(employees::list))
        .route("/employees/new", get(employees::new_form).post(employees::create))
        .route("/employees/:id", get(employees::detail))
        .route("/employees/:id/edit", get(employees::edit_form).post(employees::update))
        .route("/employees/:id/delete", post(employees::delete))
        // Invoices
        .route("/invoices", get(invoices::list))
        .route("/invoices/new", get(invoices::new_form).post(invoices::create))
        .route("/invoices/:id", get(invoices::detail))
        .route("/invoices/:id/edit", get(invoices::edit_form).post(invoices::update))
        .route("/invoices/:id/approve", post(invoices::approve))
        .route("/invoices/:id/send", post(invoices::send))
        .route("/invoices/:id/paid", post(invoices::mark_paid))
        .route("/invoices/:id/cancel", post(invoices::cancel))
        .route("/invoices/:id/xrechnung", get(invoices::export_xrechnung))
        // Changes
        .route("/changes", get(changes::list))
        .route("/changes/new", get(changes::new_form).post(changes::create))
        .route("/changes/:id", get(changes::detail))
        .route("/changes/:id/edit", get(changes::edit_form).post(changes::update))
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
