use std::error::Error as StdError;
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// Error that occurs with the database connection
    Connection(String),
    /// Error that occurs with the database pool
    Pool(String),
    /// Error that occurs with normal database operation
    Operation(String),
    /// Error that breaks application integrity (Application error)
    Application(ApplicationError),
    /// Unknown error with database
    Miscellaneous(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "database connection error: {}", msg),
            Error::Pool(msg) => write!(f, "database pool error: {}", msg),
            Error::Operation(msg) => write!(f, "database operation error: {}", msg),
            Error::Application(msg) => write!(f, "application integrity error: {}", msg),
            Error::Miscellaneous(msg) => write!(f, "miscellaneous database error: {}", msg),
        }
    }
}

impl StdError for Error {}

#[derive(Debug)]
pub enum ApplicationError {
    TaskNotFound,
    TagNotFound,

    /// WARN: do not use, prioritize better management of this
    Misc(String),
}

impl Display for ApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationError::TaskNotFound => write!(f, "task not found"),
            ApplicationError::TagNotFound => write!(f, "tag not found"),
            ApplicationError::Misc(msg) => write!(f, "unknown application error: {}", msg),
        }
    }
}

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
                Self::Pool("timed out acquiring connection from pool".to_string())
            }
            sqlx::Error::PoolClosed => {
                Self::Pool("pool closed while waiting to acquire connection".to_string())
            }

            sqlx::Error::InvalidArgument(msg) => Self::Operation(msg),
            sqlx::Error::Database(database_error) => {
                if let Some(pg_code) = database_error.code() {
                    eprintln!("UNCAUGHT INTEGRITY/TRIGGER ERROR: NEED TO FIX");

                    if pg_code == "P0001" {
                        // if error is from a custom raise in a function or trigger function
                        return Self::Application(ApplicationError::Misc(
                            database_error.message().to_string(),
                        ));
                    } else if pg_code.starts_with("23") {
                        // if error is related to integrity constraints
                        return Self::Application(ApplicationError::Misc(
                            database_error.message().to_string(),
                        ));
                    }
                }

                Self::Operation(database_error.message().to_string())
            }

            // errors associated with database operation
            sqlx::Error::RowNotFound => {
                Self::Operation("no rows returned from query expecting return".to_string())
            }
            sqlx::Error::TypeNotFound { type_name } => {
                Self::Operation(format!("type in query not found: '{}'", type_name))
            }
            sqlx::Error::ColumnIndexOutOfBounds { index, len } => Self::Operation(format!(
                "column index {} out of bounds (length: {})",
                index, len
            )),
            sqlx::Error::ColumnNotFound(column) => {
                Self::Operation(format!("column '{}' not found", column))
            }
            sqlx::Error::ColumnDecode { index, source } => {
                Self::Operation(format!("unable to decode column '{}': {}", index, source))
            }
            sqlx::Error::Encode(error) => {
                Self::Operation(format!("error encoding value: {}", error))
            }
            sqlx::Error::Decode(error) => {
                Self::Operation(format!("error decoding value: {}", error))
            }
            sqlx::Error::BeginFailed => {
                Self::Operation("error beginning database transaction".to_string())
            }
            sqlx::Error::InvalidSavePointStatement => {
                Self::Operation("error with savepoint statement".to_string())
            }

            // errors associated with miscellaneous database operations
            sqlx::Error::AnyDriverError(error) => Self::Miscellaneous(format!(
                "error mapping between Any driver and Postgres driver: {}",
                error
            )),
            sqlx::Error::WorkerCrashed => {
                Self::Miscellaneous("database background worker crashed".to_string())
            }
            sqlx::Error::Migrate(migrate_error) => Self::Miscellaneous(migrate_error.to_string()),

            unknown => panic!("found uncaught error from database: {}", unknown),
        }
    }
}
