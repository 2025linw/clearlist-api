use deadpool_postgres::{Manager, ManagerConfig, Object, Pool};
use tokio_postgres::{Config, NoTls};

use crate::error::{Error, Result};

#[derive(Clone)]
pub struct DatabaseConn {
    pool: Pool,
}

impl DatabaseConn {
    pub fn connect(
        host: String,
        port: u16,
        database: String,
        user: String,
        pass: String,
    ) -> Result<Self> {
        let mut pg_config = Config::new();
        pg_config.host(host).port(port);
        pg_config.user(user).password(pass).dbname(database);

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
