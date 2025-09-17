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
    data::{create_project, delete_project, query_project, retrieve_project, update_project},
    error::ErrorResponse,
    models::{
        FilterOptions, ToResponse,
        jwt::Claim,
        project::{CreateRequest, QueryRequest, ResponseModel, UpdateRequest},
    },
    response::{ERR, OK, Response, SUCCESS},
};

const NOT_FOUND: &str = "project not found";
const NO_UPDATES: &str = "no project updates were requested";

pub async fn create_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let project_id = create_project(&mut conn, user_id, body).await?;

    let project = match retrieve_project(&conn, project_id, user_id).await? {
        Some(p) => p,
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
        json!(project.to_response()),
    ))
}

pub async fn retrieve_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let project = match retrieve_project(&conn, project_id, user_id).await? {
        Some(p) => p,
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
        json!(project.to_response()),
    ))
}

pub async fn update_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(project_id): Path<Uuid>,
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

    let project_id = match update_project(&mut conn, project_id, user_id, body).await? {
        Some(p) => {
            assert_eq!(
                p, project_id,
                "error occured with query, as the project ids do not match after update"
            );

            p
        }
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    let project = match retrieve_project(&conn, project_id, user_id).await? {
        Some(p) => p,
        None => unreachable!("project should exist after update"),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(project.to_response()),
    ))
}

pub async fn delete_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if delete_project(&mut conn, project_id, user_id)
        .await?
        .is_none()
    {
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

    let projects = query_project(&conn, user_id, body, limit, offset).await?;

    Ok(Response::with_data(
        StatusCode::OK,
        OK,
        json!({
            "count": projects.len(),
            "projects": projects.into_iter().map(|p| p.to_response()).collect::<Vec<ResponseModel>>(),
        }),
    ))
}
