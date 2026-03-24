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
    com::model::{
        Task, TaskQuery,
        db::SQLCmp,
        query::{DateFilter, Pagination},
    },
    db::task::{delete_task, insert_task, query_tasks, select_task, update_task},
    error::Error,
    response::{ERR, ErrorResponse, OK, Response, SUCCESS},
    util::Session,
};

const NOT_FOUND: &str = "task not found";

pub async fn query_handler(
    session: Session,
    State(data): State<AppState>,
    Query(TaskQuery {
        pagination: Pagination { page, limit },
        start_date,
        deadline,
        deleted,
    }): Query<TaskQuery>,
) -> Result<Response, ErrorResponse> {
    let page = i64::try_from(page).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let limit = i64::try_from(limit).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let offset = (page - 1) * limit;

    let start = if let Some(start) = start_date {
        match start {
            DateFilter::Exact(date) => Some(vec![(SQLCmp::Equal, date)]),
            DateFilter::BracketInterval(interval) => {
                let cmps = interval.get_cmps();

                // check if any comparisons are overspecified (i.e. lt and lte, or gt and gte)
                if cmps
                    .iter()
                    .filter(|(cmp, _)| {
                        matches!(cmp, SQLCmp::LessThan) || matches!(cmp, SQLCmp::LessThanEqual)
                    })
                    .count()
                    > 1
                {
                    return Err(Error::InvalidRequest(
                        "conflicting comparison operators for 'start_date'".to_string(),
                    )
                    .into());
                }

                Some(cmps)
            }
            DateFilter::ISO8601Interval([start, end]) => Some(vec![
                (SQLCmp::GreaterThanEqual, start),
                (SQLCmp::LessThanEqual, end),
            ]),
        }
    } else {
        None
    };
    let deadline = if let Some(deadline) = deadline {
        match deadline {
            DateFilter::Exact(date) => Some(vec![(SQLCmp::Equal, date)]),
            DateFilter::BracketInterval(interval) => {
                if !interval.is_valid() {
                    return Err(Error::InvalidRequest(
                        "conflicting comparison operators for 'deadline'".to_string(),
                    )
                    .into());
                }

                Some(interval.get_cmps())
            }
            DateFilter::ISO8601Interval([start, end]) => Some(vec![
                (SQLCmp::GreaterThanEqual, start),
                (SQLCmp::LessThanEqual, end),
            ]),
        }
    } else {
        None
    };

    let tasks: Vec<Task> = query_tasks(
        data.db.pool(),
        session.user_id,
        limit,
        offset,
        deleted,
        start,
        deadline,
    )
    .await?;

    Ok(Response::new(StatusCode::OK).status(OK).data(json!({
        "count": tasks.len(),
        "tasks": tasks,
    })))
}

pub async fn create_handler(
    session: Session,
    State(data): State<AppState>,
    Json(body): Json<Task>,
) -> Result<Response, ErrorResponse> {
    let task_id = insert_task(data.db.pool(), session.user_id, body).await?;

    Ok(Response::new(StatusCode::CREATED)
        .status(SUCCESS)
        .data(json!({"taskId": task_id})))
}

pub async fn retrieve_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    if let Some(task) = select_task(data.db.pool(), task_id, session.user_id).await? {
        Ok(Response::new(StatusCode::OK).status(OK).data(json!(task)))
    } else {
        Err(ErrorResponse::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}

pub async fn update_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<Task>,
) -> Result<Response, ErrorResponse> {
    if let Some(task) = update_task(data.db.pool(), task_id, session.user_id, body).await? {
        Ok(Response::new(StatusCode::OK)
            .status(SUCCESS)
            .data(json!(task)))
    } else {
        Err(ErrorResponse::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}

pub async fn delete_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    if let Some(()) = delete_task(data.db.pool(), task_id, session.user_id).await? {
        Ok(Response::new(StatusCode::NO_CONTENT))
    } else {
        Err(ErrorResponse::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}

pub mod tag {
    use axum::{
        Json,
        extract::{Path, State},
        http::StatusCode,
    };
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        AppState,
        com::model::query::PathTaskTag,
        db::{is_task_exists, task::tag},
        response::{ErrorResponse, OK, Response},
        routes::task::NOT_FOUND,
        util::Session,
    };

    pub async fn query_handler(
        session: Session,
        State(data): State<AppState>,
        Path(task_id): Path<Uuid>,
    ) -> Result<Response, ErrorResponse> {
        if !is_task_exists(data.db.pool(), task_id, session.user_id).await? {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        let tags = tag::query_task_tags(data.db.pool(), task_id, session.user_id).await?;

        Ok(Response::new(StatusCode::OK).status(OK).data(json!({
            "count": tags.len(),
            "tags": tags,
        })))
    }

    pub async fn create_handler(
        session: Session,
        State(data): State<AppState>,
        Path(PathTaskTag { task_id, tag_id }): Path<PathTaskTag>,
    ) -> Result<Response, ErrorResponse> {
        if !is_task_exists(data.db.pool(), task_id, session.user_id).await? {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::add_task_tag(data.db.pool(), task_id, tag_id).await? {
            Ok(Response::new(StatusCode::NO_CONTENT))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND))
        }
    }

    pub async fn update_handler(
        session: Session,
        State(data): State<AppState>,
        Path(task_id): Path<Uuid>,
        Json(tag_ids): Json<Vec<Uuid>>,
    ) -> Result<Response, ErrorResponse> {
        if !is_task_exists(data.db.pool(), task_id, session.user_id).await? {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::update_task_tags(data.db.pool(), task_id, tag_ids).await? {
            Ok(Response::new(StatusCode::NO_CONTENT))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND).msg("one or more tags do not exist"))
        }
    }

    pub async fn delete_handler(
        session: Session,
        State(data): State<AppState>,
        Path(PathTaskTag { task_id, tag_id }): Path<PathTaskTag>,
    ) -> Result<Response, ErrorResponse> {
        if !is_task_exists(data.db.pool(), task_id, session.user_id).await? {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::delete_task_tag(data.db.pool(), task_id, tag_id).await? {
            Ok(Response::new(StatusCode::NO_CONTENT))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND))
        }
    }
}
