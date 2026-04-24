//! # Task Route Handlers
//!
//! This module contains collection of route handler functions for tasks

pub mod tag;

use axum::{extract::State, http::StatusCode};
use serde_json::json;
use uuid::Uuid;

use super::{
    Error,
    models::{
        Completed, Pagination,
        task::{Filter, Model},
    },
    util::{Json, Path, Query, Session},
};
use crate::{
    AppState,
    com::constants::{DEFAULT_LIMIT, TASK_NOT_FOUND},
    db::{
        filters::DateFilter,
        task::{
            TaskQueryOptions, complete_task, delete_task, insert_task, query_tasks, restore_task,
            select_task, update_task,
        },
    },
    response::{Response, TaskResponse},
};

/// Task Query Handler
pub async fn query_handler(
    session: Session,
    State(data): State<AppState>,
    Query(Filter {
        pagination: Pagination { page, limit },
        sort_by,
        sort_order,
        start_date,
        deadline,
        completed,
        deleted,
    }): Query<Filter>,
) -> Result<Response, Error> {
    let page = if page < 1 { 1 } else { page };
    let limit = if limit < 1 { DEFAULT_LIMIT } else { limit };
    let offset = (page - 1) * limit;

    let start_filter = if let Some(start) = start_date {
        Some(DateFilter::try_from(start).map_err(Error::from)?)
    } else {
        None
    };
    let deadline_filter = if let Some(deadline) = deadline {
        Some(DateFilter::try_from(deadline).map_err(Error::from)?)
    } else {
        None
    };

    let opts = TaskQueryOptions {
        sort_order: (sort_by, sort_order).into(),
        limit,
        offset,
        completed,
        deleted,
        start_filter,
        deadline_filter,
    };

    let tasks = query_tasks(data.db.pool(), session.user_id(), Some(opts))
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::OK).data(json!({
        "count": tasks.len(),
        "tasks": tasks.into_iter().map(TaskResponse::from).collect::<Vec<TaskResponse>>(),
    })))
}

/// Task Create Handler
pub async fn create_handler(
    session: Session,
    State(data): State<AppState>,
    Json(body): Json<Model>,
) -> Result<Response, Error> {
    let task = insert_task(data.db.pool(), session.user_id(), body)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::CREATED).data(json!(TaskResponse::from(task))))
}

/// Task Retrieve Handler
pub async fn retrieve_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, Error> {
    if let Some(task) = select_task(data.db.pool(), task_id, session.user_id())
        .await
        .map_err(Error::from)?
    {
        Ok(Response::new(StatusCode::OK).data(json!(TaskResponse::from(task))))
    } else {
        Err(Error::NotFound(TASK_NOT_FOUND.to_string()))
    }
}

/// Task Update Handler
pub async fn update_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<Model>,
) -> Result<Response, Error> {
    let task = update_task(data.db.pool(), task_id, session.user_id(), body)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::OK).data(json!(TaskResponse::from(task))))
}

/// Task Delete Handler
pub async fn delete_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, Error> {
    if let Err(err) = delete_task(data.db.pool(), task_id, session.user_id())
        .await
        .map_err(Error::from)
    {
        if let Error::NotFound(msg) = err {
            return Ok(Response::new(StatusCode::NO_CONTENT).message(&msg));
        } else {
            return Err(err);
        }
    }

    Ok(Response::new(StatusCode::NO_CONTENT))
}

/// Task Restore Handler
pub async fn restore_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, Error> {
    restore_task(data.db.pool(), task_id, session.user_id())
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::NO_CONTENT))
}

/// Task Complete (and Uncomplete) Handler
pub async fn complete_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(Completed { completed }): Json<Completed>,
) -> Result<Response, Error> {
    complete_task(data.db.pool(), task_id, session.user_id(), completed)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::NO_CONTENT))
}

#[cfg(test)]
mod query {}

#[cfg(test)]
mod create {}

#[cfg(test)]
mod retrieve {}

#[cfg(test)]
mod update {}

#[cfg(test)]
mod delete {}
