//! # Task Tag Route Handlers
//!
//! This module contains collection of route handler functions for task tags

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;

use super::{Error, Session};
use crate::{
    AppState,
    db::task::tag,
    response::{Response, TagResponse},
    routes::task::TaskTagQuery,
};

/// Task Tags Query Handler
pub async fn query_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, Error> {
    let tags = tag::query_task_tags(data.db.pool(), task_id, session.user_id())
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::OK).data(json!({
        "count": tags.len(),
        "tags": tags.into_iter().map(TagResponse::from).collect::<Vec<TagResponse>>(),
    })))
}

/// Task Tags Create Handler
pub async fn create_handler(
    session: Session,
    State(data): State<AppState>,
    Path(TaskTagQuery { task_id, tag_id }): Path<TaskTagQuery>,
) -> Result<Response, Error> {
    tag::insert_task_tag(data.db.pool(), task_id, session.user_id(), tag_id)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::NO_CONTENT))
}

/// Task Tags Update Handler
pub async fn update_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(tag_ids): Json<Vec<Uuid>>,
) -> Result<Response, Error> {
    tag::update_task_tags(data.db.pool(), task_id, session.user_id(), tag_ids)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::NO_CONTENT))
}

/// Task Tags Delete Handler
pub async fn delete_handler(
    session: Session,
    State(data): State<AppState>,
    Path(TaskTagQuery { task_id, tag_id }): Path<TaskTagQuery>,
) -> Result<Response, Error> {
    tag::delete_task_tag(data.db.pool(), task_id, session.user_id(), tag_id)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::NO_CONTENT))
}

#[cfg(test)]
mod query_tests {}

#[cfg(test)]
mod create_tests {}

#[cfg(test)]
mod update_tests {}

#[cfg(test)]
mod delete_tests {}
