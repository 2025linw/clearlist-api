use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    model::task::TaskModel,
    util::{NULL, PostgresCmp, SQLQueryBuilder, ToPostgresCmp, ToSQLQueryBuilder},
};

use super::{QueryMethod, UpdateMethod};

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct CreateTaskSchema {
    title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub?
}

impl ToSQLQueryBuilder for CreateTaskSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(TaskModel::TABLE);
        builder.set_return(vec![TaskModel::ID]);

        if let Some(ref s) = self.title {
            builder.add_column(TaskModel::TITLE, s);
        }
        if let Some(ref s) = self.notes {
            builder.add_column(TaskModel::NOTES, s);
        }
        if let Some(ref d) = self.start_date {
            builder.add_column(TaskModel::START_DATE, d);
        }
        if let Some(ref t) = self.start_time {
            builder.add_column(TaskModel::START_TIME, t);
        }
        if let Some(ref d) = self.deadline {
            builder.add_column(TaskModel::DEADLINE, d);
        }

        if let Some(ref i) = self.area_id {
            builder.add_column(TaskModel::AREA_ID, i);
        }
        if let Some(ref i) = self.project_id {
            builder.add_column(TaskModel::PROJECT_ID, i);
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskSchema {
    title: Option<UpdateMethod<String>>,
    notes: Option<UpdateMethod<String>>,
    start_date: Option<UpdateMethod<NaiveDate>>,
    start_time: Option<UpdateMethod<NaiveTime>>,
    deadline: Option<UpdateMethod<NaiveDate>>,

    completed: Option<bool>,
    logged: Option<bool>,
    trashed: Option<bool>,

    area_id: Option<UpdateMethod<Uuid>>,
    project_id: Option<UpdateMethod<Uuid>>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub?

    #[serde(default)]
    timestamp: DateTime<Local>,
}

impl UpdateTaskSchema {
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
            && self.project_id.is_none()
            && self.tag_ids.is_none()
    }
}

impl ToSQLQueryBuilder for UpdateTaskSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(TaskModel::TABLE);
        builder.add_column(TaskModel::UPDATED, &self.timestamp);
        builder.set_return(vec![TaskModel::ID]);

        if let Some(ref u) = self.title {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::TITLE, u);
            }
        }
        if let Some(ref u) = self.notes {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::NOTES, u);
            }
        }
        if let Some(ref u) = self.start_date {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::START_DATE, u);
            }
        }
        if let Some(ref u) = self.start_time {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::START_TIME, u);
            }
        }
        if let Some(ref u) = self.deadline {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::DEADLINE, u);
            }
        }

        if let Some(b) = self.completed {
            if b {
                builder.add_column(
                    TaskModel::COMPLETED,
                    builder.get_column(0).unwrap().to_owned(),
                );
            } else {
                builder.add_column(TaskModel::COMPLETED, &None::<DateTime<Local>>);
            }
        }
        if let Some(b) = self.logged {
            if b {
                builder.add_column(TaskModel::LOGGED, builder.get_column(0).unwrap().to_owned());
            } else {
                builder.add_column(TaskModel::LOGGED, &None::<DateTime<Local>>);
            }
        }
        if let Some(b) = self.trashed {
            if b {
                builder.add_column(
                    TaskModel::TRASHED,
                    builder.get_column(0).unwrap().to_owned(),
                );
            } else {
                builder.add_column(TaskModel::TRASHED, &None::<DateTime<Local>>);
            }
        }

        if let Some(ref u) = self.area_id {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::AREA_ID, u);
            }
        }
        if let Some(ref u) = self.project_id {
            if matches!(u, UpdateMethod::Remove(true) | UpdateMethod::Change(..)) {
                builder.add_column(TaskModel::PROJECT_ID, u);
            }
        }

        builder
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all = "camelCase")]
pub struct QueryTaskSchema {
    title: Option<QueryMethod<String>>,
    notes: Option<QueryMethod<String>>,
    start_date: Option<QueryMethod<NaiveDate>>,
    start_time: Option<QueryMethod<NaiveTime>>,
    deadline: Option<QueryMethod<NaiveDate>>,

    completed: Option<bool>,
    logged: Option<bool>,
    trashed: Option<bool>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    pub(crate) tag_ids: Option<Vec<Uuid>>, // TODO: is there a way that this can not be pub? (ExtractTagSchema)
}

impl ToSQLQueryBuilder for QueryTaskSchema {
    fn to_sql_builder(&self) -> SQLQueryBuilder {
        let mut builder = SQLQueryBuilder::new(TaskModel::TABLE);

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
            builder.add_condition(TaskModel::TITLE, cmp, q);
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
            builder.add_condition(TaskModel::NOTES, cmp, q);
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
            builder.add_condition(TaskModel::START_DATE, cmp, q);
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
            builder.add_condition(TaskModel::START_TIME, cmp, q);
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
            builder.add_condition(TaskModel::DEADLINE, cmp, q);
        }

        if let Some(b) = self.completed {
            if b {
                builder.add_condition(TaskModel::COMPLETED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(TaskModel::COMPLETED, PostgresCmp::IsNull, &NULL);
            }
        }
        if let Some(b) = self.logged {
            if b {
                builder.add_condition(TaskModel::LOGGED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(TaskModel::LOGGED, PostgresCmp::IsNull, &NULL);
            }
        }
        if let Some(b) = self.trashed {
            if b {
                builder.add_condition(TaskModel::TRASHED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(TaskModel::TRASHED, PostgresCmp::IsNull, &NULL);
            }
        }

        if let Some(ref i) = self.area_id {
            builder.add_condition(TaskModel::AREA_ID, PostgresCmp::Equal, i);
        }
        if let Some(ref i) = self.project_id {
            builder.add_condition(TaskModel::PROJECT_ID, PostgresCmp::Equal, i);
        }

        builder
    }
}

#[cfg(test)]
mod create_schema_test {
    use chrono::Local;
    use uuid::Uuid;

    use crate::util::ToSQLQueryBuilder;

    use super::CreateTaskSchema;

    #[test]
    fn text_only() {
        let mut schema = CreateTaskSchema::default();
        schema.title = Some("Test Title".to_string());
        schema.notes = Some("Test Note".to_string());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.tasks (task_title, notes) VALUES ($1, $2) RETURNING task_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn date_time_only() {
        let now = Local::now();

        let mut schema = CreateTaskSchema::default();
        schema.start_date = Some(now.date_naive());
        schema.start_time = Some(now.time());
        schema.deadline = Some(now.date_naive());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.tasks (start_date, start_time, deadline) VALUES ($1, $2, $3) RETURNING task_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn id_only() {
        let mut schema = CreateTaskSchema::default();
        schema.area_id = Some(Uuid::new_v4());
        schema.project_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement.as_str(),
            "INSERT INTO data.tasks (area_id, project_id) VALUES ($1, $2) RETURNING task_id"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = CreateTaskSchema::default();
        schema.title = Some("Test Title".to_string());
        schema.notes = Some("Test Note".to_string());
        schema.start_date = Some(now.date_naive());
        schema.start_time = Some(now.time());
        schema.deadline = Some(now.date_naive());
        schema.area_id = Some(Uuid::new_v4());
        schema.project_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_insert();

        assert_eq!(
            statement,
            "INSERT INTO data.tasks (task_title, notes, start_date, start_time, deadline, area_id, project_id) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING task_id"
        );
        assert_eq!(params.len(), 7);
    }

    // TEST: make production example
}

#[cfg(test)]
mod update_schema_test {
    use chrono::Local;
    use uuid::Uuid;

    use crate::{schema::UpdateMethod, util::ToSQLQueryBuilder};

    use super::UpdateTaskSchema;

    #[test]
    fn text_only() {
        let mut schema = UpdateTaskSchema::default();
        schema.title = Some(UpdateMethod::Change("Test Title".to_string()));
        schema.notes = Some(UpdateMethod::Change("Test Note".to_string()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, task_title=$2, notes=$3 RETURNING task_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn date_time_only() {
        let now = Local::now();

        let mut schema = UpdateTaskSchema::default();
        schema.start_date = Some(UpdateMethod::Change(now.date_naive()));
        schema.start_time = Some(UpdateMethod::Change(now.time()));
        schema.deadline = Some(UpdateMethod::Change(now.date_naive()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, start_date=$2, start_time=$3, deadline=$4 RETURNING task_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn bool_only() {
        let mut schema = UpdateTaskSchema::default();
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, completed_on=$2, logged_on=$3, trashed_on=$4 RETURNING task_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn id_only() {
        let mut schema = UpdateTaskSchema::default();
        schema.area_id = Some(UpdateMethod::Change(Uuid::new_v4()));
        schema.project_id = Some(UpdateMethod::Change(Uuid::new_v4()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, area_id=$2, project_id=$3 RETURNING task_id"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = UpdateTaskSchema::default();
        schema.title = Some(UpdateMethod::Change("Test Title".to_string()));
        schema.notes = Some(UpdateMethod::Change("Test Note".to_string()));
        schema.start_date = Some(UpdateMethod::Change(now.date_naive()));
        schema.start_time = Some(UpdateMethod::Change(now.time()));
        schema.deadline = Some(UpdateMethod::Change(now.date_naive()));
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);
        schema.area_id = Some(UpdateMethod::Change(Uuid::new_v4()));
        schema.project_id = Some(UpdateMethod::Change(Uuid::new_v4()));

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, task_title=$2, notes=$3, start_date=$4, start_time=$5, deadline=$6, completed_on=$7, logged_on=$8, trashed_on=$9, area_id=$10, project_id=$11 RETURNING task_id"
        );
        assert_eq!(params.len(), 11);
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

    use super::QueryTaskSchema;

    #[test]
    fn empty() {
        let schema = QueryTaskSchema::default();

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM data.tasks");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn text_only() {
        let mut schema = QueryTaskSchema::default();
        schema.title = Some(QueryMethod::Match("Test Title".to_string()));
        schema.notes = Some(QueryMethod::Match("Test Note".to_string()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE task_title ILIKE '%' || $1 || '%' AND notes ILIKE '%' || $2 || '%'"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn date_time_eq_only() {
        let now = Local::now();

        let mut schema = QueryTaskSchema::default();
        schema.start_date = Some(QueryMethod::Match(now.date_naive()));
        schema.start_time = Some(QueryMethod::Match(now.time()));
        schema.deadline = Some(QueryMethod::Match(now.date_naive()));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE start_date = $1 AND start_time = $2 AND deadline = $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn date_time_cmp_only() {
        let now = Local::now();

        let mut schema = QueryTaskSchema::default();
        schema.start_date = Some(QueryMethod::Compare(now.date_naive(), Compare::Less));
        schema.start_time = Some(QueryMethod::Compare(now.time(), Compare::LessEq));
        schema.deadline = Some(QueryMethod::Compare(now.date_naive(), Compare::Greater));

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE start_date < $1 AND start_time <= $2 AND deadline > $3"
        );
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn bool_t_only() {
        let mut schema = QueryTaskSchema::default();
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.trashed = Some(true);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE completed_on NOT NULL AND logged_on NOT NULL AND trashed_on NOT NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = QueryTaskSchema::default();
        schema.completed = Some(false);
        schema.logged = Some(false);
        schema.trashed = Some(false);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE completed_on IS NULL AND logged_on IS NULL AND trashed_on IS NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn id_only() {
        let mut schema = QueryTaskSchema::default();
        schema.area_id = Some(Uuid::new_v4());
        schema.project_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE area_id = $1 AND project_id = $2"
        );
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn full() {
        let now = Local::now();

        let mut schema = QueryTaskSchema::default();
        schema.title = Some(QueryMethod::Match("Test Title".to_string()));
        schema.notes = Some(QueryMethod::Match("Test Note".to_string()));
        schema.start_date = Some(QueryMethod::Match(now.date_naive()));
        schema.start_time = Some(QueryMethod::Match(now.time()));
        schema.deadline = Some(QueryMethod::Compare(now.date_naive(), Compare::Greater));
        schema.completed = Some(false);
        schema.logged = Some(true);
        schema.trashed = Some(false);
        schema.area_id = Some(Uuid::new_v4());
        schema.project_id = Some(Uuid::new_v4());

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE task_title ILIKE '%' || $1 || '%' AND notes ILIKE '%' || $2 || '%' AND start_date = $3 AND start_time = $4 AND deadline > $5 AND completed_on IS NULL AND logged_on NOT NULL AND trashed_on IS NULL AND area_id = $6 AND project_id = $7"
        );
        assert_eq!(params.len(), 7);
    }

    // TEST: test with tags

    // TEST: make production example
}
