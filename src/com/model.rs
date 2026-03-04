mod tag;
mod task;

pub mod query;

pub use tag::{Tag, TagQuery};
pub use task::{Task, TaskQuery};

use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PathTaskTag {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct TaskTag {
    pub tag_ids: Vec<Uuid>,
}
