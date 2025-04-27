use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use super::{
    ToResponse,
    tag::{TagModel, TagResponseModel},
};

/// Project Database Model
#[derive(Debug, Deserialize)]
pub struct ProjectModel {
    project_id: Uuid,

    project_title: Option<String>,
    notes: Option<String>,
    start_date: Option<NaiveDate>,
    start_time: Option<NaiveTime>,
    deadline: Option<NaiveDate>,

    completed_on: Option<DateTime<Local>>,
    logged_on: Option<DateTime<Local>>,
    trashed_on: Option<DateTime<Local>>,

    area_id: Option<Uuid>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl ProjectModel {
    pub const TABLE: &str = "data.projects";

    pub const ID: &str = "project_id";

    pub const TITLE: &str = "project_title";
    pub const NOTES: &str = "notes";
    pub const START_DATE: &str = "start_date";
    pub const START_TIME: &str = "start_time";
    pub const DEADLINE: &str = "deadline";

    pub const COMPLETED: &str = "completed_on";
    pub const LOGGED: &str = "logged_on";
    pub const TRASHED: &str = "trashed_on";

    pub const AREA_ID: &str = "area_id";

    pub const USER_ID: &str = "user_id";
    pub const CREATED: &str = "created_on";
    pub const UPDATED: &str = "updated_on";
}

impl ProjectModel {
    pub fn project_id(&self) -> &Uuid {
        &self.project_id
    }
}

impl From<Row> for ProjectModel {
    fn from(value: Row) -> Self {
        Self {
            project_id: value.get(Self::ID),
            project_title: value.get(Self::TITLE),
            notes: value.get(Self::NOTES),
            start_date: value.get(Self::START_DATE),
            start_time: value.get(Self::START_TIME),
            deadline: value.get(Self::DEADLINE),
            completed_on: value.get(Self::COMPLETED),
            logged_on: value.get(Self::LOGGED),
            trashed_on: value.get(Self::TRASHED),
            area_id: value.get(Self::AREA_ID),
            user_id: value.get(Self::USER_ID),
            created_on: value.get(Self::CREATED),
            updated_on: value.get(Self::UPDATED),
        }
    }
}

impl ToResponse for ProjectModel {
    type Response = ProjectResponseModel;

    fn to_response(&self) -> Self::Response {
        Self::Response {
            id: self.project_id,
            title: self.project_title.to_owned().unwrap_or_default(),
            notes: self.notes.to_owned().unwrap_or_default(),
            start_date: self.start_date,
            start_time: self.start_time,
            deadline: self.deadline,
            completed_on: self.completed_on,
            logged_on: self.logged_on,
            trashed_on: self.trashed_on,
            area_id: self.area_id,
            tags: Vec::new(),
            user_id: self.user_id,
            created_on: self.created_on,
            updated_on: self.updated_on,
        }
    }
}

/// Project Response Model
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectResponseModel {
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
    tags: Vec<TagResponseModel>,

    user_id: Uuid,
    created_on: DateTime<Local>,
    updated_on: DateTime<Local>,
}

impl ProjectResponseModel {
    pub fn add_tags(&mut self, tags: Vec<TagModel>) -> &Self {
        self.tags.extend(tags.iter().map(|t| t.to_response()));

        self
    }
}

pub struct ProjectTagModel {}

impl ProjectTagModel {
    pub const TABLE: &str = "data.project_tags";
    pub const PROJECT_ID: &str = "project_id";
    pub const TAG_ID: &str = "tag_id";
}
