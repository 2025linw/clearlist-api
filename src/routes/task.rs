use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_jwt_auth::Claims;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    data::{create_task, delete_task, query_task, retrieve_task, update_task},
    error::ErrorResponse,
    models::{
        FilterOptions, ToResponse,
        jwt::Claim,
        task::{CreateRequest, QueryRequest, ResponseModel, UpdateRequest},
    },
    response::{ERR, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "task not found";
const NO_UPDATES: &str = "no task updates were requested";

pub async fn create_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let task_id = create_task(&mut conn, user_id, body).await?;

    let task = match retrieve_task(&conn, task_id, user_id).await? {
        Some(t) => t,
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    Ok(Response::with_data(
        StatusCode::CREATED,
        SUCCESS,
        json!(task.to_response()),
    ))
}

pub async fn retrieve_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let task = match retrieve_task(&conn, task_id, user_id).await? {
        Some(t) => t,
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(task.to_response()),
    ))
}

pub async fn update_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
    Json(body): Json<UpdateRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if body.is_empty() {
        return Err(ErrorResponse::with_msg(
            StatusCode::BAD_REQUEST,
            ERR,
            NO_UPDATES,
        ));
    }

    let task_id = match update_task(&mut conn, task_id, user_id, body).await? {
        Some(t) => {
            assert_eq!(
                t, task_id,
                "error occured with query, as the task ids do not match after update"
            );

            t
        }
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    let task = match retrieve_task(&conn, task_id, user_id).await? {
        Some(t) => t,
        None => unreachable!("task should exist after update"),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(task.to_response()),
    ))
}

pub async fn delete_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if delete_task(&mut conn, task_id, user_id).await?.is_none() {
        // TODO: consider other reasons for this function to return none

        return Err(ErrorResponse::with_msg(
            StatusCode::NOT_FOUND,
            ERR,
            NOT_FOUND,
        ));
    }

    Ok(Response::empty(StatusCode::NO_CONTENT, SUCCESS))
}

pub async fn query_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Query(opts): Query<FilterOptions>,
    Json(body): Json<QueryRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let page = opts.page.unwrap_or(1);
    let limit = opts.limit.unwrap_or(25);
    let offset = (page - 1) * limit;

    let tasks = query_task(&conn, user_id, body, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": tasks.len(),
            "tasks": tasks.into_iter().map(|t| t.to_response()).collect::<Vec<ResponseModel>>(),
        }),
    ))
}
