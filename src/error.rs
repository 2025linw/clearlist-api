use axum::{Json, http::StatusCode};
use serde_json::json;

// TODO: use thiserror
// TODO: better differentiate between different types of errors (user auth, database query, database pool, etc)
// TODO: include server side console prints or logs
// TODO: include client side responses that don't reveal too much about server architecture

/// Errors that are used to return back to Client
///
/// Any function that returns to client must use this to avoid
/// revealing any information about backend issues.
#[derive(Debug)]
pub enum Error {
    Internal(String),
    InvalidRequest(String),
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(value: deadpool_postgres::PoolError) -> Self {
        Self::Internal(value.to_string())
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

impl Into<(StatusCode, Json<serde_json::Value>)> for Error {
    fn into(self) -> (StatusCode, Json<serde_json::Value>) {
        match self {
            Error::Internal(s) => {
                eprintln!("Internal Error: {}", s);

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "status": "error",
                        "message": "error occured internally",
                    })),
                )
            }
            Error::InvalidRequest(s) => {
                eprintln!("User Request Error: {}", s);

                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "error",
                        "message": "error occured with request"
                    })),
                )
            }
        }
    }
}
