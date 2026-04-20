//! # Tag Route Handlers
//!
//! This module contains collection of route handler functions for tags

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;

use super::{
    Error,
    models::{
        Pagination,
        tag::{Filter, Model},
    },
    util::Session,
};
use crate::{
    AppState,
    com::constants::{DEFAULT_LIMIT, TAG_NOT_FOUND},
    db::tag::{TagQueryOptions, delete_tag, insert_tag, query_tags, select_tag, update_tag},
    response::{Response, TagResponse},
};

/// Tag Query Handler
pub async fn query_handler(
    session: Session,
    State(data): State<AppState>,
    Query(Filter {
        pagination: Pagination { page, limit },
        sort_by,
        sort_order,
    }): Query<Filter>,
) -> Result<Response, Error> {
    let page = if page < 1 { 1 } else { page };
    let limit = if limit < 1 { DEFAULT_LIMIT } else { limit };
    let offset = (page - 1) * limit;

    let opts = TagQueryOptions {
        sort_order: (sort_by, sort_order).into(),
        limit: limit,
        offset: offset,
    };

    let tags = query_tags(data.db.pool(), session.user_id(), opts)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::OK).data(json!({
        "count": tags.len(),
        "tags": tags.into_iter().map(TagResponse::from).collect::<Vec<TagResponse>>(),
    })))
}

/// Tag Create Handler
pub async fn create_handler(
    session: Session,
    State(data): State<AppState>,
    Json(body): Json<Model>,
) -> Result<Response, Error> {
    let tag = insert_tag(data.db.pool(), session.user_id(), body)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::CREATED).data(json!(TagResponse::from(tag))))
}

/// Tag Retrieve Handler
pub async fn retrieve_handler(
    session: Session,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, Error> {
    if let Some(tag) = select_tag(data.db.pool(), tag_id, session.user_id())
        .await
        .map_err(Error::from)?
    {
        Ok(Response::new(StatusCode::OK).data(json!(TagResponse::from(tag))))
    } else {
        Err(Error::NotFound(TAG_NOT_FOUND.to_string()))
    }
}

/// Tag Update Handler
pub async fn update_handler(
    session: Session,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
    Json(body): Json<Model>,
) -> Result<Response, Error> {
    let tag = update_tag(data.db.pool(), tag_id, session.user_id(), body)
        .await
        .map_err(Error::from)?;

    Ok(Response::new(StatusCode::OK).data(json!(TagResponse::from(tag))))
}

/// Tag Delete Handler
pub async fn delete_handler(
    session: Session,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<Response, Error> {
    if let Err(err) = delete_tag(data.db.pool(), tag_id, session.user_id())
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

#[cfg(test)]
mod query_tests {}

#[cfg(test)]
mod create_tests {}

#[cfg(test)]
mod retrieve_tests {}

#[cfg(test)]
mod update_tests {}

#[cfg(test)]
mod delete_tests {}
