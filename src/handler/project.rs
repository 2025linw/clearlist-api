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
        project::{ProjectModel, ProjectResponseModel, ProjectTagModel},
        tag::TagModel,
    },
    schema::{
        FilterOptions,
        auth::Claim,
        project::{CreateProjectSchema, QueryProjectSchema, UpdateProjectSchema},
    },
    util::{AddToQuery, Join, PostgresCmp, SQLQueryBuilder},
};

pub async fn create_project_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Json(body): Json<CreateProjectSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Create project
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_column(ProjectModel::USER_ID, &user_id);
    body.add_to_query(&mut query_builder);
    query_builder.set_return(vec![ProjectModel::ID]);

    let (statement, params) = query_builder.build_insert();

    let row = transaction
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let project_id: Uuid = row.get(ProjectModel::ID);

    // Add tags
    if let Some(ref v) = body.tag_ids {
        for tag in v {
            let mut query_builder = SQLQueryBuilder::new(ProjectTagModel::TABLE);
            query_builder.add_column(ProjectTagModel::PROJECT_ID, &project_id);
            query_builder.add_column(ProjectTagModel::TAG_ID, tag);

            let (statement, params) = query_builder.build_insert();

            if transaction
                .execute(&statement, &params)
                .await
                .map_err(|e| Error::from(e).into())?
                != 1
            {
                return Err(Error::Internal("tag was not added to project".to_string()).into());
            }
        }
    }

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get created project
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(ProjectModel::ID, PostgresCmp::Equal, &project_id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row = conn
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let project = ProjectModel::from(row);

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, ProjectTagModel::TABLE, ProjectTagModel::TAG_ID);
    query_builder.add_condition(ProjectTagModel::PROJECT_ID, PostgresCmp::Equal, &project_id);
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
                "project": project.to_response().add_tags(tags),
            }),
        })),
    ))
}

pub async fn retrieve_project_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Retrieve project
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(ProjectModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row_opt = conn
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let project = match row_opt {
        Some(row) => ProjectModel::from(row),
        None => {
            let json_message = json!({
                "status": "not found",
                "message": format!("project not found"),
            });

            return Err((StatusCode::NOT_FOUND, Json(json_message)));
        }
    };

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, ProjectTagModel::TABLE, ProjectTagModel::TAG_ID);
    query_builder.add_condition(ProjectTagModel::PROJECT_ID, PostgresCmp::Equal, &id);
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
            "project": project.to_response().add_tags(tags),
        }),
    })))
}

pub async fn update_project_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection and start transaction
    let mut conn = data.get_conn().await.map_err(|e| e.into())?;
    let transaction = conn
        .transaction()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Update project
    let timestamp = Local::now();
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_column(ProjectModel::UPDATED, &timestamp);
    body.add_to_query(&mut query_builder);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(ProjectModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return(vec![ProjectModel::ID]);

    let (statement, params) = query_builder.build_update();

    let row_opt = transaction
        .query_opt(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    if row_opt.is_none() {
        let json_message = json!({
            "status": "not found",
            "message": format!("project not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    // Update tags (first delete existing tags)
    if let Some(ref v) = body.tag_ids {
        let mut query_builder = SQLQueryBuilder::new(ProjectTagModel::TABLE);
        query_builder.add_condition(ProjectTagModel::PROJECT_ID, PostgresCmp::Equal, &id);

        let (statement, params) = query_builder.build_delete();

        transaction
            .execute(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        for tag in v {
            let mut query_builder = SQLQueryBuilder::new(ProjectTagModel::TABLE);
            query_builder.add_column(ProjectTagModel::PROJECT_ID, &id);
            query_builder.add_column(ProjectTagModel::TAG_ID, tag);

            let (statement, params) = query_builder.build_insert();

            if transaction
                .execute(&statement, &params)
                .await
                .map_err(|e| Error::from(e).into())?
                != 1
            {
                return Err(Error::Internal("tag was not added to project".to_string()).into());
            }
        }
    }

    // Commit transaction
    transaction
        .commit()
        .await
        .map_err(|e| Error::from(e).into())?;

    // Get updated project
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(ProjectModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return_all();

    let (statement, params) = query_builder.build_select();

    let row = conn
        .query_one(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let project = ProjectModel::from(row);

    // Get related tags
    let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
    query_builder.add_join(Join::Inner, ProjectTagModel::TABLE, ProjectTagModel::TAG_ID);
    query_builder.add_condition(ProjectTagModel::PROJECT_ID, PostgresCmp::Equal, &id);
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
            "project": project.to_response().add_tags(tags),
        }),
    })))
}

pub async fn delete_project_handler(
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

    // Delete project
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.add_condition(ProjectModel::ID, PostgresCmp::Equal, &id);
    query_builder.set_return(vec![ProjectModel::ID]);

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
            "message": format!("project not found"),
        });

        return Err((StatusCode::NOT_FOUND, Json(json_message)));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn query_project_handler(
    Claims(claim): Claims<Claim>,
    State(data): State<AppState>,
    _jar: CookieJar,
    Query(opts): Query<FilterOptions>,
    Json(body): Json<QueryProjectSchema>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Get user id
    let user_id = claim.sub;

    // Get database connection
    let conn = data.get_conn().await.map_err(|e| e.into())?;

    // Get pagination info
    let page = opts.page.unwrap_or(1);
    let limit = opts.limit.unwrap_or(25);
    let offset = (page - 1) * limit;

    // Query projects
    let mut query_builder = SQLQueryBuilder::new(ProjectModel::TABLE);
    body.add_to_query(&mut query_builder);
    query_builder.add_condition(ProjectModel::USER_ID, PostgresCmp::Equal, &user_id);
    query_builder.set_limit(limit);
    query_builder.set_offset(offset);

    let (statement, params) = query_builder.build_select();

    let rows = conn
        .query(&statement, &params)
        .await
        .map_err(|e| Error::from(e).into())?;

    let mut projects: Vec<ProjectModel> = rows
        .iter()
        .map(|r| ProjectModel::from(r.to_owned()))
        .collect();

    // Filter projects by tags
    if let Some(ref v) = body.tag_ids {
        let num_tags = v.len() as i64;

        let mut query_builder = SQLQueryBuilder::new(ProjectTagModel::TABLE);
        query_builder.set_return(vec![ProjectTagModel::PROJECT_ID]);
        query_builder.add_condition(ProjectTagModel::TAG_ID, PostgresCmp::In, v);
        query_builder.set_group_by(vec![ProjectTagModel::PROJECT_ID]);
        query_builder.set_having(
            format!("COUNT(DISTINCT {})", ProjectTagModel::TAG_ID).as_str(),
            PostgresCmp::Equal,
            &num_tags,
        );

        let (statement, params) = query_builder.build_select();

        let rows = conn
            .query(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        let project_ids: Vec<Uuid> = rows
            .iter()
            .map(|r| r.get::<&str, Uuid>(ProjectTagModel::PROJECT_ID))
            .collect();

        projects.retain(|p| project_ids.contains(p.project_id()));
    }

    // Get related tags
    let mut project_responses: Vec<ProjectResponseModel> = Vec::new();
    for project in projects {
        let mut query_builder = SQLQueryBuilder::new(TagModel::TABLE);
        query_builder.add_join(Join::Inner, ProjectTagModel::TABLE, ProjectTagModel::TAG_ID);
        query_builder.add_condition(
            ProjectTagModel::PROJECT_ID,
            PostgresCmp::Equal,
            project.project_id(),
        );
        query_builder.set_return_all();

        let (statement, params) = query_builder.build_select();

        let rows = conn
            .query(&statement, &params)
            .await
            .map_err(|e| Error::from(e).into())?;

        let tags: Vec<TagModel> = rows.iter().map(|r| TagModel::from(r.to_owned())).collect();

        let mut project_response = project.to_response();
        project_response.add_tags(tags);

        project_responses.push(project_response);
    }

    Ok(Json(json!({
        "status": "ok",
        "data": json!({
            "count": project_responses.len(),
            "projects": project_responses,
        }),
    })))
}

// TEST: query handlers
