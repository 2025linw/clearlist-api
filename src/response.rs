//! # Response Module
//!
//! This module contains Response type representing response back to client from API

use axum::{Json, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    models::{Tag, Task},
    routes::models::Start,
};

/// Response type representing a JSON response
///
/// This can be converted into an Axum Response to be returned as JSON response with a status code
pub struct Response {
    code: StatusCode,
    message: Option<String>,
    data: Option<Value>,
    custom: Map<String, Value>,
}

impl Response {
    /// Creates a new empty body response with a StatusCode
    pub fn new(code: StatusCode) -> Self {
        Self {
            code,
            message: None,
            data: None,
            custom: Map::new(),
        }
    }

    /// Add message to response
    pub fn message(mut self, msg: &str) -> Self {
        self.message = Some(msg.to_string());

        self
    }

    /// Add data to response
    pub fn data(mut self, data: Value) -> Self {
        self.data = Some(data);

        self
    }

    pub(crate) fn add_kv(mut self, key: &str, value: Value) -> Self {
        self.custom.insert(key.to_string(), value);

        self
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        if self.code == StatusCode::NO_CONTENT {
            (self.code).into_response()
        } else {
            let mut body: Map<String, Value> = Map::new();

            if let Some(message) = self.message {
                body.insert("message".to_string(), Value::String(message));
            }

            if let Some(data) = self.data {
                body.insert("data".to_string(), data);
            }

            if !self.custom.is_empty() {
                for (key, value) in self.custom {
                    body.insert(key, value);
                }
            }

            (self.code, Json::from(Value::Object(body))).into_response()
        }
    }
}

/// Task Response Type
///
/// DTO model for Task
///
/// This model is returned back to the user in responses that contain task(s)
#[derive(Serialize)]
pub struct TaskResponse {
    pub id: uuid::Uuid,

    pub title: String,
    pub notes: Option<String>,
    pub start: Option<Start>,
    pub deadline: Option<chrono::NaiveDate>,
    pub tags: Vec<TagResponse>,

    pub completed_at: Option<chrono::DateTime<Utc>>,
    pub deleted_at: Option<chrono::DateTime<Utc>>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<Task> for TaskResponse {
    fn from(value: Task) -> Self {
        let start = if let Some(dt) = value.start_dt {
            if value.has_time {
                Some(Start::At(dt))
            } else {
                Some(Start::On(dt.date_naive()))
            }
        } else {
            None
        };

        Self {
            id: value.id,
            title: value.title,
            notes: value.notes,
            start,
            deadline: value.deadline,
            tags: value.tags.into_iter().map(TagResponse::from).collect(),
            completed_at: value.completed_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

/// Response type for Tag
///
/// DTO model for Tag
///
/// This model is returned back to the user in responses that contain tag(s)
#[derive(Serialize)]
pub struct TagResponse {
    pub id: uuid::Uuid,

    pub label: String,
    pub category: Option<String>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<Tag> for TagResponse {
    fn from(value: Tag) -> Self {
        Self {
            id: value.id,
            label: value.label,
            category: value.category,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
