use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{
    models::UpdateMethod,
    util::{SqlQueryBuilder, ToSqlQueryBuilder},
};

use super::ToResponse;

/// User Database Model
#[derive(Debug, Deserialize)]
pub struct DatabaseModel {
    user_id: Uuid,

    username: String,
    email: String,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl DatabaseModel {
    pub const TABLE: &str = "auth.users";

    pub const ID: &str = "user_id";

    pub const USERNAME: &str = "username";
    pub const EMAIL: &str = "email";
    pub const PASS_HASH: &str = "password_hash";

    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl DatabaseModel {
    // pub fn user_id(&self) -> Uuid {
    //     self.user_id
    // }

    // pub fn password_hash(&self) -> &str {
    //     &self.password_hash
    // }
}

impl From<Row> for DatabaseModel {
    fn from(value: Row) -> Self {
        Self {
            user_id: value.get(Self::ID),
            username: value.get(Self::USERNAME),
            email: value.get(Self::EMAIL),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for DatabaseModel {
    type Response = ResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.user_id,
            username: self.username.to_owned(),
            email: self.email.to_owned(),
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseModel {
    id: Uuid,

    username: String,
    email: String,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateRequest {
    username: UpdateMethod<String>,
    email: UpdateMethod<String>,

    #[serde(default = "chrono::Local::now")]
    timestamp: DateTime<Local>,
}

impl UpdateRequest {
    pub fn is_empty(&self) -> bool {
        self.username.is_noop() && self.email.is_noop()
    }
}

impl ToSqlQueryBuilder for UpdateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.add_column(DatabaseModel::UPDATED, &self.timestamp);
        builder.set_return(&[DatabaseModel::ID]);

        if !self.username.is_noop() {
            builder.add_column(DatabaseModel::USERNAME, &self.username);
        }
        if !self.email.is_noop() {
            builder.add_column(DatabaseModel::EMAIL, &self.email);
        }

        builder
    }
}
