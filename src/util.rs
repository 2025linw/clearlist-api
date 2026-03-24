use std::{env, sync::LazyLock};

use axum::{extract::FromRequestParts, http::StatusCode};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres};
use uuid::Uuid;

use crate::AppState;

static COOKIE_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("COOKIE_KEY").unwrap_or("better-auth.session_token".to_string()));

#[derive(Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub token: String,
    pub user_id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl FromRequestParts<AppState> for Session {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_headers(&parts.headers);
        if cookies.get(&COOKIE_KEY).is_none() {
            return Err((StatusCode::UNAUTHORIZED, "not authorized"));
        }

        let session_id = cookies
            .get(&COOKIE_KEY)
            .unwrap()
            .value_trimmed()
            .split('.')
            .next()
            .unwrap();

        let conn = state.db.pool();
        let session: Session = match sqlx::query_as::<Postgres, Session>(
            "SELECT * FROM auth.session WHERE token = $1",
        )
        .bind(session_id)
        .fetch_one(&conn)
        .await
        {
            Err(_) => return Err((StatusCode::UNAUTHORIZED, "session has expired")),
            Ok(session) => session,
        };

        if session.expires_at < Utc::now() {
            return Err((StatusCode::UNAUTHORIZED, "session has expired"));
        }

        Ok(session)
    }
}

pub type OptionalSession = Option<Session>;

impl FromRequestParts<AppState> for OptionalSession {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match Session::from_request_parts(parts, state).await {
            Ok(session) => Ok(Some(session)),
            Err(_) => Ok(None),
        }
    }
}
