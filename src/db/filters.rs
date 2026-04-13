//! # Database Filter Module
//!
//! This module contains types and functions used for database filters

mod date_range;
mod sort;

pub use date_range::{DateBound, DateFilter};
pub use sort::{TagSort, TaskSort};

/// SQL Comparison Type
///
/// This represents all comparison types in SQL
///
/// Each have a mapping to SQL equivalent string
pub enum SQLCmp {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Exists,
    NotExists,
}

impl std::fmt::Display for SQLCmp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SQLCmp::Equal => write!(f, "="),
            SQLCmp::NotEqual => write!(f, "<>"),
            SQLCmp::LessThan => write!(f, "<"),
            SQLCmp::LessThanEqual => write!(f, "<="),
            SQLCmp::GreaterThan => write!(f, ">"),
            SQLCmp::GreaterThanEqual => write!(f, ">="),
            SQLCmp::Exists => write!(f, " IS NOT NULL"),
            SQLCmp::NotExists => write!(f, " IS NULL"),
        }
    }
}
