pub mod tag;

use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{
    models::tag as tag_model,
    util::{NULL, PostgresCmp, SqlQueryBuilder, ToPostgresCmp, ToSqlQueryBuilder},
};

use super::{QueryMethod, ToResponse, UpdateMethod};

/// Task Database Model
#[derive(Debug)]
pub struct DatabaseModel {
    id: Uuid,

    task_title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    completed_on: Option<DateTime<Local>>,
    logged_on: Option<DateTime<Local>>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    tags: Vec<tag_model::DatabaseModel>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
    deleted_on: Option<DateTime<Local>>,
}

impl DatabaseModel {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn id_as_ref(&self) -> &Uuid {
        &self.id
    }

    pub fn set_tags(&mut self, tags: Vec<tag_model::DatabaseModel>) {
        self.tags = tags;
    }
}

impl DatabaseModel {
    pub const TABLE: &str = "data.tasks";

    pub const ID: &str = "task_id";

    pub const TITLE: &str = "task_title";
    pub const NOTES: &str = "notes";
    pub const START_DATE: &str = "start_date";
    pub const START_TIME: &str = "start_time";
    pub const DEADLINE: &str = "deadline";

    pub const COMPLETED: &str = "completed_on";
    pub const LOGGED: &str = "logged_on";

    pub const AREA_ID: &str = "area_id";
    pub const PROJECT_ID: &str = "project_id";

    pub const USER_ID: &str = "user_id";

    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
    pub const DELETED: &str = "deleted_on";
}

impl From<Row> for DatabaseModel {
    fn from(value: Row) -> Self {
        Self {
            id: value.get(Self::ID),
            task_title: value.get(Self::TITLE),
            notes: value.get(Self::NOTES),
            start_date: value.get(Self::START_DATE),
            start_time: value.get(Self::START_TIME),
            deadline: value.get(Self::DEADLINE),
            completed_on: value.get(Self::COMPLETED),
            logged_on: value.get(Self::LOGGED),
            area_id: value.get(Self::AREA_ID),
            project_id: value.get(Self::PROJECT_ID),
            tags: Vec::new(),
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
            title: self.task_title.to_owned().unwrap_or_default(),
            notes: self.notes.to_owned().unwrap_or_default(),
            start_date: self.start_date,
            start_time: self.start_time,
            deadline: self.deadline,
            completed_on: self.completed_on,
            logged_on: self.logged_on,
            area_id: self.area_id,
            project_id: self.project_id,
            tags: self.tags.iter().map(|t| t.to_response()).collect(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
            deleted_on: self.deleted_on,
        }
    }
}

/// Task Response Model
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseModel {
    id: Uuid,

    title: String,
    notes: String,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    completed_on: Option<DateTime<Local>>,
    logged_on: Option<DateTime<Local>>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    tags: Vec<tag_model::ResponseModel>,

    user_id: Uuid,

    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
    deleted_on: Option<DateTime<Local>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateRequest {
    title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    tag_ids: Option<Vec<Uuid>>,
}

impl CreateRequest {
    pub fn tag_ids(&self) -> Option<&[Uuid]> {
        self.tag_ids.as_deref()
    }
}

impl ToSqlQueryBuilder for CreateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return(&[DatabaseModel::ID]);

        if let Some(ref s) = self.title {
            builder.add_column(DatabaseModel::TITLE, s);
        }

        if let Some(ref s) = self.notes {
            builder.add_column(DatabaseModel::NOTES, s);
        }
        if let Some(ref d) = self.start_date {
            builder.add_column(DatabaseModel::START_DATE, d);
        }
        if let Some(ref t) = self.start_time {
            builder.add_column(DatabaseModel::START_TIME, t);
        }
        if let Some(ref d) = self.deadline {
            builder.add_column(DatabaseModel::DEADLINE, d);
        }

        if let Some(ref i) = self.area_id {
            builder.add_column(DatabaseModel::AREA_ID, i);
        }
        if let Some(ref i) = self.project_id {
            builder.add_column(DatabaseModel::PROJECT_ID, i);
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateRequest {
    title: UpdateMethod<String>,
    notes: UpdateMethod<String>,
    start_date: UpdateMethod<NaiveDate>,
    start_time: UpdateMethod<NaiveTime>,
    deadline: UpdateMethod<NaiveDate>,

    completed: UpdateMethod<bool>,
    logged: UpdateMethod<bool>,

    area_id: UpdateMethod<Uuid>,
    project_id: UpdateMethod<Uuid>,
    tag_ids: UpdateMethod<Vec<Uuid>>,

    deleted: UpdateMethod<bool>,

    #[serde(default = "chrono::Local::now")]
    timestamp: DateTime<Local>,
}

impl UpdateRequest {
    pub fn is_empty(&self) -> bool {
        self.title.is_noop()
            && self.notes.is_noop()
            && self.start_date.is_noop()
            && self.start_time.is_noop()
            && self.deadline.is_noop()
            && self.completed.is_noop()
            && self.logged.is_noop()
            && self.area_id.is_noop()
            && self.project_id.is_noop()
            && self.tag_ids.is_noop()
            && self.deleted.is_noop()
    }

    pub fn tag_ids(&self) -> Option<&[Uuid]> {
        if let UpdateMethod::Set(ref tag_ids) = self.tag_ids {
            Some(tag_ids)
        } else {
            None
        }
    }
}

impl ToSqlQueryBuilder for UpdateRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.add_column(DatabaseModel::UPDATED, &self.timestamp);
        builder.set_return(&[DatabaseModel::ID]);

        if !self.title.is_noop() {
            builder.add_column(DatabaseModel::TITLE, &self.title);
        }
        if !self.notes.is_noop() {
            builder.add_column(DatabaseModel::NOTES, &self.notes);
        }
        if !self.start_date.is_noop() {
            builder.add_column(DatabaseModel::START_DATE, &self.start_date);
        }
        if !self.start_time.is_noop() {
            builder.add_column(DatabaseModel::START_TIME, &self.start_time);
        }
        if !self.deadline.is_noop() {
            builder.add_column(DatabaseModel::DEADLINE, &self.deadline);
        }

        if !self.completed.is_noop() {
            match self.completed {
                UpdateMethod::Set(true) => {
                    builder.add_column(DatabaseModel::COMPLETED, &self.timestamp);
                }
                UpdateMethod::Set(false) | UpdateMethod::Remove => {
                    builder.add_column(DatabaseModel::COMPLETED, &None::<DateTime<Local>>);
                }
                UpdateMethod::NoOp => unreachable!(),
            };
        }
        if !self.logged.is_noop() {
            match self.logged {
                UpdateMethod::Set(true) => {
                    builder.add_column(DatabaseModel::LOGGED, &self.timestamp)
                }
                UpdateMethod::Set(false) | UpdateMethod::Remove => {
                    builder.add_column(DatabaseModel::LOGGED, &None::<DateTime<Local>>)
                }
                UpdateMethod::NoOp => unreachable!(),
            };
        }

        if !self.area_id.is_noop() {
            builder.add_column(DatabaseModel::AREA_ID, &self.area_id);
        }
        if !self.project_id.is_noop() {
            builder.add_column(DatabaseModel::PROJECT_ID, &self.project_id);
        }

        if !self.deleted.is_noop() {
            match self.deleted {
                UpdateMethod::Set(true) => {
                    builder.add_column(DatabaseModel::DELETED, &self.timestamp)
                }
                UpdateMethod::Set(false) | UpdateMethod::Remove => {
                    builder.add_column(DatabaseModel::DELETED, &None::<DateTime<Local>>)
                }
                UpdateMethod::NoOp => unreachable!(),
            };
        }

        builder
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QueryRequest {
    title: Option<QueryMethod<String>>,
    notes: Option<QueryMethod<String>>,
    start_date: Option<QueryMethod<NaiveDate>>,
    start_time: Option<QueryMethod<NaiveTime>>,
    deadline: Option<QueryMethod<NaiveDate>>,

    completed: Option<bool>,
    logged: Option<bool>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    tag_ids: Vec<Uuid>,

    deleted: Option<bool>,
}

impl QueryRequest {
    pub fn tag_ids(&self) -> &[Uuid] {
        &self.tag_ids
    }
}

impl ToSqlQueryBuilder for QueryRequest {
    fn to_sql_builder(&self) -> SqlQueryBuilder<'_> {
        let mut builder = SqlQueryBuilder::new(DatabaseModel::TABLE);
        builder.set_return_all();

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
            builder.add_condition(DatabaseModel::TITLE, cmp, q);
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
            builder.add_condition(DatabaseModel::NOTES, cmp, q);
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
            builder.add_condition(DatabaseModel::START_DATE, cmp, q);
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
            builder.add_condition(DatabaseModel::START_TIME, cmp, q);
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
            builder.add_condition(DatabaseModel::DEADLINE, cmp, q);
        }

        if let Some(b) = self.completed {
            if b {
                builder.add_condition(DatabaseModel::COMPLETED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(DatabaseModel::COMPLETED, PostgresCmp::IsNull, &NULL);
            }
        }
        if let Some(b) = self.logged {
            if b {
                builder.add_condition(DatabaseModel::LOGGED, PostgresCmp::NotNull, &NULL);
            } else {
                builder.add_condition(DatabaseModel::LOGGED, PostgresCmp::IsNull, &NULL);
            }
        }

        if let Some(ref i) = self.area_id {
            builder.add_condition(DatabaseModel::AREA_ID, PostgresCmp::Equal, i);
        }
        if let Some(ref i) = self.project_id {
            builder.add_condition(DatabaseModel::PROJECT_ID, PostgresCmp::Equal, i);
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
    use chrono::Local;
    use uuid::Uuid;

    use crate::util::ToSqlQueryBuilder;

    use super::CreateRequest;

    #[test]
    fn text_only() {
        let mut schema = CreateRequest::default();
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

        let mut schema = CreateRequest::default();
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
        let mut schema = CreateRequest::default();
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

        let mut schema = CreateRequest::default();
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
}

#[cfg(test)]
mod update_schema {
    use chrono::Local;
    use uuid::Uuid;

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
        schema.title = UpdateMethod::Set("Test Title".to_string());
        schema.notes = UpdateMethod::Set("Test Note".to_string());

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

        let mut schema = UpdateRequest::default();
        schema.start_date = UpdateMethod::Set(now.date_naive());
        schema.start_time = UpdateMethod::Set(now.time());
        schema.deadline = UpdateMethod::Set(now.date_naive());

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, start_date=$2, start_time=$3, deadline=$4 RETURNING task_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn bool_t_only() {
        let mut schema = UpdateRequest::default();
        schema.completed = UpdateMethod::Set(true);
        schema.logged = UpdateMethod::Set(true);
        schema.deleted = UpdateMethod::Set(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, completed_on=$2, logged_on=$3, deleted_on=$4 RETURNING task_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = UpdateRequest::default();
        schema.completed = UpdateMethod::Set(false);
        schema.logged = UpdateMethod::Set(false);
        schema.deleted = UpdateMethod::Set(false);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, completed_on=$2, logged_on=$3, deleted_on=$4 RETURNING task_id"
        );
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn id_only() {
        let mut schema = UpdateRequest::default();
        schema.area_id = UpdateMethod::Set(Uuid::new_v4());
        schema.project_id = UpdateMethod::Set(Uuid::new_v4());

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

        let mut schema = UpdateRequest::default();
        schema.title = UpdateMethod::Set("Test Title".to_string());
        schema.notes = UpdateMethod::Set("Test Note".to_string());
        schema.start_date = UpdateMethod::Set(now.date_naive());
        schema.start_time = UpdateMethod::Set(now.time());
        schema.deadline = UpdateMethod::Set(now.date_naive());
        schema.completed = UpdateMethod::Set(true);
        schema.logged = UpdateMethod::Set(true);
        schema.area_id = UpdateMethod::Set(Uuid::new_v4());
        schema.project_id = UpdateMethod::Set(Uuid::new_v4());
        schema.deleted = UpdateMethod::Set(true);

        let (statement, params) = schema.to_sql_builder().build_update();

        assert_eq!(
            statement.as_str(),
            "UPDATE data.tasks SET updated_on=$1, task_title=$2, notes=$3, start_date=$4, start_time=$5, deadline=$6, completed_on=$7, logged_on=$8, area_id=$9, project_id=$10, deleted_on=$11 RETURNING task_id"
        );
        assert_eq!(params.len(), 11);
    }
}

#[cfg(test)]
mod query_schema {
    use chrono::Local;
    use uuid::Uuid;

    use crate::{models::Compare, util::ToSqlQueryBuilder};

    use super::{QueryMethod, QueryRequest};

    #[test]
    fn empty() {
        let schema = QueryRequest::default();

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(statement.as_str(), "SELECT * FROM data.tasks");
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn text_only() {
        let mut schema = QueryRequest::default();
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

        let mut schema = QueryRequest::default();
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

        let mut schema = QueryRequest::default();
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
        let mut schema = QueryRequest::default();
        schema.completed = Some(true);
        schema.logged = Some(true);
        schema.deleted = Some(true);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE completed_on NOT NULL AND logged_on NOT NULL AND deleted_on NOT NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn bool_f_only() {
        let mut schema = QueryRequest::default();
        schema.completed = Some(false);
        schema.logged = Some(false);
        schema.deleted = Some(false);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE completed_on IS NULL AND logged_on IS NULL AND deleted_on IS NULL"
        );
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn id_only() {
        let mut schema = QueryRequest::default();
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

        let mut schema = QueryRequest::default();
        schema.title = Some(QueryMethod::Match("Test Title".to_string()));
        schema.notes = Some(QueryMethod::Match("Test Note".to_string()));
        schema.start_date = Some(QueryMethod::Match(now.date_naive()));
        schema.start_time = Some(QueryMethod::Match(now.time()));
        schema.deadline = Some(QueryMethod::Compare(now.date_naive(), Compare::Greater));
        schema.completed = Some(false);
        schema.logged = Some(true);
        schema.area_id = Some(Uuid::new_v4());
        schema.project_id = Some(Uuid::new_v4());
        schema.deleted = Some(false);

        let (statement, params) = schema.to_sql_builder().build_select();

        assert_eq!(
            statement.as_str(),
            "SELECT * FROM data.tasks WHERE task_title ILIKE '%' || $1 || '%' AND notes ILIKE '%' || $2 || '%' AND start_date = $3 AND start_time = $4 AND deadline > $5 AND completed_on IS NULL AND logged_on NOT NULL AND area_id = $6 AND project_id = $7 AND deleted_on IS NULL"
        );
        assert_eq!(params.len(), 7);
    }
}
