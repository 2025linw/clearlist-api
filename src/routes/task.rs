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
        db::{DateFilter, SortOrder as SortOrderDB},
        query::{Pagination, SortBy, SortOrder as SortOrderQuery},
    },
    db::task::{TaskQueryOptions, delete_task, insert_task, query_tasks, select_task, update_task},
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
        sort_by,
        sort_order,
        start_date,
        deadline,
        deleted,
    }): Query<TaskQuery>,
) -> Result<Response, ErrorResponse> {
    let page = i64::try_from(page).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let limit = i64::try_from(limit).map_err(|e| Error::InvalidRequest(e.to_string()))?;
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
    let sort_order = match sort_by {
        SortBy::Created => match sort_order {
            SortOrderQuery::Ascending => SortOrderDB::CreatedAsc,
            SortOrderQuery::Descending => SortOrderDB::CreatedDesc,
        },
        SortBy::Updated => match sort_order {
            SortOrderQuery::Ascending => SortOrderDB::UpdatedAsc,
            SortOrderQuery::Descending => SortOrderDB::UpdatedDesc,
        },
    };

    let opts = TaskQueryOptions {
        user_id: session.user_id,
        limit: Some(limit),
        offset: Some(offset),
        deleted,
        start_filter,
        deadline_filter,
        sort_order,
    };

    let tasks: Vec<Task> = query_tasks(data.db.pool(), opts)
        .await
        .map_err(Error::from)?;

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
    let task_id = insert_task(data.db.pool(), session.user_id, body)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::CREATED)
        .status(SUCCESS)
        .data(json!({"taskId": task_id})))
}

pub async fn retrieve_handler(
    session: Session,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    if let Some(task) = select_task(data.db.pool(), task_id, session.user_id)
        .await
        .map_err(Error::from)?
    {
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
    if let Some(task) = update_task(data.db.pool(), task_id, session.user_id, body)
        .await
        .map_err(Error::from)?
    {
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
    if let Some(()) = delete_task(data.db.pool(), task_id, session.user_id)
        .await
        .map_err(Error::from)?
    {
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
        error::Error,
        response::{ErrorResponse, OK, Response},
        routes::task::NOT_FOUND,
        util::Session,
    };

    pub async fn query_handler(
        session: Session,
        State(data): State<AppState>,
        Path(task_id): Path<Uuid>,
    ) -> Result<Response, ErrorResponse> {
        if !is_task_exists(data.db.pool(), task_id, session.user_id)
            .await
            .map_err(Error::from)?
        {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        let tags = tag::query_task_tags(data.db.pool(), task_id, session.user_id)
            .await
            .map_err(Error::from)?;

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
        if !is_task_exists(data.db.pool(), task_id, session.user_id)
            .await
            .map_err(Error::from)?
        {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::add_task_tag(data.db.pool(), task_id, tag_id)
            .await
            .map_err(Error::from)?
        {
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
        if !is_task_exists(data.db.pool(), task_id, session.user_id)
            .await
            .map_err(Error::from)?
        {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::update_task_tags(data.db.pool(), task_id, tag_ids)
            .await
            .map_err(Error::from)?
        {
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
        if !is_task_exists(data.db.pool(), task_id, session.user_id)
            .await
            .map_err(Error::from)?
        {
            return Err(ErrorResponse::new(StatusCode::NOT_FOUND).msg(NOT_FOUND));
        }

        if let Some(()) = tag::delete_task_tag(data.db.pool(), task_id, tag_id)
            .await
            .map_err(Error::from)?
        {
            Ok(Response::new(StatusCode::NO_CONTENT))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND))
        }
    }
}
