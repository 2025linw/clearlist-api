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
    routes::models::{Start, tag::Tag as TagCreate, task::Task as TaskCreate},
};

/// Conversion from Task model into user Task
impl From<Task> for TaskCreate {
    fn from(value: Task) -> Self {
        if value.start_on.is_some() && value.start_at.is_some() {
            // TODO: no panic
            panic!("have both start_on and start_at");
        }

        let start = if let Some(date) = value.start_on {
            use crate::routes::models::Start;

            Some(Start::On(date))
        } else {
            value.start_at.map(Start::At)
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

/// Conversion from Tag model into user Tag
impl From<Tag> for TagCreate {
    fn from(value: Tag) -> Self {
        Self {
            label: value.label,
            category: value.category,
        }
    }
}
