use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::util::{NULL, PostgresCmp, SqlQueryBuilder, ToPostgresCmp, ToSqlQueryBuilder};

use super::{QueryMethod, ToResponse, UpdateMethod};

/// Area Database Model
#[derive(Debug)]
pub struct DatabaseModel {
    id: Uuid,

    area_name: Option<String>,
    icon_url: Option<String>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
    deleted_on: Option<DateTime<Local>>,
}

impl DatabaseModel {
    pub const TABLE: &str = "data.areas";

    pub const ID: &str = "area_id";

    pub const NAME: &str = "area_name";
    pub const ICON_URL: &str = "icon_url";

    pub const USER_ID: &str = "user_id";

    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
    pub const DELETED: &str = "deleted_on";
}

impl From<Row> for DatabaseModel {
    fn from(value: Row) -> Self {
        Self {
            id: value.get(Self::ID),
            area_name: value.get(Self::NAME),
            icon_url: value.get(Self::ICON_URL),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
            deleted_on: value.get(Self::DELETED),
        }
    }
}

impl ToResponse for DatabaseModel {
    type Response = ResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.id,
            name: self.area_name.to_owned().unwrap_or_default(),
            icon_url: self.icon_url.to_owned(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
            deleted_on: self.deleted_on,
        }
    }
}

/// Area Response Model
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseModel {
    id: Uuid,

    name: String,
    icon_url: Option<String>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
    deleted_on: Option<DateTime<Local>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateRequest {
    name: Option<String>,
    icon_url: Option<String>,
}

impl ToSqlQueryBuilder for CreateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return(&[DatabaseModel::ID]);

        if let Some(ref s) = self.name {
            builder.add_column(DatabaseModel::NAME, s);
        }
        if let Some(ref s) = self.icon_url {
            builder.add_column(DatabaseModel::ICON_URL, s);
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateRequest {
    name: UpdateMethod<String>,
    icon_url: UpdateMethod<String>,

    deleted: UpdateMethod<bool>,

    #[serde(default = "chrono::Local::now")]
    timestamp: DateTime<Local>,
}

impl UpdateRequest {
    pub fn is_empty(&self) -> bool {
        self.name.is_noop() && self.icon_url.is_noop() && self.deleted.is_noop()
    }
}

impl ToSqlQueryBuilder for UpdateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.add_column(DatabaseModel::UPDATED, &self.timestamp);
        builder.set_return(&[DatabaseModel::ID]);

        if !self.name.is_noop() {
            builder.add_column(DatabaseModel::NAME, &self.name);
        }
        if !self.icon_url.is_noop() {
            builder.add_column(DatabaseModel::ICON_URL, &self.icon_url);
        }

        if !self.deleted.is_noop() {
            match self.deleted {
                UpdateMethod::Set(true) => {
                    builder.add_column(DatabaseModel::DELETED, &self.timestamp);
                }
                UpdateMethod::Set(false) | UpdateMethod::Remove => {
                    builder.add_column(DatabaseModel::DELETED, &None::<DateTime<Local>>);
                }
                UpdateMethod::NoOp => unreachable!(),
            }
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueryRequest {
    name: Option<QueryMethod<String>>,

    deleted: Option<bool>,
}

impl ToSqlQueryBuilder for QueryRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return_all();

        if let Some(ref q) = self.name {
            let cmp;
            match q {
                QueryMethod::NotNull(b) => {
                    if *b {
                        cmp = PostgresCmp::NotNull;
                    } else {
                        cmp = PostgresCmp::IsNull;
                    }
                }
                QueryMethod::Match(_) => cmp = PostgresCmp::ILike,
                QueryMethod::Compare(_, c) => cmp = c.to_postgres_cmp(),
            }
            builder.add_condition(DatabaseModel::NAME, cmp, q);
        }

        if let Some(b) = self.deleted {
            if b {
                builder.add_condition(DatabaseModel::DELETED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(DatabaseModel::DELETED, PostgresCmp::IsNull, &NULL);
            }
        }

        builder
    }
}

#[cfg(test)]
mod create_schema {
    use crate::util::ToSqlQueryBuilder;

    use super::CreateRequest;

    #[test]
    fn full() {
        let mut schema = CreateRequest::default();
        schema.name = Some("Test Name".to_string());
        schema.icon_url = Some("https://www.google.com/favicon.ico".to_string());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.areas (area_name, icon_url) VALUES ($1, $2) RETURNING area_id"
        );
        assert_eq!(params.len(), 2);
    }
}

#[cfg(test)]
mod update_schema {
    use crate::{models::UpdateMethod, util::ToSqlQueryBuilder};

    use super::UpdateRequest;

    #[test]
    fn is_empty() {
        let schema = UpdateRequest::default();

        assert!(schema.is_empty());
    }

    #[test]
    fn text_only() {
        let mut schema = UpdateRequest::default();
        schema.name = UpdateMethod::Set("Test Name".to_string());
        schema.icon_url = UpdateMethod::Set("https://www.mozilla.org/media/protocol/img/logos/firefox/browser/logo.eb1324e44442.svg".to_string());

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.areas SET updated_on=$1, area_name=$2, icon_url=$3 RETURNING area_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn bool_t_only() {
        let mut schema = UpdateRequest::default();
        schema.deleted = UpdateMethod::Set(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.areas SET updated_on=$1, deleted_on=$2 RETURNING area_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = UpdateRequest::default();
        schema.deleted = UpdateMethod::Set(false);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.areas SET updated_on=$1, deleted_on=$2 RETURNING area_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn full() {
        let mut schema = UpdateRequest::default();
        schema.name = UpdateMethod::Set("Test Name".to_string());
        schema.icon_url = UpdateMethod::Set("https://www.mozilla.org/media/protocol/img/logos/firefox/browser/logo.eb1324e44442.svg".to_string());
        schema.deleted = UpdateMethod::Set(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.areas SET updated_on=$1, area_name=$2, icon_url=$3, deleted_on=$4 RETURNING area_id"
        );
        assert_eq!(params.len(), 4);
    }
}

#[cfg(test)]
mod query_schema {
    use crate::{models::QueryMethod, util::ToSqlQueryBuilder};

    use super::QueryRequest;

    #[test]
    fn empty() {
        let schema = QueryRequest::default();

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM data.areas");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn text_only() {
        let mut schema = QueryRequest::default();
        schema.name = Some(QueryMethod::Match("Test Name".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.areas WHERE area_name ILIKE '%' || $1 || '%'",
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn bool_t_only() {
        let mut schema = QueryRequest::default();
        schema.deleted = Some(true);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.areas WHERE deleted_on NOT NULL",
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = QueryRequest::default();
        schema.deleted = Some(false);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.areas WHERE deleted_on IS NULL",
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn full() {
        let mut schema = QueryRequest::default();
        schema.name = Some(QueryMethod::Match("Test Name".to_string()));
        schema.deleted = Some(true);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.areas WHERE area_name ILIKE '%' || $1 || '%' AND deleted_on NOT NULL",
        );
        assert_eq!(params.len(), 1);
    }
}
