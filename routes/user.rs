use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_jwt_auth::Claims;
use serde_json::json;

use crate::{
    AppState,
    data::{delete_user, retrieve_user, update_user},
    error::ErrorResponse,
    models::{ToResponse, jwt::Claim, user::UpdateRequest},
    response::{ERR, Response, SUCCESS},
};

const NOT_FOUND: &str = "user not found";
const NO_UPDATES: &str = "no user updates were requested";

pub async fn retrieve_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    let user = match retrieve_user(&conn, user_id).await? {
        Some(u) => u,
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
        json!(user.to_response()),
    ))
}

pub async fn update_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
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

    let user_id = match update_user(&mut conn, user_id, body).await? {
        Some(u) => {
            assert_eq!(
                u, user_id,
                "error occured with query, as the user ids do not match after update"
            );

            u
        }
        None => {
            return Err(ErrorResponse::with_msg(
                StatusCode::NOT_FOUND,
                ERR,
                NOT_FOUND,
            ));
        }
    };

    let user = match retrieve_user(&conn, user_id).await? {
        Some(u) => u,
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
        json!(user.to_response()),
    ))
}

pub async fn delete_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    let user_id = claim.sub;

    if delete_user(&mut conn, user_id).await?.is_none() {
        // TODO: consider other reasons for this function to return none

        return Err(ErrorResponse::with_msg(
            StatusCode::NOT_FOUND,
            ERR,
            NOT_FOUND,
        ));
    }

    Ok(Response::empty(StatusCode::NO_CONTENT, SUCCESS))
}
