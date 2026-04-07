//! Route Error Module
//!
//! This module contains the Error type used within the handlers
//!
//! Includes:
//!
//! * Conversion of other module Error into common Error to convert into response to client

use std::error::Error as StdError;
use std::fmt::Display;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::db::{ApplicationError, Error as DbError};

/// Unified error type returned by API handlers
///
/// This enum represents all errors that can be exposed to clients/
///
/// Each variant maps to an HTTP status code:
/// - `NotFound`        -> 404 Not Found
/// - `InvalidRequest`  -> 400 Bad Request
/// - `InternalServer`  -> 500 Internal Server Error
#[derive(Debug)]
pub enum Error {
    NotFound(String),
    InvalidRequest(String),
    InternalServer(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound(msg) => write!(f, "{}", msg),
            Error::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Error::InternalServer(msg) => write!(f, "Internal server error: {}", msg),
        }
    }
}

impl StdError for Error {}

impl From<DbError> for Error {
    fn from(value: DbError) -> Self {
        match value {
            DbError::Application(ref app_err) => match app_err {
                ApplicationError::TaskNotFound | ApplicationError::TagNotFound => {
                    Self::NotFound(app_err.to_string())
                }
                ApplicationError::InvalidDateRange(_) => Self::InvalidRequest(app_err.to_string()),
                ApplicationError::UncaughtIntegrity(msg)
                | ApplicationError::UncaughtTrigger(msg) => Self::InternalServer(msg.to_string()),
            },
            err => Self::InternalServer(err.to_string()),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json::from(json!({
                    "message": msg,
                })),
            ),
            Error::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json::from(json!({
                    "message": msg,
                })),
            ),
            Error::InternalServer(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json::from(json!({
                    "message": msg,
                })),
            ),
        }
        .into_response()
    }
}
