use std::fmt::Display;

use axum::http::StatusCode;
use tracing::{error, warn};

use crate::response::{ERR, Response};

pub const LOGIN_EXISTS: &str = "email already registered, login with password or reset password";
pub const LOGIN_FAILED: &str = "no user with email and password combination found";

pub const INTERNAL: &str = "unexpected error occured internally";
pub const INTERNAL_DB: &str = "unexpected error occured retrieving data";

pub const INVALID_REQUEST: &str = "invalid user request";

pub type Result<T> = std::result::Result<T, Error>;

/// Error used with crate utility functions
///
/// Most function (except Axum handlers) will utilize this Error.
/// These errors are handled within each handler, however, not returned to client
#[derive(Debug)]
pub enum Error {
    InternalDatabase(String),
    Internal(String),
    UserRequest(String),
    UserAuth(String),
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(value: deadpool_postgres::PoolError) -> Self {
        Self::InternalDatabase(value.to_string())
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(value: tokio_postgres::Error) -> Self {
        Self::InternalDatabase(value.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InternalDatabase(s)
            | Error::Internal(s)
            | Error::UserRequest(s)
            | Error::UserAuth(s) => write!(f, "Error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

pub type ErrorResponse = Response;

impl From<Error> for ErrorResponse {
    fn from(value: Error) -> Self {
        match value {
            Error::InternalDatabase(s) => {
                error!("Database error: {s}");

                Self::with_msg(StatusCode::INTERNAL_SERVER_ERROR, ERR, INTERNAL_DB)
            }
            Error::Internal(s) => {
                error!("Internal error: {s}");

                Self::with_msg(StatusCode::INTERNAL_SERVER_ERROR, ERR, INTERNAL)
            }
            Error::UserRequest(s) => {
                warn!("Request error: {s}");

                Self::with_msg(StatusCode::BAD_REQUEST, ERR, INVALID_REQUEST)
            }
            Error::UserAuth(s) => {
                warn!("Authentication error: {s}");

                Self::with_msg(StatusCode::UNAUTHORIZED, ERR, s.as_str())
            }
        }
    }
}
