//! Route Query Models
//!
//! This module contains types used for various queries and filters

use std::borrow::Cow;

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, de::Error};
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
#[serde(untagged)]
pub enum DateFilter {
    Exact(NaiveDate),

    BracketInterval(BracketInterval),

    #[serde(deserialize_with = "deserialize_iso8601daterange")]
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
    OldestFirst,
    #[default]
    #[serde(rename = "desc")]
    NewestFirst,
}

#[derive(Debug, Deserialize)]
pub struct Completed {
    pub completed: bool,
}

pub fn deserialize_iso8601daterange<'de, D>(deserialize: D) -> Result<[NaiveDate; 2], D::Error>
where
    D: Deserializer<'de>,
{
    let s: Cow<'_, str> = Deserialize::deserialize(deserialize)?;

    if s.trim().is_empty() {
        return Err(D::Error::custom("Date range cannot be empty"));
    }

    let mut parts = s.split('/');

    let start = parts
        .next()
        .ok_or_else(|| D::Error::custom("Missing start date"))?
        .parse::<NaiveDate>()
        .map_err(D::Error::custom)?;

    let end = parts
        .next()
        .ok_or_else(|| D::Error::custom("Missing end date"))?
        .parse::<NaiveDate>()
        .map_err(D::Error::custom)?;

    if parts.next().is_some() {
        return Err(D::Error::custom("Found too many dates in range"));
    }

    Ok([start, end])
}
