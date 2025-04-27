use std::fs;

use argon2::{Argon2, PasswordHash};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng};
use serde_json::json;

use crate::{
    AppState,
    error::Error,
    model::auth::UserModel,
    schema::auth::{Claim, LoginDetails},
    util::{PostgresCmp, SQLQueryBuilder},
};

const MISSING_LOGIN: &str = "missing email and/or password";
const ERROR_LOGIN: &str = "user with given email and password combination not found";

pub async fn register_user(
    State(data): State<AppState>,
    Json(body): Json<LoginDetails>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Check for email and password
    if body.email.is_none() || body.password.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": MISSING_LOGIN,
            })),
        ));
    }

    let email = body.email.unwrap();
    let password = body.password.unwrap();

    // Get database connection
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;

    // Check if user exists
    let mut query_builder = SQLQueryBuilder::new(UserModel::TABLE);
    query_builder.add_condition(UserModel::EMAIL, PostgresCmp::Equal, &email);

    let (statement, params) = query_builder.build_select();

    if conn
        .execute(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?
        != 0
    {
        return Err(
            Error::InvalidRequest("user with provided email already exists".to_string()).into(),
        );
    }

    // Hash password with Argon2
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::Internal(e.to_string()).into())?
        .to_string();

    // Create new user
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    let mut query_builder = SQLQueryBuilder::new(UserModel::TABLE);
    query_builder.add_column(UserModel::EMAIL, &email);
    query_builder.add_column(UserModel::PASS_HASH, &password_hash);

    let (statement, params) = query_builder.build_insert();

    transaction
        .execute(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    Ok(StatusCode::CREATED)
}

pub async fn login_user(
    State(data): State<AppState>,
    Json(body): Json<LoginDetails>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Check for email and password
    if body.email.is_none() || body.password.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": MISSING_LOGIN,
            })),
        ));
    }

    let email = body.email.unwrap();
    let password = body.password.unwrap();

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Check if user exists
    let mut query_builder = SQLQueryBuilder::new(UserModel::TABLE);
    query_builder.add_condition(UserModel::EMAIL, PostgresCmp::Equal, &email);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row_opt = conn
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let user = match row_opt {
        Some(row) => UserModel::from(row),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": ERROR_LOGIN,
                })),
            ));
        }
    };

    // Verify hash
    let parsed_hash = PasswordHash::new(user.password_hash())
        .map_err(|e| Error::Internal(e.to_string()).into())?;

    let verify_result = Argon2::default().verify_password(password.as_bytes(), &parsed_hash);
    if let Err(e) = verify_result {
        match e {
            password_hash::Error::Password => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "status": "error",
                        "message": ERROR_LOGIN,
                    })),
                ));
            }
            _ => {
                return Err(Error::Internal(e.to_string()).into());
            }
        };
    }

    // Encode key
    let private_key =
        fs::read("./privkey.der").map_err(|e| Error::Internal(e.to_string()).into())?;
    let key = EncodingKey::from_ed_der(&private_key);

    let header = Header::new(Algorithm::EdDSA);

    let exp = Utc::now() + Duration::weeks(1);
    let claims = Claim::new(user.user_id().to_owned(), exp.timestamp() as u64);

    let token = encode::<Claim>(&header, &claims, &key)
        .map_err(|e| Error::Internal(e.to_string()).into())?;

    Ok(token.into_response())
}

// TODO: Add update user

// TODO: Add delete user

// TEST: authentication handlers
