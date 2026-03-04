use serde::{Deserialize, Serialize};

use super::{
    Tag,
    query::{DateFilter, Pagination},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub notes: String,
    pub start_date: Option<chrono::NaiveDate>,
    pub start_time: Option<chrono::NaiveTime>,
    pub deadline: Option<chrono::NaiveDate>,

    #[serde(default)]
    pub tags: Option<Vec<Tag>>,
}

impl From<tokio_postgres::Row> for Task {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get("id"),
            title: value.get("title"),
            notes: value.get("notes"),
            start_date: value.get("start_date"),
            start_time: value.get("start_time"),
            deadline: value.get("deadline"),
            tags: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    pub start: Option<DateFilter>,
    pub deadline: Option<DateFilter>,
}
