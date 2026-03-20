use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    com::model::{Tag, TagQuery, query::Pagination},
    db::tag::{delete_tag_soft, insert_tag, query_tags, select_tag, update_tag},
    error::Error,
    response::{ERR, ErrorResponse, OK, Response, SUCCESS},
    util::CurrentSession,
};

const NOT_FOUND: &str = "tag not found";

pub async fn query_handler(
    CurrentSession(user, _): CurrentSession,
    State(data): State<AppState>,
    Query(TagQuery {
        pagination: Pagination { page, limit },
        deleted,
    }): Query<TagQuery>,
) -> Result<Response, ErrorResponse> {
    let page = i64::try_from(page).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let limit = i64::try_from(limit).map_err(|e| Error::InvalidRequest(e.to_string()))?;
    let offset = (page - 1) * limit;

    let conn = data.db.get_pool_ref();

    let tags: Vec<Tag> = query_tags(conn, user.id, limit, offset, deleted).await?;

    Ok(Response::new(StatusCode::OK).status(OK).data(json!({
        "count": tags.len(),
        "tags": tags,
    })))
}

pub async fn create_handler(
    CurrentSession(user, _): CurrentSession,
    State(data): State<AppState>,
    Json(body): Json<Tag>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db.get_pool_ref();

    let tag_id = insert_tag(conn, user.id, body).await?;

    Ok(Response::new(StatusCode::CREATED)
        .status(SUCCESS)
        .data(json!({"tagId": tag_id})))
}

pub async fn retrieve_handler(
    CurrentSession(user, _): CurrentSession,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db.get_pool_ref();

    if let Some(tag) = select_tag(conn, tag_id, user.id).await? {
        Ok(Response::new(StatusCode::OK).status(OK).data(json!(tag)))
    } else {
        Err(Response::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}

pub async fn update_handler(
    CurrentSession(user, _): CurrentSession,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
    Json(body): Json<Tag>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db.get_pool_ref();

    if let Some(tag) = update_tag(conn, tag_id, user.id, body).await? {
        Ok(Response::new(StatusCode::OK)
            .status(SUCCESS)
            .data(json!(tag)))
    } else {
        Err(Response::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}

pub async fn delete_handler(
    CurrentSession(user, _): CurrentSession,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, ErrorResponse> {
    let conn = data.db.get_pool_ref();

    if let Some(()) = delete_tag_soft(conn, tag_id, user.id).await? {
        Ok(Response::new(StatusCode::NO_CONTENT))
    } else {
        Err(Response::new(StatusCode::NOT_FOUND)
            .status(ERR)
            .msg(NOT_FOUND))
    }
}
