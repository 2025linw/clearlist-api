mod error;

pub mod tag;
pub mod task;

pub use error::{Error, Result};

use std::{env, str::FromStr};

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool};
use dotenvy::dotenv;
use tokio_postgres::{Config, NoTls};

const MAX_SIZE: usize = 20;

#[derive(Clone)]
pub struct DatabaseConn {
    pool: Pool,
}

impl DatabaseConn {
    pub fn connect(user: &str, pass: &str, host: &str, port: u16, base: &str) -> Result<Self> {
        let mut pg_config = Config::new();
        pg_config
            .user(user)
            .password(pass)
            .host(host)
            .port(port)
            .dbname(base);
        let manager = Manager::from_config(pg_config, NoTls, ManagerConfig::default());

        let pool = Pool::builder(manager)
            .max_size(MAX_SIZE)
            .build()
            .map_err(|e| Error::DatabaseOp(e.to_string()))?;

        Ok(Self { pool })
    }

    pub fn connect_str(url: &str) -> Result<Self> {
        let pg_config = Config::from_str(url)?;
        let manager = Manager::from_config(pg_config, NoTls, ManagerConfig::default());

        let pool = Pool::builder(manager)
            .max_size(MAX_SIZE)
            .build()
            .map_err(|e| Error::DatabaseOp(e.to_string()))?;

        Ok(Self { pool })
    }

    pub fn connect_env() -> Result<Self> {
        dotenv().map_err(|e| Error::DatabaseOp(e.to_string()))?;

        if env::var("DATABASE_URL").is_ok() {
            Self::connect_str(&env::var("DATABASE_URL").unwrap())
        } else {
            Err(Error::DatabaseOp("DATABASE_URL not found".to_string()))
        }
    }

    pub async fn is_active(&self) -> bool {
        if self.pool.is_closed() {
            return false;
        }

        // Check if we can get an connection from connection pool
        let conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => {
                return false;
            }
        };

        // Check if we can make a query
        if conn.query_one("SELECT 1", &[]).await.is_err() {
            return false;
        }

        true
    }

    /// Get connection from pool
    pub async fn get_conn(&self) -> Result<Object> {
        Ok(self.pool.get().await?)
    }
}
