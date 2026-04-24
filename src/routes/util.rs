//! # Route Utility Module
//!
//! This module contains utilities used in routes, such as session extractors and rate limiters

use std::{env, sync::LazyLock};

use axum::{
    extract::{
        FromRequest, FromRequestParts, Request,
        rejection::{JsonRejection, PathRejection},
    },
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use governor::{clock::QuantaInstant, middleware::NoOpMiddleware};
use serde_qs::axum::QsQueryRejection;
use sqlx::{FromRow, Postgres};
use tower_governor::{
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::SmartIpKeyExtractor,
};
use uuid::Uuid;

use crate::{AppState, routes::Error};

/// Key for cookie holding authorization session token
static COOKIE_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("COOKIE_KEY").unwrap_or("better-auth.session_token".to_string()));

/// Creates a rate limiter
///
/// # Arguments
///
/// * `num_requests`: request quota
/// * `refresh_rate`: rate in which quotas are replenished in quota
pub fn create_rate_limiter(
    num_requests: u32,
    refresh_rate: u64,
) -> GovernorConfig<SmartIpKeyExtractor, NoOpMiddleware<QuantaInstant>> {
    GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .burst_size(num_requests)
        .per_second(refresh_rate)
        .finish()
        .unwrap()
}

/// Session Type
///
/// Used for routes that require authorization
///
/// Implements FromRequestParts to allow for use as extractor in handlers
#[allow(dead_code)]
#[derive(Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Session {
    id: Uuid,
    token: String,
    user_id: Uuid,
    user_agent: Option<String>,
    ip_address: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl Session {
    /// Gets user_id from session
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
}

impl FromRequestParts<AppState> for Session {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_headers(&parts.headers);
        if cookies.get(&COOKIE_KEY).is_none() {
            return Err(Error::NotAuthorized);
        }

        let session_id = cookies
            .get(&COOKIE_KEY)
            .unwrap() // TODO: do not unwrap here
            .value_trimmed()
            .split('.')
            .next()
            .unwrap(); // TODO: or here

        let conn = state.db.pool();
        let session: Session = match sqlx::query_as::<Postgres, Session>(
            "SELECT * FROM auth.session WHERE token = $1",
        )
        .bind(session_id)
        .fetch_one(&conn)
        .await
        {
            Err(_) => return Err(Error::NotAuthorized),
            Ok(session) => session,
        };

        if session.expires_at < Utc::now() {
            return Err(Error::NotAuthorized);
        }

        Ok(session)
    }
}

/// Optional Session Type
///
/// Used for routes that don't require authorization
///
/// Implements FromRequestParts to allow for use as extractor in handlers
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

/// Custom Path extractor to customize error
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    axum::extract::Path<T>: FromRequestParts<S, Rejection = PathRejection>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(Error::InvalidRequest(rejection.body_text())),
        }
    }
}

/// Custom Json extractor to customize error
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    axum::Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(Error::InvalidRequest(rejection.body_text())),
        }
    }
}

/// Custom QsForm (Query) extractor to customize error
pub struct Query<T>(pub T);

impl<S, T> FromRequest<S> for Query<T>
where
    serde_qs::axum::QsForm<T>: FromRequest<S, Rejection = QsQueryRejection>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match serde_qs::axum::QsForm::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(Error::InvalidRequest(rejection.to_string())),
        }
    }
}
