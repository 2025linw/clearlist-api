//! # Route Task Model
//!
//! This module contains types used for querying and filtering

use chrono::NaiveDate;
use serde::Deserialize;
use uuid::Uuid;

use super::{DateFilter, Pagination, SortOrder, Start};

/// Task Request Model
///
/// This represents the fields a client is able to create/modify for a Task
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default, Clone))]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Model {
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
#[serde(deny_unknown_fields)]
pub struct Filter {
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
    #[serde(alias = "trash")]
    pub deleted: bool,
}

/// Task Sort Type
///
/// This represents all the values that it is possible to sort Tasks by
#[derive(Debug, Default, Deserialize)]
pub enum SortBy {
    Created,
    #[default]
    Updated,
    Title,
    Start,
    #[serde(alias = "due")]
    Deadline,
}
