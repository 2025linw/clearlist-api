//! # Sort Conversion
//!
//! This module contains the conversion between route- and database-level sort types

use crate::{
    db::filters::SortOrder,
    routes::models::{SortBy, SortOrder as SortOrderQuery},
};

impl From<(SortBy, SortOrderQuery)> for SortOrder {
    fn from(value: (SortBy, SortOrderQuery)) -> Self {
        match value {
            (SortBy::Updated, SortOrderQuery::NewestFirst) => Self::UpdatedDesc,
            (SortBy::Updated, SortOrderQuery::OldestFirst) => Self::UpdatedAsc,
            (SortBy::Created, SortOrderQuery::NewestFirst) => Self::CreatedDesc,
            (SortBy::Created, SortOrderQuery::OldestFirst) => Self::CreatedAsc,
        }
    }
}
