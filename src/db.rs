mod error;

pub mod tag;
pub mod task;

pub use error::{Error, Result};
use uuid::Uuid;

use std::env;

use sqlx::{
    Database, FromRow, PgPool, Postgres,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    query::QueryAs,
};

const MAX_CONNECTIONS: u32 = 20;

#[derive(Clone)]
pub struct DatabaseConn {
    pool: PgPool,
}

impl DatabaseConn {
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

    pub async fn connect_str(url: &str) -> Result<Self> {
        let pg_pool_config = PgPoolOptions::new().max_connections(MAX_CONNECTIONS);

        let pool = pg_pool_config.connect(url).await?;

        Ok(Self { pool })
    }

    pub async fn connect_env() -> Result<Self> {
        if env::var("DATABASE_URL").is_ok() {
            Self::connect_str(&env::var("DATABASE_URL").unwrap()).await
        } else {
            Err(Error::Operation("DATABASE_URL not found".to_string()))
        }
    }

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
    pub fn get_pool_ref(&self) -> &PgPool {
        &self.pool
    }
}

fn query_as_wrapper<'q, T>(
    sql: &'q str,
) -> QueryAs<'q, Postgres, T, <Postgres as Database>::Arguments<'q>>
where
    T: for<'r> FromRow<'r, <Postgres as Database>::Row>,
{
    sqlx::query_as(sql)
}

pub async fn is_task_exists(conn: &PgPool, task_id: Uuid, user_id: String) -> Result<bool> {
    if sqlx::query("SELECT * FROM app.tasks WHERE id = $1 AND created_by = $2")
        .bind(task_id)
        .bind(user_id)
        .execute(conn)
        .await?
        .rows_affected()
        == 0
    {
        Ok(false)
    } else {
        Ok(true)
    }
}
