use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::query::Pagination;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Tag {
    #[serde(default)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub label: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TagQuery {
    #[serde(flatten)]
    pub pagination: Pagination,
}
