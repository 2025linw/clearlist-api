//! # Route Tag Model
//!
//! This module contains types used for tag queries and filters

use serde::Deserialize;

use super::{Pagination, SortOrder};

/// Tag Request Model
///
/// This represents the fields a client is able to create/modify for a Tag
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct Model {
    #[serde(default)]
    pub label: String,
    pub category: Option<String>,
}

/// Tag Filter Model
///
/// This represents the url parameter fields for filtering Tags queried
#[derive(Debug, Deserialize)]
pub struct Filter {
    #[serde(flatten)]
    pub pagination: Pagination,

    #[serde(default)]
    #[serde(rename = "sort")]
    pub sort_by: SortBy,
    #[serde(default)]
    #[serde(rename = "order")]
    pub sort_order: SortOrder,
}

/// Tag Sort Type
///
/// This represents all the fields that it is possible to sort Tags by
#[derive(Debug, Default, Deserialize)]
pub enum SortBy {
    Created,
    #[default]
    Updated,
    Label,
}
