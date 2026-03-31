use chrono::NaiveDate;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use uuid::Uuid;

use crate::com::util::deserialize_daterange;

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Pagination {
    #[serde_as(as = "DisplayFromStr")]
    pub page: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub limit: u64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: 1, limit: 25 }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DateFilter {
    Exact(NaiveDate),

    BracketInterval(BracketInterval),

    #[serde(deserialize_with = "deserialize_daterange")]
    ISO8601Interval(ISO8601Interval),
}

#[derive(Debug, Deserialize)]
pub struct BracketInterval {
    pub(crate) ne: Option<NaiveDate>,
    pub(crate) gt: Option<NaiveDate>,
    pub(crate) gte: Option<NaiveDate>,
    pub(crate) lt: Option<NaiveDate>,
    pub(crate) lte: Option<NaiveDate>,
}

pub type ISO8601Interval = [NaiveDate; 2];

#[derive(Debug, Deserialize)]
pub struct TaskTag {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}

#[derive(Debug, Default, Deserialize)]
pub enum SortBy {
    #[serde(rename = "created")]
    Created,
    #[default]
    #[serde(rename = "updated")]
    Updated,
    // TODO: add deadline
    // TODO: add start date
}

#[derive(Debug, Default, Deserialize)]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[default]
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Debug, Deserialize)]
pub struct Completed {
    pub completed: bool,
}
