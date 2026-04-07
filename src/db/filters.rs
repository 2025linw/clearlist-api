//! # Database Filter Module
//!
//! This module contains types and functions used for database filters

mod date_range;

pub use date_range::{DateBound, DateFilter};

// TODO: add Exists and NotExists
pub enum SQLCmp {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
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
        }
    }
}

#[derive(Default)]
pub enum SortOrder {
    #[default]
    UpdatedDesc,
    UpdatedAsc,
    CreatedDesc,
    CreatedAsc,
}
