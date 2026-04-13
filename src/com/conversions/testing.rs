//! Testing Conversions
//!
//! This module contains type conversions only used in tests
//!
//! They are not supported or useful operations in normal operation
//!
//! Conversions:
//! * from model-level Task to user request Task
//! * From model-level Tag to user request Tag

use crate::{
    models::{Tag, Task},
    routes::models::{Start, tag::Model as TagCreate, task::Model as TaskCreate},
};

impl From<Task> for TaskCreate {
    fn from(value: Task) -> Self {
        let start = if let Some(dt) = value.start_dt {
            if value.has_time {
                Some(Start::At(dt))
            } else {
                Some(Start::On(dt.date_naive()))
            }
        } else {
            None
        };

        Self {
            title: value.title,
            notes: value.notes,
            start,
            deadline: value.deadline,
            tags: value.tags.iter().map(|tag| tag.id).collect(),
        }
    }
}

impl From<Tag> for TagCreate {
    fn from(value: Tag) -> Self {
        Self {
            label: value.label,
            category: value.category,
        }
    }
}
