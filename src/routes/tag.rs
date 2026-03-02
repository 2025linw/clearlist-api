use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    com::model::{Pagination, Tag},
    db::tag::{delete_tag, insert_tag, query_tags, select_tag, update_tag},
    error::Error,
    response::{ERR, ErrorResponse, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "tag not found";

pub async fn query_handler(
    State(data): State<AppState>,
    Query(Pagination { page, limit }): Query<Pagination>,
) -> Result<Response, ErrorResponse> {
    let page = i64::try_from(page).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let limit = i64::try_from(limit).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let offset = (page - 1) * limit;

    let conn = data.db_conn.get_conn().await?;

    let tags: Vec<Tag> = query_tags(&conn, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": tags.len(),
            "tags": tags,
        }),
    ))
}

pub async fn create_handler(
    State(data): State<AppState>,
    Json(body): Json<Tag>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let tag_id = insert_tag(&mut conn, body).await?;

    Ok(Response::with_data(
        StatusCode::CREATED,
        SUCCESS,
        json!({"tagId": tag_id}),
    ))
}

pub async fn retrieve_handler(
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    if let Some(tag) = select_tag(&conn, tag_id).await? {
        Ok(Response::with_data(StatusCode::OK, OK, json!(tag)))
    } else {
        Err(Response::with_msg(StatusCode::NOT_FOUND, ERR, NOT_FOUND))
    }
}

pub async fn update_handler(
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
    Json(body): Json<Tag>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    if let Some(tag) = update_tag(&mut conn, tag_id, body).await? {
        Ok(Response::with_data(StatusCode::OK, SUCCESS, json!(tag)))
    } else {
        Err(Response::with_msg(StatusCode::NOT_FOUND, ERR, NOT_FOUND))
    }
}

pub async fn delete_handler(
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    if let Some(()) = delete_tag(&mut conn, tag_id).await? {
        Ok(Response::code(StatusCode::NO_CONTENT))
    } else {
        // TODO: consider other reasons for this function to return none

        Err(Response::with_msg(StatusCode::NOT_FOUND, ERR, NOT_FOUND))
    }
}
