use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::{
    Tag,
    query::{DateFilter, Pagination},
};

#[derive(Debug, Deserialize, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    #[serde(default)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub start_time: Option<chrono::NaiveTime>,
    pub deadline: Option<chrono::NaiveDate>,

    #[serde(default)]
    #[sqlx(skip)]
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    #[serde(flatten)]
    pub pagination: Pagination,

    pub start_date: Option<DateFilter>,
    pub deadline: Option<DateFilter>,
}
