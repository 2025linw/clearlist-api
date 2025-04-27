use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use super::ToResponse;

/// Tag Database Model
#[derive(Debug, Deserialize)]
pub struct TagModel {
    tag_id: Uuid,

    tag_label: Option<String>,
    color: Option<String>,

    category: Option<String>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl TagModel {
    pub const TABLE: &str = "data.tags";

    pub const ID: &str = "tag_id";

    pub const LABEL: &str = "tag_label";
    pub const COLOR: &str = "color";

    pub const CATEGORY: &str = "category";

    pub const USER_ID: &str = "user_id";
    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl From<Row> for TagModel {
    fn from(value: Row) -> Self {
        Self {
            tag_id: value.get(Self::ID),
            tag_label: value.get(Self::LABEL),
            color: value.get(Self::COLOR),
            category: value.get(Self::CATEGORY),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for TagModel {
    type Response = TagResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.tag_id,
            label: self.tag_label.to_owned().unwrap_or_default(),
            color: self.color.to_owned().unwrap_or_default(),
            category: self.category.to_owned().unwrap_or_default(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

/// Tag Response Model
#[derive(Debug, Deserialize, Serialize)]
pub struct TagResponseModel {
    id: Uuid,

    label: String,
    color: String,

    category: String,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}
