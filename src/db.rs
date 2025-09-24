use std::str::FromStr;
use deadpool_postgres::{Manager, ManagerConfig, Object, Pool};
use tokio_postgres::{Config, NoTls};

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct DatabaseConn {
    pool: Pool,
}

impl DatabaseConn {
    pub fn connect(
        url: &str
    ) -> Result<Self> {
        let pg_config = Config::from_str(url)?;

        let manager = Manager::from_config(pg_config, NoTls, ManagerConfig::default());

        let pool = Pool::builder(manager)
            .max_size(16)
            .build()
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self { pool })
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
