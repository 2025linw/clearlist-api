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

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct Task {
    // primary key - not specifiable by user
    #[serde(skip_deserializing)]
    pub id: uuid::Uuid,

    // values that are specified by user
    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    pub start: Option<Start>,
    pub deadline: Option<chrono::NaiveDate>,
    #[serde(default)]
    pub tags: Vec<Tag>,

    // values associated with other functions/actions (not directly specifiable)
    #[serde(skip_deserializing)]
    pub completed_at: Option<chrono::DateTime<Utc>>,
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

#[derive(FromRow)]
pub struct TaskIntermediate {
    pub id: uuid::Uuid,

    pub title: String,
    pub notes: Option<String>,
    pub start_on: Option<chrono::NaiveDate>,
    pub start_at: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::NaiveDate>,

    pub completed_at: Option<chrono::DateTime<Utc>>,
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
            (Some(date), None) => Some(Start::On(date)),
            (None, Some(datetime)) => Some(Start::At(datetime)),
            (None, None) => None,
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
            completed_at: value.completed_at,
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
    pub completed: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, FromRow)]
pub struct TaskTagIntermediate {
    pub task_id: uuid::Uuid,

    #[sqlx(flatten)]
    pub tag: Tag,
}
