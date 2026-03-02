use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::json;
use serde_qs::axum::QsForm as Query;
use uuid::Uuid;

use crate::{
    AppState,
    com::model::{Pagination, Task, TaskQuery},
    db::task::{delete_task, insert_task, query_tasks, select_task, update_task},
    error::Error,
    response::{ERR, ErrorResponse, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "task not found";

pub async fn query_handler(
    State(data): State<AppState>,
    Query(TaskQuery {
        pagination: Pagination { page, limit },
        start_from,
        start_to,
        deadline_from,
        deadline_to,
    }): Query<TaskQuery>,
) -> Result<Response, ErrorResponse> {
    let page = i64::try_from(page).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let limit = i64::try_from(limit).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let offset = (page - 1) * limit;

    let conn = data.db_conn.get_conn().await?;

    let tasks: Vec<Task> = query_tasks(&conn, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": tasks.len(),
            "tasks": tasks,
        }),
    ))
}

pub async fn create_handler(
    State(data): State<AppState>,
    Json(body): Json<Task>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let task_id = insert_task(&mut conn, body).await?;

    Ok(Response::with_data(
        StatusCode::CREATED,
        SUCCESS,
        json!({"taskId": task_id}),
    ))
}

pub async fn retrieve_handler(
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    if let Some(task) = select_task(&conn, task_id).await? {
        Ok(Response::with_data(StatusCode::OK, OK, json!(task)))
    } else {
        Err(ErrorResponse::with_msg(
            StatusCode::NOT_FOUND,
            ERR,
            NOT_FOUND,
        ))
    }
}

pub async fn update_handler(
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<Task>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    if let Some(task) = update_task(&mut conn, task_id, body).await? {
        Ok(Response::with_data(StatusCode::OK, SUCCESS, json!(task)))
    } else {
        Err(ErrorResponse::with_msg(
            StatusCode::NOT_FOUND,
            ERR,
            NOT_FOUND,
        ))
    }
}

pub async fn delete_handler(
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    if let Some(()) = delete_task(&mut conn, task_id).await? {
        Ok(Response::code(StatusCode::NO_CONTENT))
    } else {
        // TODO: consider other reasons for this function to return none

        Err(ErrorResponse::with_msg(
            StatusCode::NOT_FOUND,
            ERR,
            NOT_FOUND,
        ))
    }
}
