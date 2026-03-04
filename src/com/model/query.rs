use chrono::NaiveDate;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

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

    #[serde(deserialize_with = "deserialize_daterange")]
    Range([NaiveDate; 2]),
}
