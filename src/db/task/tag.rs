//! # Task Tag Database Functions
//!
//! This module contains database functions for tags associated with tasks

use sqlx::{PgConnection, PgPool, QueryBuilder};
use uuid::Uuid;

use super::select_task_inner;
use crate::{
    db::{Error, Result, error::ApplicationError, query_as_wrapper},
    models::Tag,
};

/// Query database for all tags associated with a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to query tags for
/// * `user_id`: User ID of task owner
///
/// # Returns
///
/// List of tags associated with task `task_id`
pub async fn query_task_tags(pool: PgPool, task_id: Uuid, user_id: Uuid) -> Result<Vec<Tag>> {
    let mut conn = pool.acquire().await?;
    let tags = query_task_tags_inner(&mut conn, task_id, user_id).await?;
    conn.close().await?;

    Ok(tags)
}

/// Internal function for `query_task_tags`
///
/// Only used internally
async fn query_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    Ok(query_as_wrapper::<Tag>(
        "SELECT *
                FROM app.tags tg
                JOIN app.task_tags tt ON tg.id = tt.tag_id
                JOIN app.tasks t ON tt.task_id = t.id AND t.deleted_at IS NULL
                WHERE tt.task_id = $1 AND t.created_by = $2",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// Add a tag to a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to add tag to
/// * `user_id`: User ID of task owner
/// * `tag_id`: ID of tag to add to task
///
/// # Returns
///
/// Unit `()`
pub async fn insert_task_tag(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    insert_task_tag_inner(&mut tx, task_id, user_id, tag_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `insert_task_tag`
///
/// Only used internally
async fn insert_task_tag_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    if let Err(err) = sqlx::query("INSERT INTO app.task_tags (task_id, tag_id) VALUES ($1, $2)")
        .bind(task_id)
        .bind(tag_id)
        .execute(conn)
        .await
    {
        if let Some(db_err) = err.as_database_error() {
            match db_err.constraint() {
                Some("task_tags_task_id_fkey") => {
                    return Err(Error::Application(ApplicationError::TaskNotFound));
                }
                Some("task_tags_tag_id_fkey") => {
                    return Err(Error::Application(ApplicationError::TagNotFound));
                }
                Some("task_tags_pkey") => return Ok(()),
                _ => (),
            }
        }

        return Err(err.into());
    }

    Ok(())
}

/// Update all tags associated with a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to update tags for
/// * `user_id`: User ID of task owner
/// * `tag_ids`: ID of tags to update task with
///
/// # Returns
///
/// Unit `()`
pub async fn update_task_tags(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    update_task_tags_inner(&mut tx, task_id, user_id, tag_ids).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `update_task_tags`
///
/// Only used internally
pub(super) async fn update_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    let mut builder =
        QueryBuilder::new("WITH deleted AS (DELETE FROM app.task_tags WHERE task_id = ");
    builder.push_bind(task_id);
    builder.push(" RETURNING task_id) INSERT INTO app.task_tags (task_id, tag_id) SELECT COALESCE(deleted.task_id, ");
    builder.push_bind(task_id);
    builder.push("), unnest_tag FROM deleted RIGHT JOIN UNNEST(");
    builder.push_bind(&tag_ids);
    builder.push(") AS unnest_tag ON TRUE");

    if let Err(err) = builder.build().execute(conn.as_mut()).await {
        if let Some(db_err) = err.as_database_error() {
            match db_err.constraint() {
                Some("task_tags_task_id_fkey") => {
                    return Err(Error::Application(ApplicationError::TaskNotFound));
                }
                Some("task_tags_tag_id_fkey") => {
                    return Err(Error::Application(ApplicationError::TagNotFound));
                }
                Some("task_tags_pkey") => {
                    return query_task_tags_inner(conn, task_id, user_id).await;
                }
                _ => (),
            }
        }

        return Err(err.into());
    }

    query_task_tags_inner(conn, task_id, user_id).await
}

/// Delete a tag from a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to delete tag from
/// * `user_id`: User ID of task owner
/// * `tag_id`: ID of tag to add to task
///
/// # Returns
///
/// Unit `()`
pub async fn delete_task_tag(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    delete_task_tag_inner(&mut tx, task_id, user_id, tag_id).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `delete_task_tag`
///
/// Only used internally
pub async fn delete_task_tag_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
        .bind(task_id)
        .bind(tag_id)
        .execute(conn)
        .await?;

    Ok(())
}

#[cfg(test)]
mod query_tests {}

#[cfg(test)]
mod insert_tests {}

#[cfg(test)]
mod update_tests {}

#[cfg(test)]
mod delete_tests {}
