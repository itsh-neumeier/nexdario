use axum::{
    routing::get,
    Router,
};
use crate::state::AppState;

pub mod customers;
pub mod system;

pub fn create_api_router() -> Router<AppState> {
    Router::new()
        // System
        .route("/system/health", get(system::health))
        .route("/system/info", get(system::info))
        // Customers
        .route("/customers", get(customers::list).post(customers::create))
        .route("/customers/:id", get(customers::get_one).put(customers::update).delete(customers::delete))
        // Additional API endpoints — TODO: implement remaining modules
}
