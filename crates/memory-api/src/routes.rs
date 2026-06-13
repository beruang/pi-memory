use axum::{routing::delete, routing::get, routing::post, routing::put, Router};

use super::handlers::{self, AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::review_ui))
        .route("/health", get(handlers::health))
        .route("/observations", get(handlers::list_observations))
        .route("/observations", post(handlers::create_observation))
        .route("/observations/{id}", get(handlers::get_observation))
        .route("/observations/{id}", put(handlers::update_observation))
        .route("/observations/{id}", delete(handlers::delete_observation))
        .route("/search", get(handlers::search))
        .route("/conflicts", get(handlers::list_conflicts))
        .route("/conflicts/{id}/resolve", post(handlers::resolve_conflict))
        .with_state(state)
}
