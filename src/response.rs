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
}

impl From<Error> for Response {
    fn from(value: Error) -> Self {
        match value {
            Error::InvalidRequest(msg) => Self::new(StatusCode::BAD_REQUEST).status(ERR).msg(&msg),
        }
    }
}

impl From<DBError> for Response {
    fn from(value: DBError) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
            .status(ERR)
            .msg(&value.to_string())
    }
}

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        if self.json_map.is_empty() {
            (self.code).into_response()
        } else {
            (self.code, Json::from(self.json_map)).into_response()
        }
    }
}
