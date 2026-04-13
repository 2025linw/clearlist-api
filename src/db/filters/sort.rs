//! Database Sort Filters
//!
//! This module contains the sorts used in the database-level for sorting Task and Tag queries

use crate::routes::models::SortOrder;

/// Task Sort Type
///
/// This represents all the values that it is possible to sort Tasks by
#[derive(Debug)]
pub enum TaskSort {
    Created(SortOrder),
    Updated(SortOrder),
    Title(SortOrder),
    Start(SortOrder),
    Deadline(SortOrder),
}

impl Default for TaskSort {
    fn default() -> Self {
        Self::Updated(SortOrder::Descending)
    }
}

/// Tag Sort Type
///
/// This represents all the fields that it is possible to sort Tasks by
#[derive(Debug)]
pub enum TagSort {
    Created(SortOrder),
    Updated(SortOrder),
    Label(SortOrder),
}

impl Default for TagSort {
    fn default() -> Self {
        Self::Label(SortOrder::Ascending)
    }
}
