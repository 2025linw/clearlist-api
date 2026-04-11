//! # Route Task Model
//!
//! This module contains types used for querying and filtering

use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use super::{DateFilter, Pagination, SortBy, SortOrder, Start};

/// Task Request Model
///
/// This represents the fields a client is able to create/modify for a Task
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct Task {
    #[serde(default)]
    pub title: String,
    pub notes: Option<String>,
    pub start: Option<Start>,
    pub deadline: Option<NaiveDate>,
    #[serde(default)]
    pub tags: Vec<Uuid>,
}

/// Task Filter Model
///
/// This represents the url parameter fields for filtering Tasks queried
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

    #[serde(alias = "start")]
    pub start_date: Option<DateFilter>,
    #[serde(alias = "due")]
    pub deadline: Option<DateFilter>,

    #[serde(default)]
    #[serde(alias = "logged")]
    #[serde(alias = "done")]
    pub completed: bool,
    #[serde(default)]
    pub deleted: bool,
}

/// Task Tag Query Path
///
/// This extracts the task_id and tag_id from the url path when calling the add and remove tag functions for Tasks
#[derive(Debug, Deserialize)]
pub struct TaskTagQuery {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}
