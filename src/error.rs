use std::fmt::Display;

/// Error used with crate utility functions
///
/// Most function (except Axum handlers) will utilize this Error.
/// These errors are handled within each handler, however, not returned to client
#[derive(Debug)]
pub enum Error {
    InvalidRequest(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidRequest(msg) => write!(f, "invalid request: {}", msg),
        }
    }
}
