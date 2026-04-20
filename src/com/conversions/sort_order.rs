//! # Sort Filter Conversions
//!
//! This modules contains the conversion between route-and database-level query order types

use crate::{
    db::filters::{TagSort, TaskSort},
    routes::models::{SortOrder, tag::SortBy as TagSortBy, task::SortBy as TaskSortBy},
};

impl From<(TaskSortBy, SortOrder)> for TaskSort {
    fn from(value: (TaskSortBy, SortOrder)) -> Self {
        match value {
            (TaskSortBy::Created, order) => Self::Created(order),
            (TaskSortBy::Updated, order) => Self::Updated(order),
            (TaskSortBy::Title, order) => Self::Title(order),
            (TaskSortBy::Start, order) => Self::Start(order),
            (TaskSortBy::Deadline, order) => Self::Deadline(order),
        }
    }
}

impl From<(TagSortBy, SortOrder)> for TagSort {
    fn from(value: (TagSortBy, SortOrder)) -> Self {
        match value {
            (TagSortBy::Created, order) => Self::Created(order),
            (TagSortBy::Updated, order) => Self::Updated(order),
            (TagSortBy::Label, order) => Self::Label(order),
        }
    }
}
