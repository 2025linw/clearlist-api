use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Default))]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct LoginDetails {
    pub email: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Claim {
    pub iss: String,
    #[serde(with = "uuid::serde::simple")]
    pub sub: Uuid,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
}

impl Claim {
    pub fn new(sub: Uuid, exp: u64) -> Self {
        Self {
            sub,
            exp,
            ..Default::default()
        }
    }
}

impl Default for Claim {
    fn default() -> Self {
        let iat = Utc::now();
        let exp = iat + Duration::hours(1);

        Self {
            iss: "todo-app-auth".to_string(),
            sub: Uuid::nil(),
            aud: "todo-app-api".to_string(),
            iat: iat.timestamp() as u64,
            exp: exp.timestamp() as u64,
        }
    }
}
