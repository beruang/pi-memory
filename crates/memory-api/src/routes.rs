use axum::{Router, routing::get, routing::post, routing::put, routing::delete};

use super::handlers;

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/observations", get(handlers::list_observations))
        .route("/observations", post(handlers::create_observation))
        .route("/observations/{id}", get(handlers::get_observation))
        .route("/observations/{id}", put(handlers::update_observation))
        .route("/observations/{id}", delete(handlers::delete_observation))
        .route("/search", get(handlers::search))
        .route("/conflicts", get(handlers::list_conflicts))
        .route("/conflicts/{id}/resolve", post(handlers::resolve_conflict))
}
