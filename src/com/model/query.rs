use chrono::NaiveDate;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use uuid::Uuid;

use crate::com::{model::db::SQLCmp, util::deserialize_daterange};

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
    ne: Option<NaiveDate>,
    lt: Option<NaiveDate>,
    gt: Option<NaiveDate>,
    lte: Option<NaiveDate>,
    gte: Option<NaiveDate>,
}

impl BracketInterval {
    pub fn is_valid(&self) -> bool {
        if self.lt.is_some() && self.lte.is_some() {
            return false;
        }

        if self.gt.is_some() && self.gte.is_some() {
            return false;
        }

        true
    }

    pub fn get_cmps(&self) -> Vec<(SQLCmp, NaiveDate)> {
        let mut cmps = Vec::new();

        if let Some(date) = self.ne {
            cmps.push((SQLCmp::NotEqual, date));
        }
        if let Some(date) = self.lt {
            cmps.push((SQLCmp::LessThan, date));
        }
        if let Some(date) = self.gt {
            cmps.push((SQLCmp::GreaterThan, date));
        }
        if let Some(date) = self.lte {
            cmps.push((SQLCmp::LessThanEqual, date));
        }
        if let Some(date) = self.gte {
            cmps.push((SQLCmp::GreaterThanEqual, date));
        }

        cmps
    }
}

pub type ISO8601Interval = [NaiveDate; 2];

#[derive(Debug, Deserialize)]
pub struct PathTaskTag {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}
