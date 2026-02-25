mod tag;
mod task;

use serde::Deserialize;

pub use tag::Tag;
pub use task::Task;

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
