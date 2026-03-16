use std::collections::HashMap;

use axum::{Json, http::StatusCode};
use serde_json::Value;

use crate::{db::Error as DBError, error::Error};

pub const OK: &str = "ok";
pub const SUCCESS: &str = "success";
pub const ERR: &str = "error";

pub struct Response {
    code: StatusCode,
    json_map: HashMap<String, serde_json::Value>,
}

pub type ErrorResponse = Response;

impl Response {
    pub fn new(code: StatusCode) -> Self {
        Self {
            code,
            json_map: HashMap::new(),
        }
    }

    pub fn add_kv(&mut self, key: &str, value: Value) {
        self.json_map.insert(key.to_string(), value);
    }

    pub fn status(mut self, status: &str) -> Self {
        self.add_kv("status", serde_json::to_value(status).unwrap());

        self
    }

    pub fn msg(mut self, msg: &str) -> Self {
        self.add_kv("message", serde_json::to_value(msg).unwrap());

        self
    }

    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.add_kv("data", data);

        self
    }

    // TODO: deprecate this? replace with status(), msg(), data() methods?
    pub fn with_msg(code: StatusCode, status: &str, msg: &str) -> Self {
        Self::new(code).status(status).msg(msg)
    }

    // TODO: deprecate this? replace with status(), msg(), data() methods?
    pub fn with_data(code: StatusCode, status: &str, data: serde_json::Value) -> Self {
        Self::new(code).status(status).data(data)
    }
}

impl From<Error> for Response {
    fn from(value: Error) -> Self {
        match value {
            Error::InvalidRequest(msg) => Self::with_msg(StatusCode::BAD_REQUEST, ERR, &msg),
        }
    }
}

impl From<DBError> for Response {
    fn from(value: DBError) -> Self {
        Self::with_msg(StatusCode::INTERNAL_SERVER_ERROR, ERR, &value.to_string())
    }
}

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        (self.code, Json::from(self.json_map)).into_response()
    }
}
