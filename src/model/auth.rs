use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use super::ToResponse;

/// User Database Model
#[derive(Debug, Deserialize)]
pub struct UserModel {
    user_id: Uuid,

    email: String,
    password_hash: String,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl UserModel {
    pub const TABLE: &str = "auth.users";

    pub const ID: &str = "user_id";

    pub const EMAIL: &str = "email";
    pub const PASS_HASH: &str = "password_hash";

    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl UserModel {
    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

impl From<Row> for UserModel {
    fn from(value: Row) -> Self {
        Self {
            user_id: value.get(Self::ID),
            email: value.get(Self::EMAIL),
            password_hash: value.get(Self::PASS_HASH),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for UserModel {
    type Response = UserResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.user_id,
            email: self.email.to_owned(),
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponseModel {
    id: Uuid,

    email: String,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}
