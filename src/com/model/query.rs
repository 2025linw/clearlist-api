use chrono::NaiveDate;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

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
pub struct QueryDateFilter {
    pub lt: Option<NaiveDate>,
    pub gt: Option<NaiveDate>,
    pub gte: Option<NaiveDate>,
    pub lte: Option<NaiveDate>,
}
