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
    data::{create_area, delete_area, query_area, retrieve_area, update_area},
    error::ErrorResponse,
    models::{
        FilterOptions, ToResponse,
        area::{CreateRequest, QueryRequest, ResponseModel, UpdateRequest},
        jwt::Claim,
    },
    response::{ERR, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "area not found";
const NO_UPDATES: &str = "no area updates were requested";

pub async fn create_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let area_id = create_area(&mut conn, user_id, body).await?;

    let area = match retrieve_area(&conn, area_id, user_id).await? {
        Some(a) => a,
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
        json!(area.to_response()),
    ))
}

pub async fn retrieve_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(area_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let area = match retrieve_area(&conn, area_id, user_id).await? {
        Some(a) => a,
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
        json!(area.to_response()),
    ))
}

pub async fn update_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(area_id): Path<Uuid>,
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

    let area_id = match update_area(&mut conn, area_id, user_id, body).await? {
        Some(a) => {
            assert_eq!(
                a, area_id,
                "error occured with query, as the area ids do not match after update"
            );

            a
        }
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    let area = match retrieve_area(&conn, area_id, user_id).await? {
        Some(a) => a,
        None => unreachable!("area should exist after update"),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(area.to_response()),
    ))
}

pub async fn delete_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(area_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if delete_area(&mut conn, area_id, user_id).await?.is_none() {
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

    let areas = query_area(&conn, user_id, body, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": areas.len(),
            "areas": areas.into_iter().map(|a| a.to_response()).collect::<Vec<ResponseModel>>(),
        }),
    ))
}
