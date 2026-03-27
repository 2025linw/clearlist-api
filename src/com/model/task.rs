use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::com::model::{
    query::{SortBy, SortOrder},
    util::Start,
};

use super::{
    Tag,
    query::{DateFilter, Pagination},
};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub start: Start,
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
    pub tags: Vec<Tag>,
}

#[derive(FromRow)]
pub struct TaskIntermediate {
    pub id: uuid::Uuid,

    pub title: String,
    pub notes: Option<String>,
    pub start_on: Option<chrono::NaiveDate>,
    pub start_at: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::NaiveDate>,

    pub deleted_at: Option<chrono::DateTime<Utc>>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,

    pub created_by: uuid::Uuid,

    #[sqlx(skip)]
    pub tags: Vec<Tag>,
}

impl From<TaskIntermediate> for Task {
    fn from(value: TaskIntermediate) -> Self {
        let start = match (value.start_on, value.start_at) {
            (Some(date), None) => Start::On(date),
            (None, Some(datetime)) => Start::At(datetime),
            (None, None) => Start::None,
            (Some(_), Some(_)) => panic!(
                "unexpected values for start_on and start_at when converting intermediate task"
            ),
        };

        Self {
            id: value.id,
            title: value.title,
            notes: value.notes,
            start,
            deadline: value.deadline,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
            created_by: value.created_by,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    #[serde(default)]
    #[serde(rename = "sort")]
    pub sort_by: SortBy,
    #[serde(default)]
    #[serde(rename = "order")]
    pub sort_order: SortOrder,

    pub start_date: Option<DateFilter>,
    pub deadline: Option<DateFilter>,

    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, FromRow)]
pub struct TaskTag {
    pub task_id: uuid::Uuid,

    #[sqlx(flatten)]
    pub tag: Tag,
}
