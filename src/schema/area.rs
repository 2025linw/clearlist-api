use chrono::{DateTime, Local};
use serde::Deserialize;

use crate::{
    model::area::AreaModel,
    util::{PostgresCmp, SQLQueryBuilder, ToPostgresCmp, ToSQLQueryBuilder},
};

use super::{QueryMethod, UpdateMethod};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct CreateAreaSchema {
    name: Option<String>,
    icon_url: Option<String>,
}

impl ToSQLQueryBuilder for CreateAreaSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(AreaModel::TABLE);
        builder.set_return_all();

        if let Some(ref s) = self.name {
            builder.add_column(AreaModel::NAME, s);
        }
        if let Some(ref s) = self.icon_url {
            builder.add_column(AreaModel::ICON_URL, s);
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct UpdateAreaSchema {
    name: Option<UpdateMethod<String>>,
    icon_url: Option<UpdateMethod<String>>,

    #[serde(default)]
    timestamp: DateTime<Local>,
}

impl UpdateAreaSchema {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.icon_url.is_none()
    }
}

impl ToSQLQueryBuilder for UpdateAreaSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(AreaModel::TABLE);
        builder.add_column(AreaModel::UPDATED, &self.timestamp);
        builder.set_return_all();

        if let Some(ref u) = self.name {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(AreaModel::NAME, u);
            }
        }
        if let Some(ref u) = self.icon_url {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(AreaModel::ICON_URL, u);
            }
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct QueryAreaSchema {
    name: Option<QueryMethod<String>>,
}

impl ToSQLQueryBuilder for QueryAreaSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(AreaModel::TABLE);

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
            builder.add_condition(AreaModel::NAME, cmp, q);
        }

        builder
    }
}

#[cfg(test)]
mod create_schema_test {
    use crate::util::ToSQLQueryBuilder;

    use super::CreateAreaSchema;

    #[test]
    fn full() {
        let mut schema = CreateAreaSchema::default();
        schema.name = Some("Test Name".to_string());
        schema.icon_url = Some("https://www.google.com/favicon.ico".to_string());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.areas (area_name, icon_url) VALUES ($1, $2) RETURNING *"
        );
        assert_eq!(params.len(), 2);
    }

    // TEST: make production example
}

#[cfg(test)]
mod update_schema_test {
    use crate::{schema::UpdateMethod, util::ToSQLQueryBuilder};

    use super::UpdateAreaSchema;

    #[test]
    fn full() {
        let mut schema = UpdateAreaSchema::default();
        schema.name = Some(UpdateMethod::Change("Test Name".to_string()));
        schema.icon_url = Some(UpdateMethod::Change("https://www.mozilla.org/media/protocol/img/logos/firefox/browser/logo.eb1324e44442.svg".to_string()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.areas SET updated_on=$1, area_name=$2, icon_url=$3 RETURNING *"
        );
        assert_eq!(params.len(), 3);
    }

    // TEST: make production example
}

#[cfg(test)]
mod query_schema_test {
    use crate::{schema::QueryMethod, util::ToSQLQueryBuilder};

    use super::QueryAreaSchema;

    #[test]
    fn empty() {
        let schema = QueryAreaSchema::default();

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM data.areas");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn full() {
        let mut schema = QueryAreaSchema::default();
        schema.name = Some(QueryMethod::Match("Test Name".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.areas WHERE area_name ILIKE '%' || $1 || '%'"
        );
        assert_eq!(params.len(), 1);
    }

    // TEST: make production example
}
