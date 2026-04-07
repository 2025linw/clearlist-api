//! Task Model
//!
//! This module contains the task model

use chrono::Utc;
use sqlx::FromRow;

use crate::models::tag::Tag;

#[allow(dead_code)]
#[derive(Debug, FromRow)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct Task {
    pub id: uuid::Uuid,

    pub title: String,
    pub notes: Option<String>,
    pub start_on: Option<chrono::NaiveDate>,
    pub start_at: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::NaiveDate>,
    #[sqlx(skip)]
    pub tags: Vec<Tag>,

    pub completed_at: Option<chrono::DateTime<Utc>>,
    pub deleted_at: Option<chrono::DateTime<Utc>>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,

    pub created_by: uuid::Uuid,
}

#[derive(Debug, FromRow)]
pub struct TaskTag {
    pub task_id: uuid::Uuid,

    #[sqlx(flatten)]
    pub tag: Tag,
}
