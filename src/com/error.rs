use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    DateRangeConversion(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DateRangeConversion(msg) => write!(f, "error converting date range: {}", msg),
        }
    }
}

impl StdError for Error {}
