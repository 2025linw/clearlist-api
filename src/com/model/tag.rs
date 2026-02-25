use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    #[serde(default)]
    pub id: uuid::Uuid,

    #[serde(default)]
    pub label: String,
    pub category: Option<String>,
}

impl From<tokio_postgres::Row> for Tag {
    fn from(value: tokio_postgres::Row) -> Self {
        Self {
            id: value.get("id"),
            label: value.get("label"),
            category: value.get("category"),
        }
    }
}
