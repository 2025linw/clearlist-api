use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::com::model::query::{SortBy, SortOrder};

use super::query::Pagination;

#[derive(Debug, Deserialize, Serialize, FromRow)]
#[cfg_attr(test, derive(Default, Clone, PartialEq))]
pub struct Tag {
    // primary key - not specifiable by user
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    // values that are selected by user
    #[serde(default)]
    pub label: String,
    pub category: Option<String>,

    // values associated with other functions/actions
    // None

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

    #[serde(default)]
    #[serde(rename = "sort")]
    pub sort_by: SortBy,
    #[serde(default)]
    #[serde(rename = "order")]
    pub sort_order: SortOrder,
}
// TODO: when adding sort order control to TagQuery, we should return InvalidRequest if user tries to sort by a value that doesn't exist
// Such as start date or deadline on Tag
