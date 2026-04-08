//! # Database Module
//!
//! `db` consists of all components of the database module

pub mod filters;
pub mod tag;
pub mod task;

mod error;

pub use error::{ApplicationError, Error, Result};

use std::env;

use sqlx::{
    FromRow, PgConnection, PgPool, Postgres, migrate,
    migrate::MigrateError,
    postgres::{PgArguments, PgConnectOptions, PgPoolOptions, PgRow, PgSslMode},
    query::QueryAs,
};

/// Maximum number of database connections in connection pool
const MAX_CONNECTIONS: u32 = 20;

/// Wrapper reusable database connection
///
/// This should be a connection pool to allow for concurrent use of pool by multiple handlers on threads.
#[derive(Clone)]
pub struct DatabaseConn {
    pool: PgPool,
}

impl DatabaseConn {
    /// Connect to database with parameters
    pub async fn connect(user: &str, pass: &str, host: &str, port: u16, db: &str) -> Result<Self> {
        let pg_config = PgConnectOptions::new()
            .username(user)
            .password(pass)
            .host(host)
            .port(port)
            .database(db)
            .ssl_mode(PgSslMode::Require);

        let pg_pool_config = PgPoolOptions::new().max_connections(MAX_CONNECTIONS);

        let pool = pg_pool_config.connect_with(pg_config).await?;

        Ok(Self { pool })
    }

    /// Connect to database using connection url
    pub async fn connect_str(url: &str) -> Result<Self> {
        let pg_pool_config = PgPoolOptions::new().max_connections(MAX_CONNECTIONS);

        let pool = pg_pool_config.connect(url).await?;

        Ok(Self { pool })
    }

    /// Connect to database using environment variables
    ///
    /// `DATABASE_URL` must be set in environment before utilizing this method
    pub async fn connect_env() -> Result<Self> {
        if env::var("DATABASE_URL").is_ok() {
            Self::connect_str(&env::var("DATABASE_URL").unwrap()).await
        } else {
            Err(Error::Operation("DATABASE_URL not found".to_string()))
        }
    }

    /// Checks if connection pool is still active
    pub async fn is_active(&self) -> bool {
        if self.pool.is_closed() {
            return false;
        }

        if sqlx::query("SELECT (1)").execute(&self.pool).await.is_err() {
            return false;
        }

        true
    }

    /// Get connection from pool
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

/// Function to run migrations
///
/// The migrations are embedded into the binary when building binary
///
/// Migrations must be in `./migrations` relative to Cargo.toml when building
pub async fn run_migration(conn: &mut PgConnection) -> std::result::Result<(), MigrateError> {
    migrate!().run(conn).await
}

/// Wrapper around `sqlx::query_as` which assumes Postgres as database
fn query_as_wrapper<'q, T>(sql: &'q str) -> QueryAs<'q, Postgres, T, PgArguments>
where
    T: for<'r> FromRow<'r, PgRow>,
{
    sqlx::query_as(sql)
}
