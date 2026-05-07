//! # Task Tag Database Functions
//!
//! This module contains database functions for tags associated with tasks

use std::collections::HashSet;

use sqlx::{PgConnection, PgPool};
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

    query_task_tags_inner_unchecked(conn, task_id, user_id).await
}

/// Unchecked internal function for querying task tags
///
/// Caller must guarantee task existence and ownership to requesting user
pub(super) async fn query_task_tags_inner_unchecked(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Tag>> {
    Ok(query_as_wrapper::<Tag>(
        "SELECT tg.*
        FROM app.tags tg
        JOIN app.task_tags tt ON tg.id = tt.tag_id
        JOIN app.tasks t ON tt.task_id = t.id
        WHERE tt.task_id = $1 AND t.created_by = $2
        ORDER BY tg.category ASC NULLS FIRST, tg.label ASC, tg.updated_at DESC, tg.id ASC",
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_all(conn)
    .await?)
}

/// Add tags to a task
///
/// This function adds tasks in addition to the existing tags in the database
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to add tag to
/// * `user_id`: User ID of task owner
/// * `tag_ids`: ID of tags to add to task
///
/// # Returns
///
/// Unit `()`
pub async fn insert_task_tags(
    pool: PgPool,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    insert_task_tags_inner(&mut tx, task_id, user_id, tag_ids).await?;
    tx.commit().await?;

    Ok(())
}

/// Internal function for `insert_task_tag`
///
/// Only used internally
async fn insert_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    insert_task_tags_inner_unchecked(conn, task_id, user_id, tag_ids).await
}

/// Unchecked internal functino for inserting task tags
///
/// Caller must guarantee task existence and ownership to requesting user
pub(super) async fn insert_task_tags_inner_unchecked(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    let res = sqlx::query(
        "INSERT INTO app.task_tags (task_id, tag_id)
        SELECT $1, unnest_tag
        FROM UNNEST($2) AS unnest_tag
        ON CONFLICT DO NOTHING",
    )
    .bind(task_id)
    .bind(tag_ids)
    .execute(conn.as_mut())
    .await;

    let inserted = match res {
        Ok(done) => done.rows_affected(),
        Err(err) => {
            if let Some(db_err) = err.as_database_error()
                && let Some("task_tags_tag_id_fkey") = db_err.constraint()
            {
                return Err(Error::Application(ApplicationError::TagNotFound));
            }

            return Err(err.into());
        }
    };

    if inserted > 0 {
        sqlx::query(
            "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;
    }

    query_task_tags_inner(conn, task_id, user_id).await
}

/// Update all tags associated with a task
///
/// This function replaces the set of tags in the database
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
async fn update_task_tags_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    update_task_tags_inner_unchecked(conn, task_id, user_id, tag_ids).await
}

/// Unchecked internal function for updating task tags
///
/// Caller must guarantee task existence and ownership to requesting user
pub(super) async fn update_task_tags_inner_unchecked(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_ids: Vec<Uuid>,
) -> Result<Vec<Tag>> {
    // check if new set is the same; if it is, stop here
    let current_tag_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT tag_id FROM app.task_tags WHERE task_id = $1")
            .bind(task_id)
            .fetch_all(conn.as_mut())
            .await?;

    let current: HashSet<Uuid> = current_tag_ids.into_iter().collect();
    let desired: HashSet<Uuid> = tag_ids.iter().copied().collect();

    // change matches existing
    if current == desired {
        return query_task_tags_inner(conn, task_id, user_id).await;
    }

    // get tags to remove
    let to_remove: Vec<Uuid> = current.difference(&desired).copied().collect();
    if !to_remove.is_empty() {
        sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = ANY($2)")
            .bind(task_id)
            .bind(to_remove)
            .execute(conn.as_mut())
            .await?;
    }

    // get tags to add
    let to_add: Vec<Uuid> = desired.difference(&current).copied().collect();
    if !to_add.is_empty()
        && let Err(err) = sqlx::query(
            "INSERT INTO app.task_tags (task_id, tag_id)
            SELECT $1, unnest_tag
            FROM UNNEST($2::uuid[]) AS unnest_tag",
        )
        .bind(task_id)
        .bind(to_add)
        .execute(conn.as_mut())
        .await
    {
        if let Some(db_err) = err.as_database_error()
            && let Some("task_tags_tag_id_fkey") = db_err.constraint()
        {
            return Err(Error::Application(ApplicationError::TagNotFound));
        }

        return Err(err.into());
    }

    // set updated at if tags were updated
    sqlx::query(
        "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
    )
    .bind(task_id)
    .bind(user_id)
    .execute(conn.as_mut())
    .await?;

    query_task_tags_inner(conn, task_id, user_id).await
}

/// Remove a tag from a task
///
/// # Arguments
///
/// * `pool`: Database connection pool
/// * `task_id`: ID of task to delete tag from
/// * `user_id`: User ID of task owner
/// * `tag_id`: ID of tag to remove from task
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
async fn delete_task_tag_inner(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    if select_task_inner(conn, task_id, user_id).await?.is_none() {
        return Err(Error::Application(ApplicationError::TaskNotFound));
    }

    delete_task_tag_inner_unchecked(conn, task_id, user_id, tag_id).await
}

/// Unchecked internal function for deleting task tags
///
/// Caller must guarantee task existence and ownership to requesting user
pub(super) async fn delete_task_tag_inner_unchecked(
    conn: &mut PgConnection,
    task_id: Uuid,
    user_id: Uuid,
    tag_id: Uuid,
) -> Result<()> {
    let deleted = sqlx::query("DELETE FROM app.task_tags WHERE task_id = $1 AND tag_id = $2")
        .bind(task_id)
        .bind(tag_id)
        .execute(conn.as_mut())
        .await?
        .rows_affected();

    if deleted > 0 {
        sqlx::query(
            "UPDATE app.tasks SET
            updated_at = CURRENT_TIMESTAMP
            WHERE id = $1 AND created_by = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .execute(conn.as_mut())
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod query {
    use std::time::Duration;

    use tokio::test;
    use uuid::Uuid;

    use super::query_task_tags_inner;
    use crate::{
        db::{
            ApplicationError, Error,
            test_utils::{check_sort_task_tag, create_test_tag, create_test_task, db_init},
            utils::order_task_tag,
        },
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn default_query() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(3),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(2),
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate::default(),
                base_time,
                base_time + Duration::from_hours(1),
            )
            .await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        assert!(task_tags.is_sorted_by(check_sort_task_tag));
    }

    #[test]
    async fn no_tags() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), 0);
    }

    #[test]
    async fn one_tag() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), 1);
        assert!(task_tags.is_sorted_by(check_sort_task_tag));

        assert_eq!(task_tags[0], tag);
    }

    #[test]
    async fn many_tags() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        assert!(task_tags.is_sorted_by(check_sort_task_tag));

        let mut tag_list = tag_list.clone();
        tag_list.sort_by(order_task_tag);
        assert_eq!(task_tags, tag_list);
    }

    #[test]
    async fn deleted_task() {
        let (_, mut tx, base_time) = db_init().await;

        // No Tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // One Tag
        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // Many Tags
        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let (_, mut tx, _) = db_init().await;

        let res = query_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        // No Tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // One Tag
        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }

        // Many Tags
        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = query_task_tags_inner(&mut tx, task.id, Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}

#[cfg(test)]
mod insert {
    use std::collections::HashSet;

    use tokio::test;
    use uuid::Uuid;

    use super::insert_task_tags_inner;
    use crate::{
        db::{
            ApplicationError, Error,
            test_utils::{create_test_tag, create_test_task, db_init, get_task},
        },
        models::Tag,
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn base_append() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());
        if let Ok(tags) = res {
            assert!(tags.iter().all(|tag| matches!(tag, Tag { .. })));
        }
    }

    #[test]
    async fn is_idempotent() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_list = [
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 1".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 2".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
            create_test_tag(
                &mut tx,
                TagCreate {
                    label: "Tag 3".to_string(),
                    ..Default::default()
                },
                base_time,
                base_time,
            )
            .await,
        ];

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags, tag_list);
    }

    #[test]
    async fn empty_list() {
        let (_, mut tx, base_time) = db_init().await;

        // No existing tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With one existing tag
        let tag_list = [create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        // With many existing tags
        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );
    }

    #[test]
    async fn one_tag() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut tag_list =
            vec![create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[0].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[1].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 2);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[2].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );
    }

    #[test]
    async fn many_tags() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[..3].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[3..6].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 6);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        tag_list.append(&mut vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ]);
        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list[6..9].iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 9);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        let res = insert_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 9);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );
    }

    #[test]
    async fn nonexistent_tag_one() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![Uuid::new_v4()]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_within() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_ids = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
        ];

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_all() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_ids = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn deleted_task() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let (_, mut tx, _) = db_init().await;

        let res = insert_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = insert_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}

#[cfg(test)]
mod update {
    use std::collections::HashSet;

    use tokio::test;
    use uuid::Uuid;

    use super::update_task_tags_inner;
    use crate::{
        db::{
            ApplicationError, Error,
            test_utils::{
                check_sort_task_tag, create_test_tag, create_test_task, db_init, get_task,
            },
            utils::order_task_tag,
        },
        models::Tag,
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn base_update() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        for tag in tags {
            assert!(matches!(tag, Tag { .. }))
        }
    }

    #[test]
    async fn is_idempotent() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        assert!(task_tags.is_sorted_by(check_sort_task_tag));

        let mut tag_list = tag_list.clone();
        tag_list.sort_by(order_task_tag);
        assert_eq!(task_tags, tag_list);

        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let task_tags = res.unwrap();
        assert_eq!(task_tags.len(), tag_list.len());
        assert!(task_tags.is_sorted_by(check_sort_task_tag));

        let mut tag_list = tag_list.clone();
        tag_list.sort_by(order_task_tag);
        assert_eq!(task_tags, tag_list);
    }

    #[test]
    async fn empty_list() {
        let (_, mut tx, base_time) = db_init().await;

        // No existing tags
        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With one existing tag
        let tag_list = [create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());

        // With many existing tags
        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_ok());

        let tags = res.unwrap();
        assert!(tags.is_empty());

        let ret_task = get_task(&mut tx, task.id).await;
        assert!(ret_task.tags.is_empty());
    }

    #[test]
    async fn one_tag() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let mut tag_list =
            vec![create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await];
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[0].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[..1]);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[1].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[1..2]);

        tag_list.push(create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await);
        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![tag_list[2].id]).await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 1);
        assert_eq!(ret_task.tags, tag_list[2..3]);
    }

    #[test]
    async fn many_tags() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];
        let res = update_task_tags_inner(
            &mut tx,
            task.id,
            Uuid::nil(),
            tag_list.iter().map(|tag| tag.id).collect(),
        )
        .await;
        assert!(res.is_ok());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
        assert_eq!(
            ret_task
                .tags
                .iter()
                .map(|tag| tag.id)
                .collect::<HashSet<Uuid>>(),
            tag_list.iter().map(|tag| tag.id).collect::<HashSet<Uuid>>()
        );
    }

    #[test]
    async fn nonexistent_tag_one() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![Uuid::new_v4()]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_within() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_ids = vec![
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time)
                .await
                .id,
            Uuid::new_v4(),
        ];

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_tag_all() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let tag_ids = vec![
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ];

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), tag_ids).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TagNotFound)
            ))
        }
    }

    #[test]
    async fn deleted_task() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let (_, mut tx, _) = db_init().await;

        let res = update_task_tags_inner(&mut tx, Uuid::new_v4(), Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = update_task_tags_inner(&mut tx, task.id, Uuid::nil(), vec![]).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}

#[cfg(test)]
mod delete {
    use tokio::test;
    use uuid::Uuid;

    use super::delete_task_tag_inner;
    use crate::{
        db::{
            ApplicationError, Error,
            test_utils::{create_test_tag, create_test_task, db_init, get_task},
        },
        routes::models::{tag::Model as TagCreate, task::Model as TaskCreate},
    };

    #[test]
    async fn base_delete() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), tag_list.len());

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]))
    }

    #[test]
    async fn is_idempotent() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: tag_list.iter().map(|tag| tag.id).collect(),
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), tag_list.len());

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]));

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[2].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 4);
        assert!(!ret_task.tags.contains(&tag_list[2]));
    }

    #[test]
    async fn tag_not_on_task() {
        let (_, mut tx, base_time) = db_init().await;

        let tag_list = [
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
            create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await,
        ];

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag_list[0].id, tag_list[2].id, tag_list[4].id],
                ..Default::default()
            },
            None,
            None,
            base_time,
            base_time,
        )
        .await;
        assert_eq!(task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[1].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag_list[3].id).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 3);
    }

    #[test]
    async fn nonexistent_tag() {
        let (_, mut tx, base_time) = db_init().await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            None,
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), ());

        let ret_task = get_task(&mut tx, task.id).await;
        assert_eq!(ret_task.tags.len(), 0);
    }

    #[test]
    async fn deleted_task() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate::default(),
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::nil(), tag.id).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn nonexistent_task() {
        let (_, mut tx, _) = db_init().await;

        let res = delete_task_tag_inner(&mut tx, Uuid::nil(), Uuid::nil(), Uuid::new_v4()).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }

    #[test]
    async fn as_nonexistent_user() {
        let (_, mut tx, base_time) = db_init().await;

        let tag = create_test_tag(&mut tx, TagCreate::default(), base_time, base_time).await;

        let task = create_test_task(
            &mut tx,
            TaskCreate {
                tags: vec![tag.id],
                ..Default::default()
            },
            None,
            Some(base_time),
            base_time,
            base_time,
        )
        .await;

        let res = delete_task_tag_inner(&mut tx, task.id, Uuid::new_v4(), tag.id).await;
        assert!(res.is_err());
        if let Err(err) = res {
            assert!(matches!(
                err,
                Error::Application(ApplicationError::TaskNotFound)
            ))
        }
    }
}
