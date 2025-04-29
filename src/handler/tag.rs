use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use axum_jwt_auth::Claims;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    error::Error,
    model::{
        ToResponse,
        tag::{TagModel, TagResponseModel},
    },
    schema::{
        FilterOptions,
        auth::Claim,
        tag::{CreateTagSchema, QueryTagSchema, UpdateTagSchema},
    },
    util::{PostgresCmp, SQLQueryBuilder, ToSQLQueryBuilder},
};

pub async fn create_tag_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Json(body): Json<CreateTagSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Create tag
    let mut query_builder = body.to_sql_builder();
    query_builder.add_column(TagModel::USER_ID, &user_id);

    let (statement, params) = query_builder.build_insert();

    let row = transaction
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    let tag = TagModel::from(row);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "ok",
            "data": {
                "tag": tag.to_response(),
            },
        })),
    ))
}

pub async fn retrieve_tag_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Retrieve tag
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_condition(TagModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TagModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row_opt = conn
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get retrieved tag
    let tag = match row_opt {
        Some(row) => TagModel::from(row),
        None => {
            let json_message = json!({
                "status": "unsuccessful",
                "message": format!("tag not found"),
            });

            return Err((StatusCode::NOT_FOUND, Json(json_message)));
        }
    };

    Ok(Json(json!({
        "status": "success",
        "data": json!({
            "tag": tag.to_response(),
        }),
    })))
}

pub async fn update_tag_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTagSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // If no updates made
    if body.is_empty() {
        return Err(Error::InvalidRequest("no area details were updated".to_string()).into());
    }

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Update tag
    let mut query_builder = body.to_sql_builder();
    query_builder.add_condition(TagModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TagModel::ID, PostgresCmp::Equal, &id);

    let (statement, params) = query_builder.build_update();

    let row_opt = transaction
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    if row_opt.is_none() {
        let json_message = json!({
            "status": "unsuccessful",
            "message": format!("tag not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get updated tag
    let tag = TagModel::from(row_opt.unwrap());

    Ok(Json(json!({
        "status": "success",
        "data": json!({
            "tag": tag.to_response(),
        }),
    })))
}

pub async fn delete_tag_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Delete tag
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_condition(TagModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TagModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return(vec![TagModel::ID]);

    let (statement, params) = query_builder.build_delete();

    let row_opt = transaction
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    if row_opt.is_none() {
        let json_message = json!({
            "status": "unsuccessful",
            "message": format!("tag not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn query_tag_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Query(opts): Query<FilterOptions>,
    Json(body): Json<QueryTagSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Get pagination info
    let page = opts.page.unwrap_or(1);
    let limit = opts.limit.unwrap_or(25);
    let offset = (page - 1) * limit;

    // Query tags
    let mut query_builder = body.to_sql_builder();
    query_builder.add_condition(TagModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.set_limit(limit);
    query_builder.set_offset(offset);

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

    let tag_responses: Vec<TagResponseModel> = tags.iter().map(|a| a.to_response()).collect();

    Ok(Json(json!({
        "status": "ok",
        "data": json!({
            "count": tag_responses.len(),
            "tags": tag_responses,
        }),
    })))
}

// TEST: tag handlers
