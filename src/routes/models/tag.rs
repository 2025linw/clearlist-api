//! # Route Tag Model
//!
//! This module contains types used for tag queries and filters

use serde::Deserialize;

use super::{Pagination, SortBy, SortOrder};

/// Tag Request Model
///
/// This represents the fields a client is able to create/modify for a Tag
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default, Clone))]
pub struct Tag {
    #[serde(default)]
    pub label: String,
    pub category: Option<String>,
}

/// Tag Filter Model
///
/// This represents the url parameter fields for filtering Tags queried
#[derive(Debug, Deserialize)]
pub struct TagFilter {
    #[serde(flatten)]
    pub pagination: Pagination,

    #[serde(default)]
    #[serde(rename = "sort")]
    pub sort_by: SortBy,
    #[serde(default)]
    #[serde(rename = "order")]
    pub sort_order: SortOrder,
}
