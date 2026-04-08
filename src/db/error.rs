//! # Database Error Module
//!
//! This module contains Error used across all database functions

use std::error::Error as StdError;
use std::fmt::Display;

use crate::com::constants::{TAG_NOT_FOUND, TASK_NOT_FOUND};

pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for database libraries and functions
///
/// This enum represents the different levels in which errors can occur
/// - `Connection`  -> Related to the connection into the database
/// - `Pool`        -> Related to getting a connection from the pool
/// - `Operation`   -> Related to a given database operation
/// - `Application` -> Related to application (business) logic correctness
#[derive(Debug)]
pub enum Error {
    /// Database connection error
    Connection(String),
    /// Database pool error
    Pool(String),
    /// Database operation error
    Operation(String),
    /// Application (Business rule) error
    Application(ApplicationError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Database connection error: {}", msg),
            Error::Pool(msg) => write!(f, "Database pool error: {}", msg),
            Error::Operation(msg) => write!(f, "Database operation error: {}", msg),
            Error::Application(msg) => write!(f, "Application integrity error: {}", msg),
        }
    }
}

impl StdError for Error {}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        match value {
            // errors associated with database connection
            sqlx::Error::Configuration(error) => Self::Connection(error.to_string()),
            sqlx::Error::Io(error) => Self::Connection(error.to_string()),
            sqlx::Error::Tls(error) => Self::Connection(error.to_string()),
            sqlx::Error::Protocol(msg) => Self::Connection(msg),

            // errors associated with database pool
            sqlx::Error::PoolTimedOut => {
                Self::Pool("Timed out acquiring connection from pool".to_string())
            }
            sqlx::Error::PoolClosed => {
                Self::Pool("Pool closed while waiting to acquire connection".to_string())
            }

            sqlx::Error::InvalidArgument(msg) => Self::Operation(msg),
            sqlx::Error::Database(database_error) => {
                if let Some(pg_code) = database_error.code() {
                    eprintln!(
                        "UNCAUGHT INTEGRITY/TRIGGER ERROR: NEED TO FIX: {}",
                        database_error.message()
                    );

                    if pg_code == "P0001" {
                        // if error is from a custom raise in a function or trigger function
                        return Self::Application(ApplicationError::UncaughtTrigger(
                            database_error.message().to_string(),
                        ));
                    } else if pg_code.starts_with("23") {
                        // if error is related to integrity constraints
                        return Self::Application(ApplicationError::UncaughtIntegrity(
                            database_error.message().to_string(),
                        ));
                    }
                }

                Self::Operation(database_error.message().to_string())
            }

            // errors associated with database operation
            sqlx::Error::RowNotFound => {
                Self::Operation("No rows returned from query expecting return".to_string())
            }
            sqlx::Error::TypeNotFound { type_name } => {
                Self::Operation(format!("Type in query not found: '{}'", type_name))
            }
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => Self::Operation(format!(
                "Column index {} out of bounds (length: {})",
                index, len
            )),
            sqlx::Error::ColumnNotFound(column) => {
                Self::Operation(format!("Column '{}' not found", column))
            }
            sqlx::Error::ColumnDecode { index, source } => {
                Self::Operation(format!("Unable to decode column '{}': {}", index, source))
            }
            sqlx::Error::Encode(error) => {
                Self::Operation(format!("Error encoding value: {}", error))
            }
            sqlx::Error::Decode(error) => {
                Self::Operation(format!("Error decoding value: {}", error))
            }
            sqlx::Error::BeginFailed => {
                Self::Operation("Error beginning database transaction".to_string())
            }
            sqlx::Error::InvalidSavePointStatement => {
                Self::Operation("Error with savepoint statement".to_string())
            }

            // errors associated with miscellaneous database operations
            sqlx::Error::AnyDriverError(error) => Self::Operation(format!(
                "Error mapping between Any driver and Postgres driver: {}",
                error
            )),
            sqlx::Error::WorkerCrashed => {
                Self::Operation("Database background worker crashed".to_string())
            }
            sqlx::Error::Migrate(migrate_error) => Self::Operation(migrate_error.to_string()),

            unknown => panic!("Found uncaught error from database: {}", unknown),
        }
    }
}

/// Application (Business) Error Type
///
/// This enum represents the different types of application (business) logic errors
#[derive(Debug)]
pub enum ApplicationError {
    TaskNotFound,
    TagNotFound,
    InvalidDateRange(String),
    UncaughtIntegrity(String),
    UncaughtTrigger(String),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationError::TaskNotFound => write!(f, "{}", TASK_NOT_FOUND),
            ApplicationError::TagNotFound => write!(f, "{}", TAG_NOT_FOUND),
            ApplicationError::InvalidDateRange(msg) => write!(f, "Invalid date range: {}", msg),
            ApplicationError::UncaughtIntegrity(msg) => {
                write!(f, "Uncaught integrity error: {}", msg)
            }
            ApplicationError::UncaughtTrigger(msg) => write!(f, "Uncaught trigger error: {}", msg),
        }
    }
}
