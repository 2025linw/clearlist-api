use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::util::{PostgresCmp, SqlQueryBuilder, ToPostgresCmp, ToSqlQueryBuilder};

use super::{QueryMethod, ToResponse, UpdateMethod};

/// Tag Database Model
#[derive(Debug)]
pub struct DatabaseModel {
    id: Uuid,

    tag_label: Option<String>,
    color: Option<String>,

    category: Option<String>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl DatabaseModel {
    pub const TABLE: &str = "data.tags";

    pub const ID: &str = "tag_id";

    pub const LABEL: &str = "tag_label";
    pub const COLOR: &str = "color";

    pub const CATEGORY: &str = "category";

    pub const USER_ID: &str = "user_id";

    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl From<Row> for DatabaseModel {
    fn from(value: Row) -> Self {
        Self {
            id: value.get(Self::ID),
            tag_label: value.get(Self::LABEL),
            color: value.get(Self::COLOR),
            category: value.get(Self::CATEGORY),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for DatabaseModel {
    type Response = ResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.id,
            label: self.tag_label.to_owned().unwrap_or_default(),
            color: self.color.to_owned(),
            category: self.category.to_owned(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

/// Tag Response Model
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseModel {
    id: Uuid,

    label: String,
    color: Option<String>,

    category: Option<String>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateRequest {
    label: Option<String>,
    color: Option<String>,

    category: Option<String>,
}

impl ToSqlQueryBuilder for CreateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return(&[DatabaseModel::ID]);

        if let Some(ref s) = self.label {
            builder.add_column(DatabaseModel::LABEL, s);
        }
        if let Some(ref s) = self.color {
            builder.add_column(DatabaseModel::COLOR, s);
        }

        if let Some(ref s) = self.category {
            builder.add_column(DatabaseModel::CATEGORY, s);
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateRequest {
    label: UpdateMethod<String>,
    color: UpdateMethod<String>,

    category: UpdateMethod<String>,

    #[serde(default = "chrono::Local::now")]
    timestamp: DateTime<Local>,
}

impl UpdateRequest {
    pub fn is_empty(&self) -> bool {
        self.label.is_noop() && self.color.is_noop() && self.category.is_noop()
    }
}

impl ToSqlQueryBuilder for UpdateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.add_column(DatabaseModel::UPDATED, &self.timestamp);
        builder.set_return(&[DatabaseModel::ID]);

        if !self.label.is_noop() {
            builder.add_column(DatabaseModel::LABEL, &self.label);
        }
        if !self.color.is_noop() {
            builder.add_column(DatabaseModel::COLOR, &self.color);
        }

        if !self.category.is_noop() {
            builder.add_column(DatabaseModel::CATEGORY, &self.category);
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueryRequest {
    label: Option<QueryMethod<String>>,

    category: Option<QueryMethod<String>>,
}

impl ToSqlQueryBuilder for QueryRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return_all();

        if let Some(ref q) = self.label {
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
            builder.add_condition(DatabaseModel::LABEL, cmp, q);
        }

        if let Some(ref q) = self.category {
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
            builder.add_condition(DatabaseModel::CATEGORY, cmp, q);
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
        schema.label = Some("Test Label".to_string());
        schema.color = Some("#2f78ed".to_string());
        schema.category = Some("Priority".to_string());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.tags (tag_label, color, category) VALUES ($1, $2, $3) RETURNING tag_id"
        );
        assert_eq!(params.len(), 3);
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
        schema.label = UpdateMethod::Set("Test Label".to_string());
        schema.color = UpdateMethod::Set("#2f78ed".to_string());
        schema.category = UpdateMethod::Set("Priority".to_string());

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tags SET updated_on=$1, tag_label=$2, color=$3, category=$4 RETURNING tag_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn full() {
        let mut schema = UpdateRequest::default();
        schema.label = UpdateMethod::Set("Test Label".to_string());
        schema.color = UpdateMethod::Set("#2f78ed".to_string());
        schema.category = UpdateMethod::Set("Priority".to_string());

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tags SET updated_on=$1, tag_label=$2, color=$3, category=$4 RETURNING tag_id"
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

        assert_eq!(statement.as_str(), "SELECT * FROM data.tags");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn text_only() {
        let mut schema = QueryRequest::default();
        schema.label = Some(QueryMethod::Match("Test Label".to_string()));
        schema.category = Some(QueryMethod::Match("Priority".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tags WHERE tag_label ILIKE '%' || $1 || '%' AND category ILIKE '%' || $2 || '%'"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn full() {
        let mut schema = QueryRequest::default();
        schema.label = Some(QueryMethod::Match("Test Label".to_string()));
        schema.category = Some(QueryMethod::Match("Priority".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tags WHERE tag_label ILIKE '%' || $1 || '%' AND category ILIKE '%' || $2 || '%'"
        );
        assert_eq!(params.len(), 2);
    }
}
