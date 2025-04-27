use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use axum_jwt_auth::Claims;
use chrono::Local;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    error::Error,
    model::{
        ToResponse,
        tag::TagModel,
        task::{TaskModel, TaskResponseModel, TaskTagModel},
    },
    schema::{
        FilterOptions,
        auth::Claim,
        task::{CreateTaskSchema, QueryTaskSchema, UpdateTaskSchema},
    },
    util::{AddToQuery, Join, PostgresCmp, SQLQueryBuilder},
};

pub async fn create_task_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Json(body): Json<CreateTaskSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Create task
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_column(TaskModel::USER_ID, &user_id);
    body.add_to_query(&mut query_builder);
    query_builder.set_return(vec![TaskModel::ID]);

    let (statement, params) = query_builder.build_insert();

    let row = transaction
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let task_id: Uuid = row.get(TaskModel::ID);

    // Add tags
    if let Some(ref v) = body.tag_ids {
        for tag in v {
            let mut query_builder = SQLQueryBuilder::new(TaskTagModel::TABLE);
            query_builder.add_column(TaskTagModel::TASK_ID, &task_id);
            query_builder.add_column(TaskTagModel::TAG_ID, tag);

            let (statement, params) = query_builder.build_insert();

            if transaction
                .execute(&statement, &params)
                .await
                .map_err(|e| Error::from(e).into())?
                != 1
            {
                return Err(Error::Internal("tag was not added to task".to_string()).into());
            }
        }
    }

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get created task
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TaskModel::ID, PostgresCmp::Equal, &task_id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row = conn
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let task = TaskModel::from(row);

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, TaskTagModel::TABLE, TaskTagModel::TAG_ID);
    query_builder.add_condition(TaskTagModel::TASK_ID, PostgresCmp::Equal, &task_id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "status": "success",
            "data": json!({
                "task": task.to_response().add_tags(tags),
            }),
        })),
    ))
}

pub async fn retrieve_task_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Retrieve task
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TaskModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row_opt = conn
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let task = match row_opt {
        Some(row) => TaskModel::from(row),
        None => {
            let json_message = json!({
                "status": "not found",
                "message": format!("task not found"),
            });

            return Err((StatusCode::NOT_FOUND, Json(json_message)));
        }
    };

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, TaskTagModel::TABLE, TaskTagModel::TAG_ID);
    query_builder.add_condition(TaskTagModel::TASK_ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

    Ok(Json(json!({
        "status": "success",
        "data": json!({
            "task": task.to_response().add_tags(tags),
        }),
    })))
}

pub async fn update_task_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTaskSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Update task
    let timestamp = Local::now();
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_column(TaskModel::UPDATED, &timestamp);
    body.add_to_query(&mut query_builder);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TaskModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return(vec![TaskModel::ID]);

    let (statement, params) = query_builder.build_update();

    let row_opt = transaction
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    if row_opt.is_none() {
        let json_message = json!({
            "status": "not found",
            "message": format!("task not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    // Update tags (first delete existing tags)
    if let Some(ref v) = body.tag_ids {
        let mut query_builder = SQLQueryBuilder::new(TaskTagModel::TABLE);
        query_builder.add_condition(TaskTagModel::TASK_ID, PostgresCmp::Equal, &id);

        let (statement, params) = query_builder.build_delete();

        transaction
            .execute(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        for tag in v {
            let mut query_builder = SQLQueryBuilder::new(TaskTagModel::TABLE);
            query_builder.add_column(TaskTagModel::TASK_ID, &id);
            query_builder.add_column(TaskTagModel::TAG_ID, tag);

            let (statement, params) = query_builder.build_insert();

            if transaction
                .execute(&statement, &params)
                .await
                .map_err(|e| Error::from(e).into())?
                != 1
            {
                return Err(Error::Internal("tag was not added to task".to_string()).into());
            }
        }
    }

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get updated task
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TaskModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row = conn
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let task = TaskModel::from(row);

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, TaskTagModel::TABLE, TaskTagModel::TAG_ID);
    query_builder.add_condition(TaskTagModel::TASK_ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

    Ok(Json(json!({
        "status": "success",
        "data": json!({
            "task": task.to_response().add_tags(tags),
        }),
    })))
}

pub async fn delete_task_handler(
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

    // Delete task
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(TaskModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return(vec![TaskModel::ID]);

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
            "status": "not found",
            "message": format!("task not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn query_task_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Query(opts): Query<FilterOptions>,
    Json(body): Json<QueryTaskSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Get pagination info
    let page = opts.page.unwrap_or(1);
    let limit = opts.limit.unwrap_or(25);
    let offset = (page - 1) * limit;

    // Query tasks
    let mut query_builder = SQLQueryBuilder::new(TaskModel::TABLE);
    body.add_to_query(&mut query_builder);
    query_builder.add_condition(TaskModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.set_limit(limit);
    query_builder.set_offset(offset);

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let mut tasks: Vec<TaskModel> = rows.iter().map(|r| TaskModel::from(r.to_owned())).collect();

    // Filter tasks by tags
    if let Some(ref v) = body.tag_ids {
        let num_tags = v.len() as i64;

        let mut query_builder = SQLQueryBuilder::new(TaskTagModel::TABLE);
        query_builder.set_return(vec![TaskTagModel::TASK_ID]);
        query_builder.add_condition(TaskTagModel::TAG_ID, PostgresCmp::In, v);
        query_builder.set_group_by(vec![TaskTagModel::TASK_ID]);
        query_builder.set_having(
            format!("COUNT(DISTINCT {})", TaskTagModel::TAG_ID).as_str(),
            PostgresCmp::Equal,
            &num_tags,
        );

        let (statement, params) = query_builder.build_select();

        let rows = conn
            .query(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        let task_ids: Vec<Uuid> = rows
            .iter()
            .map(|r| r.get::<&str, Uuid>(TaskTagModel::TASK_ID))
            .collect();

        tasks.retain(|t| task_ids.contains(t.task_id()));
    }

    // Get related tags
    let mut task_responses: Vec<TaskResponseModel> = Vec::new();
    for task in tasks {
        let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
        query_builder.add_join(Join::Inner, TaskTagModel::TABLE, TaskTagModel::TAG_ID);
        query_builder.add_condition(TaskTagModel::TASK_ID, PostgresCmp::Equal, task.task_id());
        query_builder.set_return_all();

        let (statement, params) = query_builder.build_select();

        let rows = conn
            .query(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

        let mut task_response = task.to_response();
        task_response.add_tags(tags);

        task_responses.push(task_response);
    }

    Ok(Json(json!({
        "status": "ok",
        "data": json!({
            "count": task_responses.len(),
            "tasks": task_responses,
        }),
    })))
}

// TEST: task handlers
