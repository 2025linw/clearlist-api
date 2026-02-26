mod com;
mod db;
mod error;
mod response;
mod routes;

use routes::create_api_router;

pub use db::DatabaseConn;

use axum::{Router, extract::FromRef, http::{Method, header}, routing::get};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

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
    let origins = [
        "***REMOVED***".parse().unwrap(),
        "***REMOVED***".parse().unwrap(),
    ];
    let headers = [
        header::CONTENT_TYPE,
        // header::AUTHORIZATION,
    ];

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_headers(headers)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    Router::new()
        .layer(ServiceBuilder::new().layer(cors))
        .route("/health", get(routes::health_check_handler))
        .nest("/api", create_api_router())
        .with_state(app_state)
}
