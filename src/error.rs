use std::error::Error as StdError;
use std::fmt::Display;

use crate::com::Error as ComError;
use crate::db::{ApplicationError, Error as DbError};

/// Error used with crate utility functions
///
/// Most function (except Axum handlers) will utilize this Error.
/// These errors are handled within each handler, however, not returned to client
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
            Error::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            Error::InternalServer(msg) => write!(f, "internal server error: {}", msg),
        }
    }
}

impl StdError for Error {}

impl From<ComError> for Error {
    fn from(value: ComError) -> Self {
        match value {
            ComError::DateRangeConversion(msg) => {
                Self::InvalidRequest(format!("invalid date range specified in query: {}", msg))
            }
        }
    }
}

impl From<DbError> for Error {
    fn from(value: DbError) -> Self {
        match value {
            DbError::Application(app_err) => match app_err {
                ApplicationError::TaskNotFound | ApplicationError::TagNotFound => {
                    Self::NotFound(app_err.to_string())
                }
                ApplicationError::Misc(msg) => Self::InvalidRequest(msg),
            },
            err => Self::InternalServer(err.to_string()),
        }
    }
}
