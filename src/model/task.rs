use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use super::{
    ToResponse,
    tag::{TagModel, TagResponseModel},
};

/// Task Database Model
#[derive(Debug, Deserialize)]
pub struct TaskModel {
    task_id: Uuid,

    task_title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    completed_on: Option<DateTime<Local>>,
    logged_on: Option<DateTime<Local>>,
    trashed_on: Option<DateTime<Local>>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl TaskModel {
    pub const TABLE: &str = "data.tasks";

    pub const ID: &str = "task_id";

    pub const TITLE: &str = "task_title";
    pub const NOTES: &str = "notes";
    pub const START_DATE: &str = "start_date";
    pub const START_TIME: &str = "start_time";
    pub const DEADLINE: &str = "deadline";

    pub const COMPLETED: &str = "completed_on";
    pub const LOGGED: &str = "logged_on";
    pub const TRASHED: &str = "trashed_on";

    pub const AREA_ID: &str = "area_id";
    pub const PROJECT_ID: &str = "project_id";

    pub const USER_ID: &str = "user_id";
    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl TaskModel {
    pub fn task_id(&self) -> &Uuid {
        &self.task_id
    }
}

impl From<Row> for TaskModel {
    fn from(value: Row) -> Self {
        Self {
            task_id: value.get(Self::ID),
            task_title: value.get(Self::TITLE),
            notes: value.get(Self::NOTES),
            start_date: value.get(Self::START_DATE),
            start_time: value.get(Self::START_TIME),
            deadline: value.get(Self::DEADLINE),
            completed_on: value.get(Self::COMPLETED),
            logged_on: value.get(Self::LOGGED),
            trashed_on: value.get(Self::TRASHED),
            area_id: value.get(Self::AREA_ID),
            project_id: value.get(Self::PROJECT_ID),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for TaskModel {
    type Response = TaskResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.task_id,
            title: self.task_title.to_owned().unwrap_or_default(),
            notes: self.notes.to_owned().unwrap_or_default(),
            start_date: self.start_date,
            start_time: self.start_time,
            deadline: self.deadline,
            completed_on: self.completed_on,
            logged_on: self.logged_on,
            trashed_on: self.trashed_on,
            area_id: self.area_id,
            project_id: self.project_id,
            tags: Vec::new(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

/// Task Response Model
#[derive(Debug, Deserialize, Serialize)]
pub struct TaskResponseModel {
    id: Uuid,

    title: String,
    notes: String,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    completed_on: Option<DateTime<Local>>,
    logged_on: Option<DateTime<Local>>,
    trashed_on: Option<DateTime<Local>>,

    area_id: Option<Uuid>,
    project_id: Option<Uuid>,
    tags: Vec<TagResponseModel>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl TaskResponseModel {
    pub fn add_tags(&mut self, tags: Vec<TagModel>) -> &Self {
        self.tags.extend(tags.iter().map(|t| t.to_response()));

        self
    }
}

pub struct TaskTagModel {}

impl TaskTagModel {
    pub const TABLE: &str = "data.task_tags";
    pub const TASK_ID: &str = "task_id";
    pub const TAG_ID: &str = "tag_id";
}
