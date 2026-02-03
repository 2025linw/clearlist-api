mod area;
mod project;
mod tag;
pub mod task;

mod utils;

use dotenvy::dotenv;
use std::{env, str::FromStr};

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool};
use tokio_postgres::{Config, NoTls};

use crate::error::{Error, Result};

const MAX_SIZE: usize = 128;

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
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self { pool })
    }

    pub fn connect_str(url: &str) -> Result<Self> {
        let pg_config = Config::from_str(url)?;
        let manager = Manager::from_config(pg_config, NoTls, ManagerConfig::default());

        let pool = Pool::builder(manager)
            .max_size(MAX_SIZE)
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self { pool })
    }

    pub fn connect_env() -> Result<Self> {
        dotenv().map_err(|e| Error::Internal(e.to_string()))?;

        if env::var("DATABASE_URL").is_ok() {
            Self::connect_str(&env::var("DATABASE_URL").unwrap())
        } else if env::var("DB_USER").is_ok() {
            Self::connect(
                &env::var("DB_USER").map_err(|e| {
                    Error::Internal("DB_USER not in environment variables".to_string())
                })?,
                &env::var("DB_PASS").map_err(|e| {
                    Error::Internal("DB_PASS not in environment variables".to_string())
                })?,
                &env::var("DB_HOST").map_err(|e| {
                    Error::Internal("DB_HOST not in environment variables".to_string())
                })?,
                env::var("DB_PORT")
                    .map_err(|_| {
                        Error::Internal("DB_PORT not in environment variables".to_string())
                    })?
                    .parse::<u16>()
                    .map_err(|_| Error::Internal("DB_PORT is not a number".to_string()))?,
                &env::var("DB_NAME").unwrap(),
            )
        } else {
            Err(Error::Internal(
                "No environment variables initialized".to_string(),
            ))
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
