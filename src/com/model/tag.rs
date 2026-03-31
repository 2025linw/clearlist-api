use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::query::Pagination;

#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
#[cfg_attr(test, derive(Default, PartialEq))]
pub struct Tag {
    // primary key - not specifiable by user
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    // values that are selected by user
    #[serde(default)]
    pub label: String,
    pub category: Option<String>,

    // values associated with other functions/actions
    #[serde(skip_deserializing)]
    pub deleted_at: Option<chrono::DateTime<Utc>>,

    // values that are created automatically by database
    #[serde(skip_deserializing)]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: chrono::DateTime<Utc>,

    // values that are innate to object
    #[serde(skip_deserializing)]
    pub created_by: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct TagQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    pub deleted: bool,
}
// TODO: when adding sort order control to TagQuery, we should return InvalidRequest if user tries to sort by a value that doesn't exist
// Such as start date or deadline on Tag
