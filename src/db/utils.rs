use std::cmp::Ordering;

use crate::models::Tag;

/// sort_by function to sort a Vec<Tag> by order desired by Task-Tags
pub fn sort_task_tag(a: &Tag, b: &Tag) -> Ordering {
    if a.category > b.category {
        Ordering::Greater
    } else if a.category < b.category {
        Ordering::Less
    } else if a.label > b.label {
        // same category
        Ordering::Greater
    } else if a.label < b.label {
        Ordering::Less
    } else if a.updated_at < b.updated_at {
        // same category, label
        Ordering::Greater
    } else if a.updated_at > b.updated_at {
        Ordering::Less
    } else if a.id > b.id {
        // same category, label, updated_at
        Ordering::Greater
    } else if a.id < b.id {
        Ordering::Less
    } else {
        // all the same sort fields
        Ordering::Equal
    }
}
