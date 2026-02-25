pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    DatabasePool(String),
    DatabaseOp(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DatabasePool(msg) => write!(f, "database pool error: {}", msg),
            Error::DatabaseOp(msg) => write!(f, "database operation error: {}", msg),
        }
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(value: deadpool_postgres::PoolError) -> Self {
        Self::DatabasePool(value.to_string())
    }
}

impl From<tokio_postgres::Error> for Error {
    fn from(value: tokio_postgres::Error) -> Self {
        if let Some(db_error) = value.as_db_error() {
            Self::DatabaseOp(db_error.message().to_string())
        } else {
            Self::DatabaseOp(value.to_string())
        }
    }
}
