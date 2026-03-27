use std::fmt::Display;

use crate::com::Error as ComError;
use crate::db::Error as DbError;

/// Error used with crate utility functions
///
/// Most function (except Axum handlers) will utilize this Error.
/// These errors are handled within each handler, however, not returned to client
#[derive(Debug)]
pub enum Error {
    InvalidRequest(String),
    InternalServer(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
            Error::InternalServer(msg) => write!(f, "internal server error: {}", msg),
        }
    }
}

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
        Self::InternalServer(value.to_string())
    }
}
