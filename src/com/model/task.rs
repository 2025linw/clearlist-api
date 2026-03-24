use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::{
    Tag,
    query::{DateFilter, Pagination},
};

#[derive(Debug, Deserialize, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    // TODO: when performing schema validation, check that only one of start_date OR start_at exists.
    // Try to combine into untagged enum with Start::On and Start::At
    pub start_date: Option<chrono::NaiveDate>,
    pub start_at: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::NaiveDate>,

    #[serde(skip_deserializing)]
    pub deleted_at: Option<chrono::DateTime<Utc>>,

    #[serde(skip_deserializing)]
    pub created_at: chrono::DateTime<Utc>,
    #[serde(skip_deserializing)]
    pub updated_at: chrono::DateTime<Utc>,

    #[serde(skip_deserializing)]
    pub created_by: uuid::Uuid,

    #[serde(default)]
    #[sqlx(skip)]
    pub tags: Vec<Tag>,
}

#[derive(Debug, FromRow)]
pub struct TaskTag {
    pub task_id: uuid::Uuid,

    #[sqlx(flatten)]
    pub tag: Tag,
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    pub start_date: Option<DateFilter>,
    pub deadline: Option<DateFilter>,

    #[serde(default)]
    pub deleted: bool,
}
