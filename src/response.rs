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
    pub fn code(code: StatusCode) -> Self {
        Self {
            code,
            json_map: HashMap::new(),
        }
    }

    pub fn with_msg(code: StatusCode, status: &str, msg: &str) -> Self {
        let mut map = HashMap::new();
        map.insert("status".to_string(), serde_json::to_value(status).unwrap());
        map.insert("message".to_string(), serde_json::to_value(msg).unwrap());

        Self {
            code,
            json_map: map,
        }
    }

    pub fn with_data(code: StatusCode, status: &str, data: serde_json::Value) -> Self {
        let mut map = HashMap::new();
        map.insert("status".to_string(), serde_json::to_value(status).unwrap());
        map.insert("data".to_string(), data);

        Self {
            code,
            json_map: map,
        }
    }

    pub fn add_kv(&mut self, key: &str, value: Value) {
        self.json_map.insert(key.to_string(), value);
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
