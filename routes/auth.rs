use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::EncodingKey;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    data::{check_for_email, check_for_user, get_user_login, register_user},
    error::{ErrorResponse, LOGIN_EXISTS, LOGIN_FAILED},
    models::{
        ToResponse,
        auth::{
            LoginInfo,
            LoginRequest,
            TokenResponse,
            UserLogin, // ResetSchema,
        },
        jwt::RefreshToken,
    },
    response::{ERR, Response, SUCCESS},
    util::auth::{create_jwt, hash_password, verify_jwt_and_get_id, verify_password},
};

fn create_access_jwt(encode_key: &EncodingKey, user_id: Uuid) -> crate::error::Result<String> {
    create_jwt(
        encode_key,
        user_id,
        (Utc::now() + Duration::hours(1)).timestamp() as u64,
    )
}

fn create_refresh_jwt(encode_key: &EncodingKey, user_id: Uuid) -> crate::error::Result<String> {
    create_jwt(
        encode_key,
        user_id,
        (Utc::now() + Duration::weeks(1)).timestamp() as u64,
    )
}

const LOGIN_MISSING: &str = "missing email and/or password";
// const LOGIN_RESET: &str = "if account exists with given email, password reset link will be sent";

pub async fn registration_handler(
    State(data): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let mut conn = data.db_conn.get_conn().await?;

    // Check for email and password
    if body.email().is_empty() || body.password().is_empty() {
        return Err(ErrorResponse::with_msg(
            StatusCode::BAD_REQUEST,
            ERR,
            LOGIN_MISSING,
        ));
    }

    // Check if user exists
    if check_for_email(&conn, body.email()).await? {
        return Err(ErrorResponse::with_msg(
            StatusCode::CONFLICT,
            ERR,
            LOGIN_EXISTS,
        ));
    }
    let email = body.email().to_string();

    // Hash password
    let password_hash: String = match hash_password(body.password()) {
        Ok(h) => h,
        Err(e) => return Err(e.into()),
    };

    // Add user
    let login_info = LoginInfo::new(email, password_hash);

    register_user(&mut conn, login_info).await?;

    Ok(Response::empty(StatusCode::CREATED, SUCCESS))
}

pub async fn login_handler(
    State(data): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    // Check for email and password
    if body.email().is_empty() || body.password().is_empty() {
        return Err(ErrorResponse::with_msg(
            StatusCode::BAD_REQUEST,
            ERR,
            LOGIN_MISSING,
        ));
    }

    // Check if user exists
    if !check_for_email(&conn, body.email()).await? {
        return Err(ErrorResponse::with_msg(
            StatusCode::BAD_REQUEST,
            ERR,
            LOGIN_FAILED,
        ));
    }

    // Get user
    let login: UserLogin = get_user_login(&conn, body.email()).await?;

    // === BELOW IS PROTECTED INFO ===

    // Verify password
    verify_password(login.password_hash(), body.password())?;

    let mut response = login.to_response();

    // Get access JWT
    match create_access_jwt(&data.encode_key, login.user_id()) {
        Ok(s) => response.set_access_jwt(s),
        Err(e) => return Err(e.into()),
    };

    // Get refresh JWT
    // TODO: add an option for people not to get a refresh token somehow
    // For example, in a 'remember me' or 'keep me logged in' option
    match create_refresh_jwt(&data.encode_key, login.user_id()) {
        Ok(s) => response.set_refresh_jwt(s),
        Err(e) => return Err(e.into()),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(response),
    ))
}

pub async fn refresh_handler(
    State(data): State<AppState>,
    Json(body): Json<RefreshToken>,
) -> Result<impl IntoResponse, ErrorResponse> {
    let conn = data.db_conn.get_conn().await?;

    let token = body.refresh_jwt;

    // Verify jwt
    let user_id = verify_jwt_and_get_id(&data.decode_key, &token)?;

    // Check if user exists
    // TODO: what to do in this situation? if a user has a refresh token, they should've had an account
    // Unless they deleted their account
    // Return no account found?
    if !check_for_user(&conn, user_id).await? {
        return Err(ErrorResponse::with_msg(
            StatusCode::BAD_REQUEST,
            ERR,
            LOGIN_MISSING,
        ));
    }

    // Get access JWT
    let access_jwt: String = match create_access_jwt(&data.encode_key, user_id) {
        Ok(s) => s,
        Err(e) => return Err(e.into()),
    };

    let mut response = TokenResponse::new(access_jwt);

    // Get refresh JWT
    match create_refresh_jwt(&data.encode_key, user_id) {
        Ok(s) => response.set_refresh_jwt(s),
        Err(e) => return Err(e.into()),
    };

    Ok(Response::with_data(
        StatusCode::OK,
        SUCCESS,
        json!(response),
    ))
}

// pub async fn password_reset_handler(
//     State(_data): State<AppState>,
//     Json(_body): Json<ResetSchema>,
// ) -> Result<impl IntoResponse, ErrorResponse> {

//     println!("{LOGIN_RESET}");

//     Ok(StatusCode::NOT_IMPLEMENTED)
// }
