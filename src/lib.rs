//! Clear List API Library
//!
//! This module is the main library used to create Clear List API web server

mod com;
mod models;
mod response;

mod db;
mod routes;

pub use db::{DatabaseConn, run_migration};

use axum::{Router, routing::get};

use crate::routes::missing_404_handler;

/// App State Type
///
/// `AppState` is used for reused resources throughout web server (such as database connections, etc)
#[derive(Clone)]
pub struct AppState {
    db: DatabaseConn,
}

impl AppState {
    /// Initialize an AppState with a database connection given by DatabaseConn
    pub fn init(conn: DatabaseConn) -> Self {
        Self { db: conn }
    }
}

/// Creates a new Router to be used as the app for Clear List API webserver
pub fn create_app(app_state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::health_check_handler))
        .nest("/api", routes::create_api_router())
        .fallback(missing_404_handler)
        .with_state(app_state)
}
