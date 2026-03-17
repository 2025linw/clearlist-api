use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::query::Pagination;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Tag {
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub label: String,
    pub category: Option<String>,

    #[serde(skip_deserializing)]
    pub deleted_at: Option<chrono::DateTime<Utc>>,

    #[serde(skip_deserializing)]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: chrono::DateTime<Utc>,

    #[serde(skip_deserializing)]
    pub created_by: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct TagQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    pub deleted: bool,
}
