mod com;
mod db;
mod error;
mod response;
mod routes;

use routes::create_api_router;

pub use db::DatabaseConn;

use axum::{Router, extract::FromRef, routing::get};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db_conn: DatabaseConn,
}

impl AppState {
    pub fn with_db(conn: DatabaseConn) -> Self {
        Self { db_conn: conn }
    }
}

pub fn create_app(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health_check_handler))
        .nest("/api", create_api_router())
        .with_state(app_state)
}
