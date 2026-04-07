//! # Route Task Model
//!
//! This module contains types used for task queries and filters

use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use super::{DateFilter, Pagination, SortBy, SortOrder, Start};

#[derive(Deserialize)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct Task {
    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    pub start: Option<Start>,
    pub deadline: Option<NaiveDate>,
    pub tags: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct TaskFilter {
    #[serde(flatten)]
    pub pagination: Pagination,

    #[serde(default)]
    #[serde(rename = "sort")]
    pub sort_by: SortBy,
    #[serde(default)]
    #[serde(rename = "order")]
    pub sort_order: SortOrder,

    pub start_date: Option<DateFilter>,
    pub deadline: Option<DateFilter>,

    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Deserialize)]
pub struct TaskTagQuery {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}
