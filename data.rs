use deadpool_postgres::{Object, Transaction};
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    models::{area, auth, project, tag, task, user},
    util::{Join, PostgresCmp, SqlQueryBuilder, ToSqlQueryBuilder},
};

// Task
pub async fn create_task(
    conn: &mut Object,
    user_id: Uuid,
    schema: task::CreateRequest,
) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_column(task::DatabaseModel::USER_ID, &user_id);

    let (statement, params) = builder.build_insert();

    // Insert task
    let task_id: Uuid = transaction
        .query_one(&statement, &params)
        .await?
        .get(task::DatabaseModel::ID);

    // Insert task-tags
    if let Some(tag_ids) = schema.tag_ids() {
        update_task_tags(&transaction, task_id, tag_ids).await?;
    }

    transaction.commit().await?;

    Ok(task_id)
}

pub async fn retrieve_task(
    conn: &Object,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Option<task::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(task::DatabaseModel::TABLE);
    builder.add_condition(task::DatabaseModel::ID, PostgresCmp::Equal, &task_id);
    builder.add_condition(task::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_select();

    // Get task
    let mut task: task::DatabaseModel = match conn.query_opt(&statement, &params).await? {
        Some(r) => r.into(),
        None => return Ok(None),
    };

    // Get task-tags
    let tags = get_task_tags(conn, task_id).await?;

    task.set_tags(tags);

    Ok(Some(task))
}

pub async fn update_task(
    conn: &mut Object,
    task_id: Uuid,
    user_id: Uuid,
    schema: task::UpdateRequest,
) -> Result<Option<Uuid>> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_condition(task::DatabaseModel::ID, PostgresCmp::Equal, &task_id);
    builder.add_condition(task::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_update();

    // Update task
    let task_id: Uuid = match transaction.query_opt(&statement, &params).await? {
        Some(r) => r.get(task::DatabaseModel::ID),
        None => return Ok(None),
    };

    // Update task-tags
    if let Some(tag_ids) = schema.tag_ids() {
        update_task_tags(&transaction, task_id, tag_ids).await?;
    }

    transaction.commit().await?;

    Ok(Some(task_id))
}

pub async fn delete_task(conn: &mut Object, task_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    let mut builder = SqlQueryBuilder::new(task::DatabaseModel::TABLE);
    builder.add_condition(task::DatabaseModel::ID, PostgresCmp::Equal, &task_id);
    builder.add_condition(task::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_delete();

    // Delete task
    match transaction.execute(&statement, &params).await? {
        0 => return Ok(None),
        1 => (),
        n => {
            return Err(Error::Internal(format!(
                "unexpected number of tasks removed (expected: 1, actual: {n}), no changes commited"
            )));
        }
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub async fn query_task(
    conn: &Object,
    user_id: Uuid,
    schema: task::QueryRequest,
    limit: usize,
    offset: usize,
) -> Result<Vec<task::DatabaseModel>> {
    let mut builder = schema.to_sql_builder();
    builder.add_condition(task::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);
    builder.set_limit(limit);
    builder.set_offset(offset);

    let (statement, params) = schema.to_sql_builder().build_select();

    // Query tasks
    let tasks: Vec<task::DatabaseModel> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect();

    // Get all task ids with filtered tags
    let temp = schema.tag_ids();
    let num = temp.len() as i64;
    let mut builder = SqlQueryBuilder::new(task::tag::TABLE);
    builder.add_condition(task::tag::TAG_ID, PostgresCmp::In, &temp);
    builder.set_group_by(&[task::tag::TASK_ID]);
    builder.set_having("COUNT(tag_id)", PostgresCmp::Equal, &num);
    builder.set_return(&[task::tag::TASK_ID]);

    let (statement, params) = builder.build_select();

    let task_ids: Vec<Uuid> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.get(task::tag::TASK_ID))
        .collect();

    // Filter tasks
    let mut tasks: Vec<task::DatabaseModel> = tasks
        .into_iter()
        .filter(|t| task_ids.contains(t.id_as_ref()))
        .collect();

    // Get tags for each task
    for task in tasks.iter_mut() {
        let tags = get_task_tags(conn, task.id()).await?;

        task.set_tags(tags);
    }

    Ok(tasks)
}

async fn get_task_tags(conn: &Object, task_id: Uuid) -> Result<Vec<tag::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(task::tag::TABLE);
    builder.add_condition(task::tag::TASK_ID, PostgresCmp::Equal, &task_id);
    builder.add_join(Join::Right, tag::DatabaseModel::TABLE, task::tag::TAG_ID);

    let (statement, params) = builder.build_select();

    Ok(conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect())
}

async fn update_task_tags(
    transaction: &Transaction<'_>,
    task_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<()> {
    transaction
        .execute(
            &format!(
                "DELETE FROM {} WHERE {} = $1",
                task::tag::TABLE,
                task::tag::TASK_ID
            ),
            &[&task_id],
        )
        .await?;

    for tag_id in tag_ids {
        let mut builder = SqlQueryBuilder::new(task::tag::TABLE);
        builder.add_column(task::tag::TASK_ID, &task_id);
        builder.add_column(task::tag::TAG_ID, tag_id);

        let (statement, params) = builder.build_insert();

        transaction.execute(&statement, &params).await?;
    }

    Ok(())
}

// Project
pub async fn create_project(
    conn: &mut Object,
    user_id: Uuid,
    schema: project::CreateRequest,
) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_column(project::DatabaseModel::USER_ID, &user_id);

    let (statement, params) = builder.build_insert();

    // Insert project
    let project_id: Uuid = transaction
        .query_one(&statement, &params)
        .await?
        .get(project::DatabaseModel::ID);

    // Insert project-tags
    if let Some(tag_ids) = schema.tag_ids() {
        update_project_tags(&transaction, project_id, tag_ids).await?;
    }

    transaction.commit().await?;

    Ok(project_id)
}

pub async fn retrieve_project(
    conn: &Object,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<Option<project::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(project::DatabaseModel::TABLE);
    builder.add_condition(project::DatabaseModel::ID, PostgresCmp::Equal, &project_id);
    builder.add_condition(
        project::DatabaseModel::USER_ID,
        PostgresCmp::Equal,
        &user_id,
    );

    let (statement, params) = builder.build_select();

    // Get project
    let mut project: project::DatabaseModel = match conn.query_opt(&statement, &params).await? {
        Some(r) => r.into(),
        None => return Ok(None),
    };

    // Get project-tags
    let tags = get_project_tags(conn, project_id).await?;

    project.set_tags(tags);

    Ok(Some(project))
}

pub async fn update_project(
    conn: &mut Object,
    project_id: Uuid,
    user_id: Uuid,
    schema: project::UpdateRequest,
) -> Result<Option<Uuid>> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_condition(project::DatabaseModel::ID, PostgresCmp::Equal, &project_id);
    builder.add_condition(
        project::DatabaseModel::USER_ID,
        PostgresCmp::Equal,
        &user_id,
    );

    let (statement, params) = builder.build_update();

    // Update project
    let project_id: Uuid = match transaction.query_opt(&statement, &params).await? {
        Some(r) => r.get(project::DatabaseModel::ID),
        None => return Ok(None),
    };

    // Update project-tags
    if let Some(tag_ids) = schema.tag_ids() {
        update_project_tags(&transaction, project_id, tag_ids).await?;
    }

    transaction.commit().await?;

    Ok(Some(project_id))
}

pub async fn delete_project(
    conn: &mut Object,
    project_id: Uuid,
    user_id: Uuid,
) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    let mut builder = SqlQueryBuilder::new(project::DatabaseModel::TABLE);
    builder.add_condition(project::DatabaseModel::ID, PostgresCmp::Equal, &project_id);
    builder.add_condition(
        project::DatabaseModel::USER_ID,
        PostgresCmp::Equal,
        &user_id,
    );

    let (statement, params) = builder.build_delete();

    // Delete project
    match transaction.execute(&statement, &params).await? {
        0 => return Ok(None),
        1 => (),
        n => {
            return Err(Error::Internal(format!(
                "unexpected number of projects removed (expected: 1, actual: {n}), no changes commited"
            )));
        }
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub async fn query_project(
    conn: &Object,
    user_id: Uuid,
    schema: project::QueryRequest,
    limit: usize,
    offset: usize,
) -> Result<Vec<project::DatabaseModel>> {
    let mut builder = schema.to_sql_builder();
    builder.add_condition(
        project::DatabaseModel::USER_ID,
        PostgresCmp::Equal,
        &user_id,
    );
    builder.set_limit(limit);
    builder.set_offset(offset);

    let (statement, params) = schema.to_sql_builder().build_select();

    // Query projects
    let projects: Vec<project::DatabaseModel> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect();

    // Get all project ids with filtered tags
    let temp = schema.tag_ids();
    let num = temp.len() as i64;
    let mut builder = SqlQueryBuilder::new(project::tag::TABLE);
    builder.add_condition(project::tag::TAG_ID, PostgresCmp::In, &temp);
    builder.set_group_by(&[project::tag::PROJECT_ID]);
    builder.set_having("COUNT(tag_id)", PostgresCmp::Equal, &num);
    builder.set_return(&[project::tag::PROJECT_ID]);

    let (statement, params) = builder.build_select();

    let project_ids: Vec<Uuid> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.get(project::tag::PROJECT_ID))
        .collect();

    // Filter projects
    let mut projects: Vec<project::DatabaseModel> = projects
        .into_iter()
        .filter(|p| project_ids.contains(p.id_as_ref()))
        .collect();

    // Get tags for each project
    for project in projects.iter_mut() {
        let tags = get_project_tags(conn, project.id()).await?;

        project.set_tags(tags);
    }

    Ok(projects)
}

async fn get_project_tags(conn: &Object, project_id: Uuid) -> Result<Vec<tag::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(project::tag::TABLE);
    builder.add_condition(project::tag::PROJECT_ID, PostgresCmp::Equal, &project_id);
    builder.add_join(Join::Right, tag::DatabaseModel::TABLE, project::tag::TAG_ID);

    let (statement, params) = builder.build_select();

    Ok(conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect())
}

async fn update_project_tags(
    transaction: &Transaction<'_>,
    project_id: Uuid,
    tag_ids: &[Uuid],
) -> Result<()> {
    transaction
        .execute(
            &format!(
                "DELETE FROM {} WHERE {} = $1",
                project::tag::TABLE,
                project::tag::PROJECT_ID
            ),
            &[&project_id],
        )
        .await?;

    for tag_id in tag_ids {
        let mut builder = SqlQueryBuilder::new(project::tag::TABLE);
        builder.add_column(project::tag::PROJECT_ID, &project_id);
        builder.add_column(project::tag::TAG_ID, tag_id);

        let (statement, params) = builder.build_insert();

        transaction.execute(&statement, &params).await?;
    }

    Ok(())
}

// Area
pub async fn create_area(
    conn: &mut Object,
    user_id: Uuid,
    schema: area::CreateRequest,
) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_column(area::DatabaseModel::USER_ID, &user_id);

    let (statement, params) = builder.build_insert();

    // Insert area
    let area_id: Uuid = transaction
        .query_one(&statement, &params)
        .await?
        .get(area::DatabaseModel::ID);

    transaction.commit().await?;

    Ok(area_id)
}

pub async fn retrieve_area(
    conn: &Object,
    area_id: Uuid,
    user_id: Uuid,
) -> Result<Option<area::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(area::DatabaseModel::TABLE);
    builder.add_condition(area::DatabaseModel::ID, PostgresCmp::Equal, &area_id);
    builder.add_condition(area::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_select();

    // Get area
    let area: area::DatabaseModel = match conn.query_opt(&statement, &params).await? {
        Some(r) => r.into(),
        None => return Ok(None),
    };

    Ok(Some(area))
}

pub async fn update_area(
    conn: &mut Object,
    area_id: Uuid,
    user_id: Uuid,
    schema: area::UpdateRequest,
) -> Result<Option<Uuid>> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_condition(area::DatabaseModel::ID, PostgresCmp::Equal, &area_id);
    builder.add_condition(area::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_update();

    // Update area
    let area_id: Uuid = match transaction.query_opt(&statement, &params).await? {
        Some(r) => r.get(area::DatabaseModel::ID),
        None => return Ok(None),
    };

    transaction.commit().await?;

    Ok(Some(area_id))
}

pub async fn delete_area(conn: &mut Object, area_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    let mut builder = SqlQueryBuilder::new(area::DatabaseModel::TABLE);
    builder.add_condition(area::DatabaseModel::ID, PostgresCmp::Equal, &area_id);
    builder.add_condition(area::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_delete();

    // Delete area
    match transaction.execute(&statement, &params).await? {
        0 => return Ok(None),
        1 => (),
        n => {
            return Err(Error::Internal(format!(
                "unexpected number of areas removed (expected: 1, actual: {n}), no changes commited"
            )));
        }
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub async fn query_area(
    conn: &Object,
    user_id: Uuid,
    schema: area::QueryRequest,
    limit: usize,
    offset: usize,
) -> Result<Vec<area::DatabaseModel>> {
    let mut builder = schema.to_sql_builder();
    builder.add_condition(area::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);
    builder.set_limit(limit);
    builder.set_offset(offset);

    let (statement, params) = schema.to_sql_builder().build_select();

    // Query areas
    let areas: Vec<area::DatabaseModel> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect();

    Ok(areas)
}

// Tag
pub async fn create_tag(
    conn: &mut Object,
    user_id: Uuid,
    schema: tag::CreateRequest,
) -> Result<Uuid> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_column(tag::DatabaseModel::USER_ID, &user_id);

    let (statement, params) = builder.build_insert();

    // Insert tag
    let tag_id: Uuid = transaction
        .query_one(&statement, &params)
        .await?
        .get(tag::DatabaseModel::ID);

    transaction.commit().await?;

    Ok(tag_id)
}

pub async fn retrieve_tag(
    conn: &Object,
    tag_id: Uuid,
    user_id: Uuid,
) -> Result<Option<tag::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(tag::DatabaseModel::TABLE);
    builder.add_condition(tag::DatabaseModel::ID, PostgresCmp::Equal, &tag_id);
    builder.add_condition(tag::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_select();

    // Get tag
    let tag: tag::DatabaseModel = match conn.query_opt(&statement, &params).await? {
        Some(r) => r.into(),
        None => return Ok(None),
    };

    Ok(Some(tag))
}

pub async fn update_tag(
    conn: &mut Object,
    tag_id: Uuid,
    user_id: Uuid,
    schema: tag::UpdateRequest,
) -> Result<Option<Uuid>> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_condition(tag::DatabaseModel::ID, PostgresCmp::Equal, &tag_id);
    builder.add_condition(tag::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_update();

    // Update tag
    let tag_id: Uuid = match transaction.query_opt(&statement, &params).await? {
        Some(r) => r.get(tag::DatabaseModel::ID),
        None => return Ok(None),
    };

    transaction.commit().await?;

    Ok(Some(tag_id))
}

pub async fn delete_tag(conn: &mut Object, tag_id: Uuid, user_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    let mut builder = SqlQueryBuilder::new(tag::DatabaseModel::TABLE);
    builder.add_condition(tag::DatabaseModel::ID, PostgresCmp::Equal, &tag_id);
    builder.add_condition(tag::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_delete();

    // Delete tag
    match transaction.execute(&statement, &params).await? {
        0 => return Ok(None),
        1 => (),
        n => {
            return Err(Error::Internal(format!(
                "unexpected number of tags removed (expected: 1, actual: {n}), no changes commited"
            )));
        }
    }

    transaction.commit().await?;

    Ok(Some(()))
}

pub async fn query_tag(
    conn: &Object,
    user_id: Uuid,
    schema: tag::QueryRequest,
    limit: usize,
    offset: usize,
) -> Result<Vec<tag::DatabaseModel>> {
    let mut builder = schema.to_sql_builder();
    builder.add_condition(tag::DatabaseModel::USER_ID, PostgresCmp::Equal, &user_id);
    builder.set_limit(limit);
    builder.set_offset(offset);

    let (statement, params) = schema.to_sql_builder().build_select();

    // Query tags
    let tags: Vec<tag::DatabaseModel> = conn
        .query(&statement, &params)
        .await?
        .into_iter()
        .map(|r| r.into())
        .collect();

    Ok(tags)
}

// Auth
pub async fn check_for_email(conn: &Object, email: &str) -> Result<bool> {
    let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
    builder.add_condition(user::DatabaseModel::EMAIL, PostgresCmp::Equal, &email);
    builder.set_return(&[user::DatabaseModel::EMAIL]);

    let (statement, params) = builder.build_select();

    match conn.query_opt(&statement, &params).await? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub async fn check_for_user(conn: &Object, user_id: Uuid) -> Result<bool> {
    let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
    builder.add_condition(user::DatabaseModel::ID, PostgresCmp::Equal, &user_id);
    builder.set_return(&[user::DatabaseModel::EMAIL]);

    let (statement, params) = builder.build_select();

    match conn.query_opt(&statement, &params).await? {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

pub async fn register_user(conn: &mut Object, schema: auth::LoginInfo) -> Result<()> {
    let transaction = conn.transaction().await?;

    let (statement, params) = schema.to_sql_builder().build_insert();

    transaction.query_one(&statement, &params).await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn get_user_login(conn: &Object, email: &str) -> Result<auth::UserLogin> {
    let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
    builder.add_condition(user::DatabaseModel::EMAIL, PostgresCmp::Equal, &email);
    builder.set_return(&[
        user::DatabaseModel::ID,
        user::DatabaseModel::EMAIL,
        user::DatabaseModel::PASS_HASH,
    ]);

    let (statement, params) = builder.build_select();

    // let login: auth::LoginInfo = match conn.query_opt(&statement, &params).await? {
    //     Some(r) => r.into(),
    //     None => return Ok(None),
    // };

    Ok(conn.query_one(&statement, &params).await?.into())
}

// User
pub async fn retrieve_user(conn: &Object, user_id: Uuid) -> Result<Option<user::DatabaseModel>> {
    let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
    builder.add_condition(user::DatabaseModel::ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_select();

    // Get user
    let user: user::DatabaseModel = match conn.query_opt(&statement, &params).await? {
        Some(r) => r.into(),
        None => return Ok(None),
    };

    Ok(Some(user))
}

pub async fn update_user(
    conn: &mut Object,
    user_id: Uuid,
    schema: user::UpdateRequest,
) -> Result<Option<Uuid>> {
    let transaction = conn.transaction().await?;

    let mut builder = schema.to_sql_builder();
    builder.add_condition(user::DatabaseModel::ID, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_update();

    // Update user
    let user_id: Uuid = match transaction.query_opt(&statement, &params).await? {
        Some(r) => r.get(user::DatabaseModel::ID),
        None => return Ok(None),
    };

    transaction.commit().await?;

    Ok(Some(user_id))
}

pub async fn delete_user(conn: &mut Object, user_id: Uuid) -> Result<Option<()>> {
    let transaction = conn.transaction().await?;

    let mut builder = SqlQueryBuilder::new(user::DatabaseModel::TABLE);
    builder.add_condition(user::DatabaseModel::TABLE, PostgresCmp::Equal, &user_id);

    let (statement, params) = builder.build_delete();

    // Delete user
    match transaction.execute(&statement, &params).await? {
        0 => return Ok(None),
        1 => (),
        n => {
            return Err(Error::Internal(format!(
                "unexpected number of users removed (expected: 1, actual: {n}), no changes commited"
            )));
        }
    }

    transaction.commit().await?;

    Ok(Some(()))
}
