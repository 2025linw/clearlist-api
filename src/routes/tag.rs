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
    data::{create_tag, delete_tag, query_tag, retrieve_tag, update_tag},
    error::ErrorResponse,
    models::{
        FilterOptions, ToResponse,
        jwt::Claim,
        tag::{CreateRequest, QueryRequest, ResponseModel, UpdateRequest},
    },
    response::{ERR, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "tag not found";
const NO_UPDATES: &str = "no tag updates were requested";

pub async fn create_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let tag_id = create_tag(&mut conn, user_id, body).await?;

    let tag = match retrieve_tag(&conn, tag_id, user_id).await? {
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
        json!(tag.to_response()),
    ))
}

pub async fn retrieve_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let tag = match retrieve_tag(&conn, tag_id, user_id).await? {
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
        json!(tag.to_response()),
    ))
}

pub async fn update_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
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

    let tag_id = match update_tag(&mut conn, tag_id, user_id, body).await? {
        Some(t) => {
            assert_eq!(
                t, tag_id,
                "error occured with query, as the tag ids do not match after update"
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

    let tag = match retrieve_tag(&conn, tag_id, user_id).await? {
        Some(t) => t,
        None => unreachable!("tag should exist after update"),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(tag.to_response()),
    ))
}

pub async fn delete_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(tag_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if delete_tag(&mut conn, tag_id, user_id).await?.is_none() {
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

    let tags = query_tag(&conn, user_id, body, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": tags.len(),
            "tags": tags.into_iter().map(|t| t.to_response()).collect::<Vec<ResponseModel>>(),
        }),
    ))
}
