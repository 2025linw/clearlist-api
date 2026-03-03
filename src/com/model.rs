mod tag;
mod task;

pub use tag::Tag;
pub use task::Task;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FilterOptions {
    pub page: u64,
    pub limit: u64,
}

impl Default for FilterOptions {
    fn default() -> Self {
        Self { page: 1, limit: 25 }
    }
}

#[derive(Debug, Deserialize)]
pub struct PathTaskTag {
    pub task_id: Uuid,
    pub tag_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct TaskTag {
    pub tag_ids: Vec<Uuid>,
}
