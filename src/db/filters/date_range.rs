//! # Date Range Filter
//!
//! This module contains types used for database date filtering

use chrono::NaiveDate;

use super::SQLCmp;

/// Represents a specific start/end bound of a daterange
///
/// Options
/// - Exclusive: bound that excludes the endpoint
/// - Inclusive: bound that includes the endpoint
#[derive(Debug, PartialEq)]
pub enum DateBound {
    // TODO: convert from NaiveDate to DateTime<Utc>
    Exclusive(NaiveDate),
    Inclusive(NaiveDate),
}

/// Represents filters for dates
///
/// Options
/// - On: Filter exact date
/// - NotOn: Filter not on exact date
/// - StartRange: Filter after date
/// - EndRange: Filter before date
/// - Range: Filter between two dates
#[derive(Debug, PartialEq)]
pub enum DateFilter {
    Exists(bool),
    On(NaiveDate),
    NotOn(NaiveDate),
    StartRange(DateBound),
    EndRange(DateBound),
    Range(DateBound, DateBound),
}

impl DateFilter {
    pub fn into_sql(self) -> Vec<(SQLCmp, NaiveDate)> {
        match self {
            DateFilter::Exists(has) => {
                if has {
                    vec![(SQLCmp::Exists, NaiveDate::MIN)]
                } else {
                    vec![(SQLCmp::NotExists, NaiveDate::MIN)]
                }
            }
            DateFilter::On(date) => vec![(SQLCmp::Equal, date)],
            DateFilter::NotOn(date) => vec![(SQLCmp::NotEqual, date)],
            DateFilter::StartRange(bound) => match bound {
                DateBound::Exclusive(start_date) => vec![(SQLCmp::GreaterThan, start_date)],
                DateBound::Inclusive(start_date) => vec![(SQLCmp::GreaterThanEqual, start_date)],
            },
            DateFilter::EndRange(bound) => match bound {
                DateBound::Exclusive(end_date) => vec![(SQLCmp::LessThan, end_date)],
                DateBound::Inclusive(end_date) => vec![(SQLCmp::LessThanEqual, end_date)],
            },
            DateFilter::Range(start, end) => match (start, end) {
                (DateBound::Exclusive(start_date), DateBound::Exclusive(end_date)) => vec![
                    (SQLCmp::GreaterThan, start_date),
                    (SQLCmp::LessThan, end_date),
                ],
                (DateBound::Exclusive(start_date), DateBound::Inclusive(end_date)) => vec![
                    (SQLCmp::GreaterThan, start_date),
                    (SQLCmp::LessThanEqual, end_date),
                ],
                (DateBound::Inclusive(start_date), DateBound::Exclusive(end_date)) => vec![
                    (SQLCmp::GreaterThanEqual, start_date),
                    (SQLCmp::LessThan, end_date),
                ],
                (DateBound::Inclusive(start_date), DateBound::Inclusive(end_date)) => vec![
                    (SQLCmp::GreaterThanEqual, start_date),
                    (SQLCmp::LessThanEqual, end_date),
                ],
            },
        }
    }
}
