use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use super::ToResponse;

/// Area Database Model
#[derive(Debug, Deserialize)]
pub struct AreaModel {
    area_id: Uuid,

    area_name: Option<String>,
    icon_url: Option<String>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl AreaModel {
    pub const TABLE: &str = "data.areas";

    pub const ID: &str = "area_id";

    pub const NAME: &str = "area_name";
    pub const ICON_URL: &str = "icon_url";

    pub const USER_ID: &str = "user_id";
    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl From<Row> for AreaModel {
    fn from(value: Row) -> Self {
        Self {
            area_id: value.get(Self::ID),
            area_name: value.get(Self::NAME),
            icon_url: value.get(Self::ICON_URL),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for AreaModel {
    type Response = AreaResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.area_id,
            name: self.area_name.to_owned().unwrap_or_default(),
            icon_url: self.icon_url.to_owned().unwrap_or_default(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

/// Area Response Model
#[derive(Debug, Deserialize, Serialize)]
pub struct AreaResponseModel {
    id: Uuid,

    name: String,
    icon_url: String,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}
