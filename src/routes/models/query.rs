//! Route Query Models
//!
//! This module contains types used for various queries and filters

use std::borrow::Cow;

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, de::Error};
use serde_with::{DisplayFromStr, serde_as};

use crate::com::constants::DEFAULT_LIMIT;

/// Pagination Type
///
/// This represents the url query fields for `page` and `limit`
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Pagination {
    #[serde_as(as = "DisplayFromStr")]
    pub page: i64,
    #[serde_as(as = "DisplayFromStr")]
    pub limit: i64,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            limit: DEFAULT_LIMIT,
        }
    }
}

/// Date Filter Type
///
/// This represents the url query values for date filtering
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DateFilter {
    /// Existence Filter
    #[serde(deserialize_with = "deserialize_bool")]
    Has(bool),

    /// Exact Filter, has to match exactly
    Exact(NaiveDate),

    /// Filter created by complex queries
    BracketInterval(BracketInterval),

    /// Filter created by ISO8601 Interval format (<start>/<end>)
    #[serde(deserialize_with = "deserialize_iso8601daterange")]
    ISO8601Interval(ISO8601Interval),
}

/// Bracket Interval Deserialization Type
///
/// This allows conversion of complex url query parameters (i.e. `param[cmp]=value`) with date as value and comparison operators as the complex parameters
#[derive(Debug, Deserialize)]
pub struct BracketInterval {
    #[serde(alias = "<>")]
    pub(crate) ne: Option<NaiveDate>,
    #[serde(alias = "<")]
    pub(crate) lt: Option<NaiveDate>,
    pub(crate) lte: Option<NaiveDate>,
    #[serde(alias = ">")]
    pub(crate) gt: Option<NaiveDate>,
    pub(crate) gte: Option<NaiveDate>,
}

/// ISO8601 Interval Deserialization Type
///
/// This allows conversion of url queries that have a ISO8601 Interval as a value (<start>/<end>)
pub type ISO8601Interval = [NaiveDate; 2];

/// Sort Order Type
///
/// This represents the url query for the order which to sort fields by
#[derive(Debug, Default, Deserialize)]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[default]
    #[serde(rename = "desc")]
    Descending,
}

/// Completed Body Type
///
/// This represents the body format for task complete route
#[derive(Debug, Deserialize)]
pub struct Completed {
    pub completed: bool,
}

/// Internel serde helper to deserialize iso8601 date ranges
///
/// Deserialize '<start>/<end>` into `[NaiveDate, NaiveDate]`
fn deserialize_iso8601daterange<'de, D>(deserialize: D) -> Result<[NaiveDate; 2], D::Error>
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

/// Internal serde helper to deserialze bool string into bool type
///
/// Deserialize 'true' or 'false' into `true` or `false`
fn deserialize_bool<'de, D>(deserialize: D) -> Result<bool, D::Error>
where
D: Deserializer<'de>,
{
    let s: Cow<'_, str> = Deserialize::deserialize(deserialize)?;

    if s.trim().is_empty() {
        return Err(D::Error::custom("Value cannot be empty"));
    }

    let bool = s.parse::<bool>().map_err(|_| D::Error::custom("Invalid bool value"))?;

    Ok(bool)
}
