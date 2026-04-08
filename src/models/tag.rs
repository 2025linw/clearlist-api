//! Tag Model
//!
//! This module contains the tag model

use chrono::Utc;
use sqlx::FromRow;

/// Tag Model Type
///
/// This is the ground truth model for tag; it exactly matches database schema
#[allow(dead_code)]
#[derive(Debug, FromRow)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct Tag {
    pub id: uuid::Uuid,

    pub label: String,
    pub category: Option<String>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,

    pub created_by: uuid::Uuid,
}
