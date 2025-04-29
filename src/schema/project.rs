use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    model::project::ProjectModel,
    util::{NULL, PostgresCmp, SQLQueryBuilder, ToPostgresCmp, ToSQLQueryBuilder},
};

use super::{QueryMethod, UpdateMethod};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectSchema {
    title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    area_id: Option<Uuid>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub?
}

impl ToSQLQueryBuilder for CreateProjectSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(ProjectModel::TABLE);
        builder.set_return(vec![ProjectModel::ID]);

        if let Some(ref s) = self.title {
            builder.add_column(ProjectModel::TITLE, s);
        }
        if let Some(ref s) = self.notes {
            builder.add_column(ProjectModel::NOTES, s);
        }
        if let Some(ref d) = self.start_date {
            builder.add_column(ProjectModel::START_DATE, d);
        }
        if let Some(ref t) = self.start_time {
            builder.add_column(ProjectModel::START_TIME, t);
        }
        if let Some(ref d) = self.deadline {
            builder.add_column(ProjectModel::DEADLINE, d);
        }

        if let Some(ref i) = self.area_id {
            builder.add_column(ProjectModel::AREA_ID, i);
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectSchema {
    title: Option<UpdateMethod<String>>,
    notes: Option<UpdateMethod<String>>,
    start_date: Option<UpdateMethod<NaiveDate>>,
    start_time: Option<UpdateMethod<NaiveTime>>,
    deadline: Option<UpdateMethod<NaiveDate>>,

    completed: Option<bool>,
    logged: Option<bool>,
    trashed: Option<bool>,

    area_id: Option<UpdateMethod<Uuid>>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub?

    #[serde(default)]
    timestamp: DateTime<Local>,
}

impl UpdateProjectSchema {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.notes.is_none()
            && self.start_date.is_none()
            && self.start_time.is_none()
            && self.deadline.is_none()
            && self.completed.is_none()
            && self.logged.is_none()
            && self.trashed.is_none()
            && self.area_id.is_none()
            && self.tag_ids.is_none()
    }
}

impl ToSQLQueryBuilder for UpdateProjectSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(ProjectModel::TABLE);
        builder.add_column(ProjectModel::UPDATED, &self.timestamp);
        builder.set_return(vec![ProjectModel::ID]);

        if let Some(ref u) = self.title {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::TITLE, u);
            }
        }
        if let Some(ref u) = self.notes {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::NOTES, u);
            }
        }
        if let Some(ref u) = self.start_date {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::START_DATE, u);
            }
        }
        if let Some(ref u) = self.start_time {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::START_TIME, u);
            }
        }
        if let Some(ref u) = self.deadline {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::DEADLINE, u);
            }
        }

        if let Some(b) = self.completed {
            if b {
                builder.add_column(
                    ProjectModel::COMPLETED,
                    builder.get_column(0).unwrap().to_owned(),
                );
            } else {
                builder.add_column(ProjectModel::COMPLETED, &None::<DateTime<Local>>);
            }
        }
        if let Some(b) = self.logged {
            if b {
                builder.add_column(
                    ProjectModel::LOGGED,
                    builder.get_column(0).unwrap().to_owned(),
                );
            } else {
                builder.add_column(ProjectModel::LOGGED, &None::<DateTime<Local>>);
            }
        }
        if let Some(b) = self.trashed {
            if b {
                builder.add_column(
                    ProjectModel::TRASHED,
                    builder.get_column(0).unwrap().to_owned(),
                );
            } else {
                builder.add_column(ProjectModel::TRASHED, &None::<DateTime<Local>>);
            }
        }

        if let Some(ref u) = self.area_id {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(ProjectModel::AREA_ID, u);
            }
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct QueryProjectSchema {
    title: Option<QueryMethod<String>>,
    notes: Option<QueryMethod<String>>,
    start_date: Option<QueryMethod<NaiveDate>>,
    start_time: Option<QueryMethod<NaiveTime>>,
    deadline: Option<QueryMethod<NaiveDate>>,

    completed: Option<bool>,
    logged: Option<bool>,
    trashed: Option<bool>,

    area_id: Option<Uuid>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub?
}

impl ToSQLQueryBuilder for QueryProjectSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(ProjectModel::TABLE);

        if let Some(ref q) = self.title {
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
            builder.add_condition(ProjectModel::TITLE, cmp, q);
        }
        if let Some(ref q) = self.notes {
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
            builder.add_condition(ProjectModel::NOTES, cmp, q);
        }
        if let Some(ref q) = self.start_date {
            let cmp;
            match q {
                QueryMethod::NotNull(b) => {
                    if *b {
                        cmp = PostgresCmp::NotNull;
                    } else {
                        cmp = PostgresCmp::IsNull;
                    }
                }
                QueryMethod::Match(_) => cmp = PostgresCmp::Equal,
                QueryMethod::Compare(_, c) => cmp = c.to_postgres_cmp(),
            }
            builder.add_condition(ProjectModel::START_DATE, cmp, q);
        }
        if let Some(ref q) = self.start_time {
            let cmp;
            match q {
                QueryMethod::NotNull(b) => {
                    if *b {
                        cmp = PostgresCmp::NotNull;
                    } else {
                        cmp = PostgresCmp::IsNull;
                    }
                }
                QueryMethod::Match(_) => cmp = PostgresCmp::Equal,
                QueryMethod::Compare(_, c) => cmp = c.to_postgres_cmp(),
            }
            builder.add_condition(ProjectModel::START_TIME, cmp, q);
        }
        if let Some(ref q) = self.deadline {
            let cmp;
            match q {
                QueryMethod::NotNull(b) => {
                    if *b {
                        cmp = PostgresCmp::NotNull;
                    } else {
                        cmp = PostgresCmp::IsNull;
                    }
                }
                QueryMethod::Match(_) => cmp = PostgresCmp::Equal,
                QueryMethod::Compare(_, c) => cmp = c.to_postgres_cmp(),
            }
            builder.add_condition(ProjectModel::DEADLINE, cmp, q);
        }

        if let Some(b) = self.completed {
            if b {
                builder.add_condition(ProjectModel::COMPLETED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(ProjectModel::COMPLETED, PostgresCmp::IsNull, &NULL);
            }
        }
        if let Some(b) = self.logged {
            if b {
                builder.add_condition(ProjectModel::LOGGED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(ProjectModel::LOGGED, PostgresCmp::IsNull, &NULL);
            }
        }
        if let Some(b) = self.trashed {
            if b {
                builder.add_condition(ProjectModel::TRASHED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(ProjectModel::TRASHED, PostgresCmp::IsNull, &NULL);
            }
        }

        if let Some(ref i) = self.area_id {
            builder.add_condition(ProjectModel::AREA_ID, PostgresCmp::Equal, i);
        }

        builder
    }
}

#[cfg(test)]
mod create_schema_test {
    use chrono::Local;
    use uuid::Uuid;

    use crate::util::ToSQLQueryBuilder;

    use super::CreateProjectSchema;

    #[test]
    fn text_only() {
        let mut schema = CreateProjectSchema::default();
        schema.title = Some("Test Title".to_string());
        schema.notes = Some("Test Note".to_string());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.projects (project_title, notes) VALUES ($1, $2) RETURNING project_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn date_time_only() {
        let now = Local::now();

        let mut schema = CreateProjectSchema::default();
        schema.start_date = Some(now.date_naive());
        schema.start_time = Some(now.time());
        schema.deadline = Some(now.date_naive());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.projects (start_date, start_time, deadline) VALUES ($1, $2, $3) RETURNING project_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn id_only() {
        let mut schema = CreateProjectSchema::default();
        schema.area_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.projects (area_id) VALUES ($1) RETURNING project_id"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = CreateProjectSchema::default();
        schema.title = Some("Test Title".to_string());
        schema.notes = Some("Test Note".to_string());
        schema.start_date = Some(now.date_naive());
        schema.start_time = Some(now.time());
        schema.deadline = Some(now.date_naive());
        schema.area_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement,
            "INSERT INTO data.projects (project_title, notes, start_date, start_time, deadline, area_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING project_id"
        );
        assert_eq!(params.len(), 6);
    }

    // TEST: make production example
}

#[cfg(test)]
mod update_schema_test {
    use chrono::Local;
    use uuid::Uuid;

    use crate::{schema::UpdateMethod, util::ToSQLQueryBuilder};

    use super::UpdateProjectSchema;

    #[test]
    fn text_only() {
        let mut schema = UpdateProjectSchema::default();
        schema.title = Some(UpdateMethod::Change("Test Title".to_string()));
        schema.notes = Some(UpdateMethod::Change("Test Note".to_string()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.projects SET updated_on=$1, project_title=$2, notes=$3 RETURNING project_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn date_time_only() {
        let now = Local::now();

        let mut schema = UpdateProjectSchema::default();
        schema.start_date = Some(UpdateMethod::Change(now.date_naive()));
        schema.start_time = Some(UpdateMethod::Change(now.time()));
        schema.deadline = Some(UpdateMethod::Change(now.date_naive()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.projects SET updated_on=$1, start_date=$2, start_time=$3, deadline=$4 RETURNING project_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn bool_only() {
        let mut schema = UpdateProjectSchema::default();
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.projects SET updated_on=$1, completed_on=$2, logged_on=$3, trashed_on=$4 RETURNING project_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn id_only() {
        let mut schema = UpdateProjectSchema::default();
        schema.area_id = Some(UpdateMethod::Change(Uuid::new_v4()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.projects SET updated_on=$1, area_id=$2 RETURNING project_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = UpdateProjectSchema::default();
        schema.title = Some(UpdateMethod::Change("Test Title".to_string()));
        schema.notes = Some(UpdateMethod::Change("Test Note".to_string()));
        schema.start_date = Some(UpdateMethod::Change(now.date_naive()));
        schema.start_time = Some(UpdateMethod::Change(now.time()));
        schema.deadline = Some(UpdateMethod::Change(now.date_naive()));
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);
        schema.area_id = Some(UpdateMethod::Change(Uuid::new_v4()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.projects SET updated_on=$1, project_title=$2, notes=$3, start_date=$4, start_time=$5, deadline=$6, completed_on=$7, logged_on=$8, trashed_on=$9, area_id=$10 RETURNING project_id"
        );
        assert_eq!(params.len(), 10);
    }

    // TEST: make production example
}

#[cfg(test)]
mod query_schema_test {
    use chrono::Local;
    use uuid::Uuid;

    use crate::{
        schema::{Compare, QueryMethod},
        util::ToSQLQueryBuilder,
    };

    use super::QueryProjectSchema;

    #[test]
    fn empty() {
        let schema = QueryProjectSchema::default();

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM data.projects");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn text_only() {
        let mut schema = QueryProjectSchema::default();
        schema.title = Some(QueryMethod::Match("Test Title".to_string()));
        schema.notes = Some(QueryMethod::Match("Test Note".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE project_title ILIKE '%' || $1 || '%' AND notes ILIKE '%' || $2 || '%'"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn date_time_eq_only() {
        let now = Local::now();

        let mut schema = QueryProjectSchema::default();
        schema.start_date = Some(QueryMethod::Match(now.date_naive()));
        schema.start_time = Some(QueryMethod::Match(now.time()));
        schema.deadline = Some(QueryMethod::Match(now.date_naive()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE start_date = $1 AND start_time = $2 AND deadline = $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn date_time_cmp_only() {
        let now = Local::now();

        let mut schema = QueryProjectSchema::default();
        schema.start_date = Some(QueryMethod::Compare(now.date_naive(), Compare::Less));
        schema.start_time = Some(QueryMethod::Compare(now.time(), Compare::LessEq));
        schema.deadline = Some(QueryMethod::Compare(now.date_naive(), Compare::Greater));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE start_date < $1 AND start_time <= $2 AND deadline > $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn bool_t_only() {
        let mut schema = QueryProjectSchema::default();
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE completed_on NOT NULL AND logged_on NOT NULL AND trashed_on NOT NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = QueryProjectSchema::default();
        schema.completed = Some(false);
        schema.logged = Some(false);
        schema.trashed = Some(false);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE completed_on IS NULL AND logged_on IS NULL AND trashed_on IS NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn id_only() {
        let mut schema = QueryProjectSchema::default();
        schema.area_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE area_id = $1"
        );
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = QueryProjectSchema::default();
        schema.title = Some(QueryMethod::Match("Test Title".to_string()));
        schema.notes = Some(QueryMethod::Match("Test Note".to_string()));
        schema.start_date = Some(QueryMethod::Match(now.date_naive()));
        schema.start_time = Some(QueryMethod::Match(now.time()));
        schema.deadline = Some(QueryMethod::Compare(now.date_naive(), Compare::Greater));
        schema.completed = Some(false);
        schema.logged = Some(true);
        schema.trashed = Some(false);
        schema.area_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.projects WHERE project_title ILIKE '%' || $1 || '%' AND notes ILIKE '%' || $2 || '%' AND start_date = $3 AND start_time = $4 AND deadline > $5 AND completed_on IS NULL AND logged_on NOT NULL AND trashed_on IS NULL AND area_id = $6"
        );
        assert_eq!(params.len(), 6);
    }

    // TEST: test with tags

    // TEST: make production example
}
