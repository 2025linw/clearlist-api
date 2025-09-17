use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use serde_json::{Value, json};

pub const OK: &str = "ok";
pub const SUCCESS: &str = "success";
pub const ERR: &str = "error";

pub struct Response(StatusCode, Json<serde_json::Value>);

impl Response {
    pub fn empty(code: StatusCode, status: &str) -> Self {
        Self(
            code,
            Json(json!({
                "status": status,
            })),
        )
    }

    pub fn with_msg(code: StatusCode, status: &str, msg: &str) -> Self {
        Self(
            code,
            Json(json!({
                "status": status,
                "message": msg,
            })),
        )
    }

    pub fn with_data(code: StatusCode, status: &str, data: serde_json::Value) -> Self {
        Self(
            code,
            Json(json!({
                "status": status,
                "data": data,
            })),
        )
    }

    pub fn add_kv(mut self, key: &str, value: Value) -> Self {
        if let Value::Object(ref mut map) = self.1.0 {
            map.insert(key.to_string(), value);
        }

        self
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> AxumResponse {
        (self.0, self.1).into_response()
    }
}
