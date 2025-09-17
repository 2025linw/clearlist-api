use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{
    models::{ToResponse, user},
    util::{SqlQueryBuilder, ToSqlQueryBuilder},
};

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    email: String,
    password: String,
}

impl LoginRequest {
    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct LoginInfo {
    email: String,
    password_hash: String,
}

impl LoginInfo {
    pub fn new(email: String, password_hash: String) -> Self {
        Self {
            email,
            password_hash,
        }
    }
}

impl ToSqlQueryBuilder for LoginInfo {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
        builder.add_column(user::DatabaseModel::EMAIL, &self.email);
        builder.add_column(user::DatabaseModel::PASS_HASH, &self.password_hash);

        builder.set_return(&[user::DatabaseModel::EMAIL, user::DatabaseModel::PASS_HASH]);

        builder
    }
}

pub struct UserLogin {
    user_id: Uuid,
    email: String, // TODO: do we need this?
    password_hash: String,
}

impl UserLogin {
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
}

impl From<Row> for UserLogin {
    fn from(value: Row) -> Self {
        Self {
            user_id: value.get(user::DatabaseModel::ID),
            email: value.get(user::DatabaseModel::EMAIL),
            password_hash: value.get(user::DatabaseModel::PASS_HASH),
        }
    }
}

impl ToResponse for UserLogin {
    type Response = LoginResponse;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            user_id: self.user_id,
            email: self.email.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    user_id: Uuid,
    email: String,
    #[serde(flatten)]
    tokens: TokenResponse,
}

impl LoginResponse {
    pub fn set_access_jwt(&mut self, access_jwt: String) {
        self.tokens.access_jwt = access_jwt;
    }

    pub fn set_refresh_jwt(&mut self, refresh_jwt: String) {
        self.tokens.refresh_jwt = Some(refresh_jwt);
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    access_jwt: String,
    refresh_jwt: Option<String>,
}

impl TokenResponse {
    pub fn new(access_jwt: String) -> Self {
        Self {
            access_jwt,
            ..Default::default()
        }
    }

    pub fn set_refresh_jwt(&mut self, refresh_jwt: String) {
        self.refresh_jwt = Some(refresh_jwt);
    }
}

// #[derive(Debug, Deserialize)]
// #[cfg_attr(test, derive(Default))]
// #[serde(rename_all = "camelCase")]
// pub struct ResetSchema {
//     pub email: String,
// }
