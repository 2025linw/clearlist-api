pub mod sql_builder;

pub use sql_builder::*;

use deadpool_postgres::{Manager, ManagerConfig, Pool, PoolError};
use tokio_postgres::{Config, NoTls};

pub async fn get_database_pool(
    host: String,
    port: u16,
    database: String,
    user: String,
    pass: String,
) -> Result<Pool, PoolError> {
    let mut pg_config = Config::new();
    pg_config.host(host).port(port);
    pg_config.user(user).password(pass).dbname(database);

    let manager = Manager::from_config(pg_config, NoTls, ManagerConfig::default());

    let pool = Pool::builder(manager).max_size(16).build().unwrap();
    let _ = pool.get().await?;

    Ok(pool)
}
